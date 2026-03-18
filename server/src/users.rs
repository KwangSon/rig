use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use protocol::{Permission, User};

#[derive(Serialize, Deserialize, Clone)]
pub struct UserState {
    pub users: Vec<User>,
    pub permissions: Vec<Permission>,
}

pub type SharedUserState = Arc<Mutex<UserState>>;

const JWT_SECRET: &str = "your-secret-key"; // TODO: use env var

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub fn decode_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_ref()),
        &Validation::default(),
    )
    .map(|data| data.claims)
}

pub async fn authenticate_token(db: &PgPool, token: &str) -> Result<User, StatusCode> {
    // 1. Try JWT
    let user_id = if let Ok(claims) = decode_token(token) {
        Uuid::parse_str(&claims.sub).ok()
    } else {
        None
    };

    let final_user_id = if let Some(id) = user_id {
        id
    } else {
        // 2. Try DB tokens (PAT)
        let row = sqlx::query("SELECT user_id FROM tokens WHERE token_text = $1")
            .bind(token)
            .fetch_optional(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let id: Uuid = row.get("user_id");
        // Update last used
        let _ =
            sqlx::query("UPDATE tokens SET last_used_at = CURRENT_TIMESTAMP WHERE token_text = $1")
                .bind(token)
                .execute(db)
                .await;
        id
    };

    // Fetch user details
    let user =
        sqlx::query_as::<_, User>("SELECT id, name, email, password_hash FROM users WHERE id = $1")
            .bind(final_user_id)
            .fetch_optional(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok(user)
}

pub async fn load_user_state(db: &PgPool) -> Result<UserState, sqlx::Error> {
    let users = sqlx::query_as::<_, User>("SELECT id, name, email, password_hash FROM users")
        .fetch_all(db)
        .await?;

    let permissions =
        sqlx::query_as::<_, Permission>("SELECT user_id, project_id, access FROM permissions")
            .fetch_all(db)
            .await?;

    Ok(UserState { users, permissions })
}

// Handlers

pub async fn get_users_handler(State(db): State<PgPool>) -> Result<Json<Vec<User>>, StatusCode> {
    let users = sqlx::query_as::<_, User>("SELECT id, name, email, password_hash FROM users")
        .fetch_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(users))
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

pub async fn create_user_handler(
    State(db): State<PgPool>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    // Hash the password
    let password_hash = bcrypt::hash(payload.password, bcrypt::DEFAULT_COST)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email, password_hash",
    )
    .bind(&payload.name)
    .bind(&payload.email)
    .bind(&password_hash)
    .fetch_one(&db)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn delete_user_handler(
    Path(id): Path<Uuid>,
    State(db): State<PgPool>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_permissions_handler(
    State(db): State<PgPool>,
) -> Result<Json<Vec<Permission>>, StatusCode> {
    let permissions =
        sqlx::query_as::<_, Permission>("SELECT user_id, project_id, access FROM permissions")
            .fetch_all(&db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(permissions))
}

#[derive(Deserialize)]
pub struct SetPermissionRequest {
    pub user_id: Uuid,
    pub project_name: String,
    pub access: String,
}

pub async fn set_permission_handler(
    State(db): State<PgPool>,
    Json(payload): Json<SetPermissionRequest>,
) -> Result<(StatusCode, Json<Permission>), StatusCode> {
    // Check if project exists and get its ID
    let project_row = sqlx::query("SELECT id FROM projects WHERE name = $1")
        .bind(&payload.project_name)
        .fetch_optional(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let project_id: Uuid = project_row.get("id");

    // Upsert permission
    let permission = sqlx::query_as::<_, Permission>(
        "INSERT INTO permissions (user_id, project_id, access) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, project_id) DO UPDATE SET access = EXCLUDED.access
         RETURNING user_id, project_id, access",
    )
    .bind(payload.user_id)
    .bind(project_id)
    .bind(payload.access)
    .fetch_one(&db)
    .await
    .map_err(|e| {
        eprintln!("Error setting permission: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::OK, Json(permission)))
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

pub async fn register_handler(
    State(db): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    // Hash the password
    let password_hash = bcrypt::hash(payload.password, bcrypt::DEFAULT_COST)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let final_name = if payload.name.trim().is_empty() {
        payload
            .email
            .split('@')
            .next()
            .unwrap_or("user")
            .to_string()
    } else {
        payload.name.clone()
    };

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email, password_hash",
    )
    .bind(&final_name)
    .bind(&payload.email)
    .bind(&password_hash)
    .fetch_one(&db)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Generate JWT
    let claims = Claims {
        sub: user.id.to_string(),
        exp: (std::time::SystemTime::now() + std::time::Duration::from_secs(24 * 60 * 60))
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse { token, user }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub cli_session: Option<Uuid>,
}

pub async fn login_handler(
    State(db): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, name, email, password_hash FROM users WHERE email = $1",
    )
    .bind(&payload.email)
    .fetch_optional(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify password
    let valid = bcrypt::verify(payload.password, &user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Generate JWT
    let claims = Claims {
        sub: user.id.to_string(),
        exp: (std::time::SystemTime::now() + std::time::Duration::from_secs(24 * 60 * 60))
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // If cli_session is provided, approve it automatically
    if let Some(session_id) = payload.cli_session {
        crate::auth::approve_session(&db, session_id, user.id)
            .await
            .map_err(|e| {
                eprintln!("Failed to approve CLI session: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    Ok(Json(AuthResponse { token, user }))
}

#[derive(Serialize, Deserialize)]
pub struct TokenInfo {
    pub id: Uuid,
    pub token_text: String,
    pub name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
}

pub async fn list_tokens_handler(
    headers: HeaderMap,
    State(db): State<PgPool>,
) -> Result<Json<Vec<TokenInfo>>, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = authenticate_token(&db, auth_header).await?;

    let tokens = sqlx::query_as!(
        TokenInfo,
        "SELECT id, token_text, name, created_at, last_used_at FROM tokens WHERE user_id = $1 ORDER BY created_at DESC",
        user.id
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tokens))
}

pub async fn create_token_handler(
    headers: HeaderMap,
    State(db): State<PgPool>,
    Json(payload): Json<CreateTokenRequest>,
) -> Result<(StatusCode, Json<TokenInfo>), StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = authenticate_token(&db, auth_header).await?;

    // Generate a random token
    let token_text = format!("rigp_{}", Uuid::new_v4().to_string().replace("-", ""));

    let token = sqlx::query_as!(
        TokenInfo,
        "INSERT INTO tokens (user_id, token_text, name) VALUES ($1, $2, $3) RETURNING id, token_text, name, created_at, last_used_at",
        user.id,
        token_text,
        payload.name
    )
    .fetch_one(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(token)))
}

pub async fn delete_token_handler(
    headers: HeaderMap,
    Path(token_id): Path<Uuid>,
    State(db): State<PgPool>,
) -> Result<StatusCode, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = authenticate_token(&db, auth_header).await?;

    let result = sqlx::query!(
        "DELETE FROM tokens WHERE id = $1 AND user_id = $2",
        token_id,
        user.id
    )
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
