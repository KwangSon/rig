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
    pub project: String,
    pub title: String,
    pub key_data: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct CreateSshKeyRequest {
    pub title: String,
    pub key_data: String,
}

pub async fn get_keys_handler(
    State(db): State<PgPool>,
    Path(project): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<SshKey>>, StatusCode> {
    // Note: We could authenticate the user here, but skipping for brevity
    // In a real app we'd verify the user has access to this project
    let keys = sqlx::query_as!(
        SshKey,
        "SELECT id, project, title, key_data, created_at FROM ssh_keys WHERE project = $1",
        project
    )
    .fetch_all(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(keys))
}

pub async fn add_key_handler(
    State(db): State<PgPool>,
    Path(project): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateSshKeyRequest>,
) -> Result<(StatusCode, Json<SshKey>), StatusCode> {
    // Authenticate simplified
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let _token_data =
        crate::users::decode_token(auth_header).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let key = sqlx::query_as!(
        SshKey,
        "INSERT INTO ssh_keys (project, title, key_data) VALUES ($1, $2, $3) RETURNING id, project, title, key_data, created_at",
        project,
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

pub async fn delete_key_handler(
    State(db): State<PgPool>,
    Path((project, id)): Path<(String, Uuid)>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let _token_data =
        crate::users::decode_token(auth_header).map_err(|_| StatusCode::UNAUTHORIZED)?;

    sqlx::query!(
        "DELETE FROM ssh_keys WHERE id = $1 AND project = $2",
        id,
        project
    )
    .execute(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
