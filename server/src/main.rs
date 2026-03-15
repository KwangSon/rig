use axum::{
    Router,
    extract::{FromRef, State},
    http::{Method, StatusCode},
    response::Json,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use protocol::Artifact;

// --- App State ---

#[derive(Clone)]
pub struct AppState {
    pub project_dir: PathBuf,
    pub artifacts: HashMap<String, Artifact>,
}

// --- Request Payloads ---

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
}

#[derive(Serialize)]
struct CreateProjectResponse {
    message: String,
}

#[derive(Deserialize)]
pub struct LockRequest {
    pub user: String,
}

#[derive(Deserialize)]
pub struct UnlockRequest {
    pub user: String,
    pub force: bool,
}

// --- Response Payloads ---

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
pub struct LockResponse {
    locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
}

// --- Application State ---

pub type SharedState = Arc<Mutex<HashMap<String, AppState>>>;

#[derive(Clone)]
pub struct CombinedState {
    pub projects: SharedState,
    pub users: users::SharedUserState,
}

impl FromRef<CombinedState> for SharedState {
    fn from_ref(state: &CombinedState) -> Self {
        state.projects.clone()
    }
}

impl FromRef<CombinedState> for users::SharedUserState {
    fn from_ref(state: &CombinedState) -> Self {
        state.users.clone()
    }
}

// --- Main ---

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let base_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("server/examples")
    };

    // Collect all projects under the base directory (any subdirectory with index.json)
    let mut projects: HashMap<String, AppState> = HashMap::new();
    if let Ok(entries) = fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata()
                && metadata.is_dir()
            {
                let candidate_dir = entry.path();
                let index_path = candidate_dir.join("index.json");
                if index_path.exists()
                    && let Some(project_name) = candidate_dir.file_name().and_then(|s| s.to_str())
                    && let Ok(app_state) = load_project_state(&candidate_dir)
                {
                    projects.insert(project_name.to_string(), app_state);
                }
            }
        }
    }

    let state: SharedState = Arc::new(Mutex::new(projects));
    let user_state: users::SharedUserState = Arc::new(Mutex::new(users::load_user_state()));

    let combined_state = CombinedState {
        projects: state,
        users: user_state,
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/projects", get(get_projects_handler))
        .route("/create_project", post(create_project_handler))
        .route(
            "/users",
            get(users::get_users_handler).post(users::create_user_handler),
        )
        .route("/users/{id}", delete(users::delete_user_handler))
        .route(
            "/permissions",
            get(users::get_permissions_handler).post(users::set_permission_handler),
        )
        .route("/{project}/index.json", get(artifacts::get_index_handler))
        .route(
            "/{project}/artifacts",
            post(artifacts::create_artifact_handler).get(artifacts::get_artifacts_handler),
        )
        .route(
            "/{project}/artifacts/{id}",
            get(artifacts::get_artifact_info_handler),
        )
        .route(
            "/{project}/artifacts/{id}/revisions",
            post(artifacts::create_revision_handler).get(artifacts::get_revisions_handler),
        )
        .route(
            "/{project}/artifacts/{id}/lock",
            post(artifacts::lock_handler)
                .delete(artifacts::unlock_handler)
                .get(artifacts::get_lock_handler),
        )
        .route("/{project}/push", post(artifacts::push_handler))
        .route(
            "/{project}/artifacts/{id}/{filename}",
            get(artifacts::download_artifact_handler),
        )
        .with_state(combined_state)
        .layer(cors)
        .fallback_service(ServeDir::new(&base_dir));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("Server failed");
}

fn load_project_state(project_dir: &Path) -> Result<AppState, String> {
    let index_path = project_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
    let index: serde_json::Value =
        serde_json::from_str(&index_content).map_err(|e| e.to_string())?;

    let mut artifacts = HashMap::new();
    if let Some(artifacts_obj) = index["artifacts"].as_object() {
        for (k, v) in artifacts_obj {
            let path = v["path"].as_str().unwrap_or("");
            let latest = v["latest"].as_u64().unwrap_or(0) as u32;
            let locked_by = v["locked_by"].as_str().map(|s| s.to_string());
            let revisions: Vec<protocol::Revision> = v["revisions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|r| protocol::Revision {
                    rev: r["rev"].as_u64().unwrap_or(0) as u32,
                    hash: r["hash"].as_str().unwrap_or("").to_string(),
                })
                .collect();
            artifacts.insert(
                k.clone(),
                protocol::Artifact {
                    path: path.to_string(),
                    latest,
                    locked_by,
                    revisions,
                },
            );
        }
    }

    Ok(AppState {
        project_dir: project_dir.to_path_buf(),
        artifacts,
    })
}

// --- Handlers ---

// GET /health
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

// GET /projects
async fn get_projects_handler(State(state): State<SharedState>) -> Json<Vec<String>> {
    let projects = state.lock().await;
    Json(projects.keys().cloned().collect())
}

// POST /create_project
async fn create_project_handler(
    State(state): State<SharedState>,
    Json(payload): Json<CreateProjectRequest>,
) -> (StatusCode, Json<CreateProjectResponse>) {
    let mut projects = state.lock().await;
    if projects.contains_key(&payload.name) {
        return (
            StatusCode::CONFLICT,
            Json(CreateProjectResponse {
                message: "Project already exists".to_string(),
            }),
        );
    }

    // Create project directory
    let project_dir = PathBuf::from("server/examples").join(&payload.name);
    if fs::create_dir_all(&project_dir).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateProjectResponse {
                message: "Failed to create project directory".to_string(),
            }),
        );
    }

    // Create initial index.json with all required fields
    let index_path = project_dir.join("index.json");
    let initial_index = serde_json::json!({
        "project": payload.name,
        "server_url": "http://localhost:3000",
        "artifacts": {},
        "commits": {},
        "latest_commit": ""
    });
    if fs::write(
        &index_path,
        serde_json::to_string_pretty(&initial_index).unwrap(),
    )
    .is_err()
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateProjectResponse {
                message: "Failed to create index.json".to_string(),
            }),
        );
    }

    // Load the new project state
    match load_project_state(&project_dir) {
        Ok(app_state) => {
            projects.insert(payload.name, app_state);
            (
                StatusCode::CREATED,
                Json(CreateProjectResponse {
                    message: "Project created successfully".to_string(),
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateProjectResponse {
                message: format!("Failed to load project: {}", e),
            }),
        ),
    }
}

// --- Artifacts Module ---

mod artifacts;
mod users;
