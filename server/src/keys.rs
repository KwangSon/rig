use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct SshKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub key_data: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct CreateSshKeyRequest {
    pub title: String,
    pub key_data: String,
}

pub async fn get_user_keys_handler(
    State(db): State<PgPool>,
    headers: HeaderMap,
) -> Result<Json<Vec<SshKey>>, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token_data =
        crate::users::decode_token(auth_header).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user_id = Uuid::parse_str(&token_data.sub).map_err(|_| StatusCode::BAD_REQUEST)?;

    let keys = sqlx::query_as!(
        SshKey,
        "SELECT id, user_id, title, key_data, created_at FROM ssh_keys WHERE user_id = $1",
        user_id
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(keys))
}

pub async fn add_user_key_handler(
    State(db): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateSshKeyRequest>,
) -> Result<(StatusCode, Json<SshKey>), StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token_data =
        crate::users::decode_token(auth_header).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user_id = Uuid::parse_str(&token_data.sub).map_err(|_| StatusCode::BAD_REQUEST)?;

    let key = sqlx::query_as!(
        SshKey,
        "INSERT INTO ssh_keys (user_id, title, key_data) VALUES ($1, $2, $3) RETURNING id, user_id, title, key_data, created_at",
        user_id,
        payload.title,
        payload.key_data
    )
    .fetch_one(&db)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok((StatusCode::CREATED, Json(key)))
}

pub async fn delete_user_key_handler(
    State(db): State<PgPool>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token_data =
        crate::users::decode_token(auth_header).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user_id = Uuid::parse_str(&token_data.sub).map_err(|_| StatusCode::BAD_REQUEST)?;

    let result = sqlx::query!(
        "DELETE FROM ssh_keys WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

// Legacy project-specific handlers (redirected to user keys or kept for compatibility if possible)
// Since the DB schema changed, we'll just remove them or make them return empty/error.
// The user wants to improve the flow to be "after login, register ssh".

pub async fn get_keys_handler(
    State(db): State<PgPool>,
    Path(_project): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<SshKey>>, StatusCode> {
    // For now, let's just return an empty list or error to signal it's moved
    Ok(Json(vec![]))
}

pub async fn add_key_handler(
    State(_db): State<PgPool>,
    Path(_project): Path<String>,
    _headers: HeaderMap,
    Json(_payload): Json<CreateSshKeyRequest>,
) -> Result<(StatusCode, Json<SshKey>), StatusCode> {
    Err(StatusCode::GONE)
}

pub async fn delete_key_handler(
    State(_db): State<PgPool>,
    Path((_project, _id)): Path<(String, Uuid)>,
    _headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    Err(StatusCode::GONE)
}
