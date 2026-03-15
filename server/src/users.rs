use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use protocol::{Permission, User};

#[derive(Serialize, Deserialize, Clone)]
pub struct UserState {
    pub users: Vec<User>,
    pub permissions: Vec<Permission>,
}

pub type SharedUserState = Arc<Mutex<UserState>>;

pub fn get_users_file_path() -> PathBuf {
    std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("server/examples"))
        .join("users.json")
}

pub fn load_user_state() -> UserState {
    let path = get_users_file_path();
    if path.exists()
        && let Ok(content) = fs::read_to_string(&path)
        && let Ok(state) = serde_json::from_str(&content)
    {
        return state;
    }
    UserState {
        users: vec![],
        permissions: vec![],
    }
}

pub fn save_user_state(state: &UserState) -> Result<(), String> {
    let path = get_users_file_path();
    let content = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

// Handlers

pub async fn get_users_handler(State(state): State<SharedUserState>) -> Json<Vec<User>> {
    let s = state.lock().await;
    Json(s.users.clone())
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub role: String,
}

pub async fn create_user_handler(
    State(state): State<SharedUserState>,
    Json(payload): Json<CreateUserRequest>,
) -> (StatusCode, Json<User>) {
    let mut s = state.lock().await;
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        name: payload.name,
        email: payload.email,
        role: payload.role,
    };
    s.users.push(user.clone());
    if let Err(e) = save_user_state(&s) {
        eprintln!("Failed to save users: {}", e);
    }
    (StatusCode::CREATED, Json(user))
}

pub async fn delete_user_handler(
    Path(id): Path<String>,
    State(state): State<SharedUserState>,
) -> StatusCode {
    let mut s = state.lock().await;
    s.users.retain(|u| u.id != id);
    s.permissions.retain(|p| p.user_id != id);
    if let Err(e) = save_user_state(&s) {
        eprintln!("Failed to save users: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::NO_CONTENT
}

pub async fn get_permissions_handler(
    State(state): State<SharedUserState>,
) -> Json<Vec<Permission>> {
    let s = state.lock().await;
    Json(s.permissions.clone())
}

#[derive(Deserialize)]
pub struct SetPermissionRequest {
    pub user_id: String,
    pub project: String,
    pub access: String,
}

pub async fn set_permission_handler(
    State(state): State<SharedUserState>,
    Json(payload): Json<SetPermissionRequest>,
) -> (StatusCode, Json<Permission>) {
    let mut s = state.lock().await;

    // Remove existing permission for this user and project if any
    s.permissions
        .retain(|p| !(p.user_id == payload.user_id && p.project == payload.project));

    let permission = Permission {
        user_id: payload.user_id,
        project: payload.project,
        access: payload.access,
    };
    s.permissions.push(permission.clone());

    if let Err(e) = save_user_state(&s) {
        eprintln!("Failed to save permissions: {}", e);
    }
    (StatusCode::OK, Json(permission))
}
