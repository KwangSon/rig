use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use protocol::User;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Serialize)]
pub struct CreateSessionResponse {
    pub session_id: Uuid,
}

#[derive(Serialize)]
pub struct SessionStatusResponse {
    pub status: String,
    pub token: Option<String>,
}

#[derive(Deserialize)]
pub struct PollQuery {
    pub session_id: Uuid,
}

pub async fn create_session_handler(
    State(db): State<PgPool>,
) -> Result<Json<CreateSessionResponse>, StatusCode> {
    let row = sqlx::query("INSERT INTO cli_sessions (status) VALUES ('pending') RETURNING id")
        .fetch_one(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_id: Uuid = row.get("id");
    Ok(Json(CreateSessionResponse { session_id }))
}

pub async fn poll_session_handler(
    State(db): State<PgPool>,
    Query(query): Query<PollQuery>,
) -> Result<Json<SessionStatusResponse>, StatusCode> {
    let row = sqlx::query(
        "SELECT s.status, t.token_text 
         FROM cli_sessions s 
         LEFT JOIN tokens t ON s.token_id = t.id 
         WHERE s.id = $1",
    )
    .bind(query.session_id)
    .fetch_optional(&db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let status: String = row.get("status");
    let token: Option<String> = row.try_get("token_text").ok();

    Ok(Json(SessionStatusResponse { status, token }))
}

// Internal helper for Web UI to approve a session
pub async fn approve_session(
    db: &PgPool,
    session_id: Uuid,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    // 1. Create a new token for the user
    let token_text = format!("rigp_{}", Uuid::new_v4().to_string().replace("-", ""));

    let token_row = sqlx::query(
        "INSERT INTO tokens (user_id, token_text, name) VALUES ($1, $2, 'cli-session') RETURNING id"
    )
    .bind(user_id)
    .bind(&token_text)
    .fetch_one(db)
    .await?;

    let token_id: Uuid = token_row.get("id");

    // 2. Update session status
    sqlx::query("UPDATE cli_sessions SET status = 'success', token_id = $1 WHERE id = $2")
        .bind(token_id)
        .bind(session_id)
        .execute(db)
        .await?;

    Ok(())
}
