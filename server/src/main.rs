use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

// --- Data Structures ---

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
struct CreateProjectRequest {
    name: String,
}

#[derive(Serialize)]
struct CreateProjectResponse {
    message: String,
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
    artifacts: Vec<Artifact>,
    project_dir: PathBuf,
}

type SharedState = Arc<Mutex<HashMap<String, AppState>>>;

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
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    let candidate_dir = entry.path();
                    let index_path = candidate_dir.join("index.json");
                    if index_path.exists() {
                        if let Some(project_name) =
                            candidate_dir.file_name().and_then(|s| s.to_str())
                        {
                            if let Ok(app_state) =
                                load_project_state(project_name.to_string(), &candidate_dir)
                            {
                                projects.insert(project_name.to_string(), app_state);
                            }
                        }
                    }
                }
            }
        }
    }

    let state: SharedState = Arc::new(Mutex::new(projects));

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/create_project", post(create_project_handler))
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
        .with_state(state)
        .fallback_service(ServeDir::new(base_dir));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("Server failed");
}

fn load_project_state(project_name: String, project_dir: &PathBuf) -> Result<AppState, String> {
    let index_path = project_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
    let index: serde_json::Value =
        serde_json::from_str(&index_content).map_err(|e| e.to_string())?;

    let artifacts: Vec<Artifact> = index["artifacts"]
        .as_object()
        .ok_or("Invalid index.json: artifacts is not an object".to_string())?
        .iter()
        .map(|(k, v)| {
            let path = v["path"].as_str().unwrap_or("");
            let latest = v["latest"].as_u64().unwrap_or(0) as u32;
            let locked_by = v["locked_by"].as_str();
            let revisions: Vec<Revision> = v["revisions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|r| Revision {
                    revision: r["rev"].as_u64().unwrap_or(0) as u32,
                    message: "".to_string(),
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

    Ok(AppState {
        artifacts,
        project_dir: project_dir.clone(),
    })
}

// --- Handlers ---

// GET /health
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
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
    if let Err(_) = fs::create_dir_all(&project_dir) {
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
        "commits": [],
        "latest_commit": 0
    });
    if let Err(_) = fs::write(
        &index_path,
        serde_json::to_string_pretty(&initial_index).unwrap(),
    ) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateProjectResponse {
                message: "Failed to create index.json".to_string(),
            }),
        );
    }

    // Load the new project state
    match load_project_state(payload.name.clone(), &project_dir) {
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
