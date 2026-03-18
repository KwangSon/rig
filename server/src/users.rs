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
pub fn decode_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET.as_ref()),
        &Validation::default(),
    )
    .map(|data| data.claims)
}
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    exp: u64,
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

    // Generate JWT
    let claims = Claims {
        sub: user.id.to_string(),
        exp: (std::time::SystemTime::now() + std::time::Duration::from_secs(24 * 60 * 60))
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
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
            .as_secs(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse { token, user }))
}

pub async fn me_handler(
    headers: HeaderMap,
    State(db): State<PgPool>,
) -> Result<Json<User>, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token_data = decode::<Claims>(
        auth_header,
        &DecodingKey::from_secret(JWT_SECRET.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id: Uuid = Uuid::parse_str(&token_data.claims.sub).unwrap();

    let user =
        sqlx::query_as::<_, User>("SELECT id, name, email, password_hash FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok(Json(user))
}
