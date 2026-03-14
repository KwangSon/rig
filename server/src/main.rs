use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs;
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
    project_dir: PathBuf,
}

type SharedState = Arc<Mutex<AppState>>;

// --- Main ---

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let base_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("server/examples")
    };

    // Determine actual project dir (where index.json lives)
    let project_dir = find_project_dir(&base_dir).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    // Load index.json and initialize artifacts
    let index_path = project_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path).expect("Failed to read index.json");
    let index: serde_json::Value =
        serde_json::from_str(&index_content).expect("Failed to parse index.json");

    let artifacts: Vec<Artifact> = index["artifacts"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| {
            let path = v["path"].as_str().unwrap();
            let latest = v["latest"].as_u64().unwrap() as u32;
            let locked_by = v["locked_by"].as_str();
            let revisions: Vec<Revision> = v["revisions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| Revision {
                    revision: r["rev"].as_u64().unwrap() as u32,
                    message: "".to_string(), // No message in index.json
                })
                .collect();
            Artifact {
                id: k.clone(),
                path: path.to_string(),
                latest_revision: latest,
                revisions,
                lock: locked_by.map(|u| Lock {
                    user: u.to_string(),
                }),
            }
        })
        .collect();

    let state = SharedState::new(Mutex::new(AppState {
        users: vec![],
        artifacts,
        project_dir: project_dir.clone(),
    }));

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
        .fallback_service(ServeDir::new(base_dir));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("Server failed");
}

fn find_project_dir(base: &PathBuf) -> Result<PathBuf, String> {
    let index = base.join("index.json");
    if index.exists() {
        return Ok(base.clone());
    }

    let mut candidates = Vec::new();
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    let candidate = entry.path();
                    if candidate.join("index.json").exists() {
                        candidates.push(candidate);
                    }
                }
            }
        }
    }

    match candidates.as_slice() {
        [single] => Ok(single.clone()),
        [] => Err(format!(
            "No index.json found in '{}' or its immediate subdirectories",
            base.display()
        )),
        _ => Err(format!(
            "Multiple projects found in '{}', please specify one directly",
            base.display()
        )),
    }
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

mod artifacts;
