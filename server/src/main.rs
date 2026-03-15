use axum::{
    Router,
    extract::{FromRef, Path, State},
    http::{HeaderMap, Method, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use axum::routing;
use sqlx::Row;
use sqlx::types::Uuid;

// --- App State ---

#[derive(Clone)]
pub struct AppState {
    pub project_dir: PathBuf,
    pub artifacts: HashMap<String, protocol::Artifact>,
    pub git_modules: HashMap<String, protocol::GitModule>,
}

// --- Request Payloads ---

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
}

#[derive(Serialize, Debug)]
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
    pub db: sqlx::PgPool,
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

impl FromRef<CombinedState> for sqlx::PgPool {
    fn from_ref(state: &CombinedState) -> Self {
        state.db.clone()
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

    // Connect to database
    let database_url = "postgresql://kwang@localhost/rig";
    let db = sqlx::PgPool::connect(database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    // run_migrations(&db).await.expect("Failed to run migrations");

    // Collect all projects from DB and load their state if directory exists
    let mut projects: HashMap<String, AppState> = HashMap::new();
    let project_names = sqlx::query("SELECT name FROM projects")
        .fetch_all(&db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.get::<String, _>("name"))
        .collect::<Vec<String>>();
    for name in project_names {
        let project_dir = base_dir.join(&name);
        if project_dir.exists() {
            if let Ok(app_state) = load_project_state(&project_dir) {
                projects.insert(name, app_state);
            }
        }
    }

    let state: SharedState = Arc::new(Mutex::new(projects));
    let user_state: users::SharedUserState = Arc::new(Mutex::new(
        users::load_user_state(&db)
            .await
            .expect("Failed to load user state"),
    ));

    let combined_state = CombinedState {
        projects: state,
        users: user_state,
        db,
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_origin(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/projects", get(get_projects_handler))
        .route("/projects/{name}", delete(delete_project_handler))
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
        .route("/register", post(users::register_handler))
        .route("/login", post(users::login_handler))
        .route("/me", get(users::me_handler))
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
            "/{project}/gitmodules/{*path}",
            routing::put(artifacts::update_gitmodule_handler),
        )
        .route(
            "/{project}/artifacts/{id}/{filename}",
            get(artifacts::download_artifact_handler),
        )
        .with_state(combined_state);

    let app = Router::new()
        .nest("/api/v1", api_routes)
        .layer(cors)
        .fallback_service(ServeDir::new(&base_dir));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("Server failed");
}

fn load_project_state(project_dir: &StdPath) -> Result<AppState, String> {
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

    let mut git_modules = HashMap::new();
    if let Some(modules_obj) = index["git_modules"].as_object() {
        for (k, v) in modules_obj {
            git_modules.insert(
                k.clone(),
                protocol::GitModule {
                    path: v["path"].as_str().unwrap_or("").to_string(),
                    url: v["url"].as_str().unwrap_or("").to_string(),
                    commit: v["commit"].as_str().unwrap_or("").to_string(),
                },
            );
        }
    }

    Ok(AppState {
        project_dir: project_dir.to_path_buf(),
        artifacts,
        git_modules,
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
async fn get_projects_handler(
    State(combined): State<CombinedState>,
) -> Json<Vec<serde_json::Value>> {
    let projects = sqlx::query("SELECT name, owner_id FROM projects")
        .fetch_all(&combined.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| serde_json::json!({"name": row.get::<String, _>("name"), "owner_id": row.get::<Uuid, _>("owner_id")}))
        .collect();
    Json(projects)
}

// POST /create_project
async fn create_project_handler(
    State(combined): State<CombinedState>,
    headers: HeaderMap,
    Json(payload): Json<CreateProjectRequest>,
) -> (StatusCode, Json<CreateProjectResponse>) {
    // Authenticate user
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| {
            return (
                StatusCode::UNAUTHORIZED,
                Json(CreateProjectResponse {
                    message: "Authorization required".to_string(),
                }),
            );
        })
        .unwrap();

    let token_data = match crate::users::decode_token(auth_header) {
        Ok(data) => data,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(CreateProjectResponse {
                    message: "Invalid token".to_string(),
                }),
            );
        }
    };

    let owner_id: Uuid = Uuid::parse_str(&token_data.sub).unwrap();

    // Check if project exists in DB
    let existing = sqlx::query("SELECT id FROM projects WHERE name = $1")
        .bind(&payload.name)
        .fetch_optional(&combined.db)
        .await;
    if existing.map(|opt| opt.is_some()).unwrap_or(false) {
        return (
            StatusCode::CONFLICT,
            Json(CreateProjectResponse {
                message: "Project already exists".to_string(),
            }),
        );
    }

    // Insert into DB
    let project_id =
        match sqlx::query("INSERT INTO projects (name, owner_id) VALUES ($1, $2) RETURNING id")
            .bind(&payload.name)
            .bind(&owner_id)
            .fetch_one(&combined.db)
            .await
        {
            Ok(row) => row.get::<Uuid, _>("id"),
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(CreateProjectResponse {
                        message: "Failed to create project in DB".to_string(),
                    }),
                );
            }
        };

    // Insert admin permission for owner
    if sqlx::query("INSERT INTO permissions (user_id, project, access) VALUES ($1, $2, 'admin')")
        .bind(&owner_id)
        .bind(&payload.name)
        .execute(&combined.db)
        .await
        .is_err()
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateProjectResponse {
                message: "Failed to set owner permission".to_string(),
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
            let mut projects = combined.projects.lock().await;
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

// DELETE /projects/{name}
async fn delete_project_handler(
    State(combined): State<CombinedState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> StatusCode {
    // Authenticate user
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)
        .unwrap();

    let token_data = match crate::users::decode_token(auth_header) {
        Ok(data) => data,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };

    let user_id: Uuid = Uuid::parse_str(&token_data.sub).unwrap();

    // Check if user is owner or global admin
    let project_owner = sqlx::query("SELECT owner_id FROM projects WHERE name = $1")
        .bind(&name)
        .fetch_optional(&combined.db)
        .await
        .unwrap_or(None);

    let is_owner = project_owner
        .as_ref()
        .map(|row| row.get::<Uuid, _>("owner_id") == user_id)
        .unwrap_or(false);
    let is_admin = sqlx::query("SELECT role FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(&combined.db)
        .await
        .unwrap_or(None)
        .map(|row| row.get::<String, _>("role") == "admin")
        .unwrap_or(false);

    if !is_owner && !is_admin {
        return StatusCode::FORBIDDEN;
    }

    // Delete from DB (cascade permissions)
    if sqlx::query("DELETE FROM projects WHERE name = $1")
        .bind(&name)
        .execute(&combined.db)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Delete filesystem directory
    let project_dir = PathBuf::from("server/examples").join(&name);
    if let Err(_) = fs::remove_dir_all(&project_dir) {
        // Log error but don't fail
    }

    // Remove from in-memory state
    let mut projects = combined.projects.lock().await;
    projects.remove(&name);

    StatusCode::NO_CONTENT
}

// --- Artifacts Module ---

mod artifacts;
mod users;
