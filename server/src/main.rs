use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

// --- Data Structures ---

#[derive(Serialize, Deserialize, Clone, Debug)]
struct User {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Artifact {
    id: String,
    path: String,
    latest_revision: u32,
    revisions: Vec<Revision>,
    lock: Option<Lock>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Revision {
    revision: u32,
    message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Lock {
    user: String,
}

// --- Request Payloads ---

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
}

#[derive(Deserialize)]
struct CreateArtifactRequest {
    path: String,
}

#[derive(Deserialize)]
struct CreateRevisionRequest {
    message: String,
}

#[derive(Deserialize)]
struct LockRequest {
    user: String,
}

// --- Response Payloads ---

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct CreateUserResponse {
    id: String,
}

#[derive(Serialize)]
struct CreateArtifactResponse {
    artifact_id: String,
}

#[derive(Serialize)]
struct ArtifactShortResponse {
    id: String,
    path: String,
}

#[derive(Serialize)]
struct ArtifactInfoResponse {
    id: String,
    path: String,
    latest_revision: u32,
}

#[derive(Serialize)]
struct RevisionShortResponse {
    revision: u32,
}

#[derive(Serialize)]
struct CreateRevisionResponse {
    revision: u32,
}

#[derive(Serialize)]
struct LockResponse {
    locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
}

// --- Application State ---

#[derive(Default)]
struct AppState {
    users: Vec<User>,
    artifacts: Vec<Artifact>,
}

type SharedState = Arc<Mutex<AppState>>;

// --- Main ---

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let project_root = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("examples")
    };

    let state = SharedState::new(Mutex::new(AppState::default()));

    let app = Router::new()
        .route("/health", get(health_handler))
        .route(
            "/users",
            post(users::create_user_handler).get(users::get_users_handler),
        )
        .route(
            "/artifacts",
            post(artifacts::create_artifact_handler).get(artifacts::get_artifacts_handler),
        )
        .route("/artifacts/{id}", get(artifacts::get_artifact_info_handler))
        .route(
            "/artifacts/{id}/revisions",
            post(artifacts::create_revision_handler).get(artifacts::get_revisions_handler),
        )
        .route(
            "/artifacts/{id}/lock",
            post(artifacts::lock_handler)
                .delete(artifacts::unlock_handler)
                .get(artifacts::get_lock_handler),
        )
        .with_state(state)
        .fallback_service(ServeDir::new(project_root));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

// --- Handlers ---

// GET /health
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

// --- Users Module ---

mod users {
    use super::*;
    use axum::{extract::State, http::StatusCode, response::Json};

    // POST /users
    pub async fn create_user_handler(
        State(_state): State<SharedState>,
        Json(_payload): Json<CreateUserRequest>,
    ) -> (StatusCode, Json<CreateUserResponse>) {
        // Placeholder implementation
        (
            StatusCode::CREATED,
            Json(CreateUserResponse {
                id: "u1".to_string(),
            }),
        )
    }

    // GET /users
    pub async fn get_users_handler(State(_state): State<SharedState>) -> Json<Vec<User>> {
        // Placeholder implementation
        Json(vec![User {
            id: "u1".to_string(),
            name: "Alice".to_string(),
        }])
    }
}

// --- Artifacts Module ---

mod artifacts {
    use super::*;
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::Json,
    };

    // POST /artifacts
    pub async fn create_artifact_handler(
        State(_state): State<SharedState>,
        Json(_payload): Json<CreateArtifactRequest>,
    ) -> (StatusCode, Json<CreateArtifactResponse>) {
        // Placeholder implementation
        (
            StatusCode::CREATED,
            Json(CreateArtifactResponse {
                artifact_id: "a1".to_string(),
            }),
        )
    }

    // GET /artifacts
    pub async fn get_artifacts_handler(
        State(_state): State<SharedState>,
    ) -> Json<Vec<ArtifactShortResponse>> {
        // Placeholder implementation
        Json(vec![ArtifactShortResponse {
            id: "a1".to_string(),
            path: "/path/to/artifact".to_string(),
        }])
    }

    // GET /artifacts/{id}
    pub async fn get_artifact_info_handler(
        State(_state): State<SharedState>,
        Path(_id): Path<String>,
    ) -> Result<Json<ArtifactInfoResponse>, StatusCode> {
        // Placeholder implementation
        Ok(Json(ArtifactInfoResponse {
            id: "a1".to_string(),
            path: "/path/to/artifact".to_string(),
            latest_revision: 1,
        }))
    }

    // POST /artifacts/{id}/revisions
    pub async fn create_revision_handler(
        State(_state): State<SharedState>,
        Path(_id): Path<String>,
        Json(_payload): Json<CreateRevisionRequest>,
    ) -> Result<(StatusCode, Json<CreateRevisionResponse>), StatusCode> {
        // Placeholder implementation
        Ok((
            StatusCode::CREATED,
            Json(CreateRevisionResponse { revision: 1 }),
        ))
    }

    // GET /artifacts/{id}/revisions
    pub async fn get_revisions_handler(
        State(_state): State<SharedState>,
        Path(_id): Path<String>,
    ) -> Result<Json<Vec<RevisionShortResponse>>, StatusCode> {
        // Placeholder implementation
        Ok(Json(vec![RevisionShortResponse { revision: 1 }]))
    }

    // POST /artifacts/{id}/lock
    pub async fn lock_handler(
        State(_state): State<SharedState>,
        Path(_id): Path<String>,
        Json(_payload): Json<LockRequest>,
    ) -> Result<Json<LockResponse>, StatusCode> {
        // Placeholder implementation
        Ok(Json(LockResponse {
            locked: true,
            user: Some("user1".to_string()),
        }))
    }

    // DELETE /artifacts/{id}/lock
    pub async fn unlock_handler(
        State(_state): State<SharedState>,
        Path(_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        // Placeholder implementation
        Ok(StatusCode::NO_CONTENT)
    }

    // GET /artifacts/{id}/lock
    pub async fn get_lock_handler(
        State(_state): State<SharedState>,
        Path(_id): Path<String>,
    ) -> Result<Json<LockResponse>, StatusCode> {
        // Placeholder implementation
        Ok(Json(LockResponse {
            locked: false,
            user: None,
        }))
    }
}
