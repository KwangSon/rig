use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{AppState, LockRequest, LockResponse, SharedState, UnlockRequest};
use protocol::{Artifact, Revision};

#[derive(Deserialize)]
pub struct CreateArtifactRequest {
    path: String,
    content_base64: String,
    _message: Option<String>,
}

#[derive(Serialize)]
pub struct CreateArtifactResponse {
    artifact_id: String,
}

#[derive(Serialize)]
pub struct ArtifactShortResponse {
    id: String,
    path: String,
}

#[derive(Serialize)]
pub struct ArtifactInfoResponse {
    id: String,
    path: String,
    latest_revision: u32,
}

#[derive(Serialize)]
pub struct RevisionShortResponse {
    revision: u32,
}

#[derive(Deserialize)]
pub struct CreateRevisionRequest {
    content_base64: String,
    _message: Option<String>,
}

#[derive(Serialize)]
pub struct CreateRevisionResponse {
    revision: u32,
}

// Placeholder implementations

pub async fn create_artifact_handler(
    Path(project): Path<String>,
    State(state): State<SharedState>,
    Json(payload): Json<CreateArtifactRequest>,
) -> (StatusCode, Json<CreateArtifactResponse>) {
    let mut projects = state.lock().await;
    let app_state = match projects.get_mut(&project) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(CreateArtifactResponse {
                    artifact_id: payload.path,
                }),
            );
        }
    };

    if app_state.artifacts.contains_key(&payload.path) {
        return (
            StatusCode::CONFLICT,
            Json(CreateArtifactResponse {
                artifact_id: payload.path,
            }),
        );
    }

    // Ensure artifacts directory exists
    let artifact_dir = app_state.project_dir.join("artifacts").join(&payload.path);
    fs::create_dir_all(&artifact_dir).ok();

    // Write first revision file
    let rev_path = artifact_dir.join("rev1.blend");
    let bytes = general_purpose::STANDARD
        .decode(&payload.content_base64)
        .unwrap_or_default();
    if fs::write(&rev_path, &bytes).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateArtifactResponse {
                artifact_id: payload.path,
            }),
        );
    }

    app_state.artifacts.insert(
        payload.path.clone(),
        Artifact {
            path: payload.path.clone(),
            latest: 1,
            locked_by: None,
            revisions: vec![Revision::new(1, &bytes)],
        },
    );

    // Persist index.json (best-effort)
    if let Err(e) = persist_index(app_state) {
        eprintln!("Failed to persist index.json: {}", e);
    }

    (
        StatusCode::CREATED,
        Json(CreateArtifactResponse {
            artifact_id: payload.path,
        }),
    )
}

pub async fn get_artifacts_handler(
    Path(project): Path<String>,
    State(state): State<SharedState>,
) -> Json<Vec<ArtifactShortResponse>> {
    let projects = state.lock().await;
    if let Some(app_state) = projects.get(&project) {
        Json(
            app_state
                .artifacts
                .iter()
                .map(|(id, a)| ArtifactShortResponse {
                    id: id.clone(),
                    path: a.path.clone(),
                })
                .collect(),
        )
    } else {
        Json(vec![])
    }
}

pub async fn get_artifact_info_handler(
    Path((project, id)): Path<(String, String)>,
    State(state): State<SharedState>,
) -> Json<ArtifactInfoResponse> {
    let projects = state.lock().await;
    if let Some(app_state) = projects.get(&project)
        && let Some(artifact) = app_state.artifacts.get(&id)
    {
        return Json(ArtifactInfoResponse {
            id: id.clone(),
            path: artifact.path.clone(),
            latest_revision: artifact.latest,
        });
    }

    Json(ArtifactInfoResponse {
        id: "".to_string(),
        path: "".to_string(),
        latest_revision: 0,
    })
}

pub async fn create_revision_handler(
    Path((project, id)): Path<(String, String)>,
    State(state): State<SharedState>,
    Json(payload): Json<CreateRevisionRequest>,
) -> (StatusCode, Json<CreateRevisionResponse>) {
    let mut projects = state.lock().await;
    let app_state = match projects.get_mut(&project) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(CreateRevisionResponse { revision: 0 }),
            );
        }
    };
    let project_dir = app_state.project_dir.clone();

    let artifact = match app_state.artifacts.get_mut(&id) {
        Some(a) => a,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(CreateRevisionResponse { revision: 0 }),
            );
        }
    };

    // Must be locked by someone to allow revision
    if artifact.locked_by.is_none() {
        return (
            StatusCode::CONFLICT,
            Json(CreateRevisionResponse { revision: 0 }),
        );
    }

    let new_rev = artifact.latest + 1;
    let artifact_dir = project_dir.join("artifacts").join(&id);
    fs::create_dir_all(&artifact_dir).ok();

    let rev_path = artifact_dir.join(format!("rev{}.blend", new_rev));
    let bytes = general_purpose::STANDARD
        .decode(&payload.content_base64)
        .unwrap_or_default();
    if fs::write(&rev_path, &bytes).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateRevisionResponse { revision: 0 }),
        );
    }

    artifact.latest = new_rev;
    artifact.revisions.push(Revision::new(new_rev, &bytes));

    // Persist index.json (best-effort)
    if let Err(e) = persist_index(app_state) {
        eprintln!("Failed to persist index.json: {}", e);
    }

    (
        StatusCode::CREATED,
        Json(CreateRevisionResponse { revision: new_rev }),
    )
}

pub async fn get_revisions_handler(
    Path((project, id)): Path<(String, String)>,
    State(state): State<SharedState>,
) -> Json<Vec<RevisionShortResponse>> {
    let projects = state.lock().await;
    if let Some(app_state) = projects.get(&project)
        && let Some(artifact) = app_state.artifacts.get(&id)
    {
        return Json(
            artifact
                .revisions
                .iter()
                .map(|r| RevisionShortResponse { revision: r.rev })
                .collect(),
        );
    }

    Json(vec![])
}

pub async fn lock_handler(
    Path((project, id)): Path<(String, String)>,
    State(state): State<SharedState>,
    Json(payload): Json<LockRequest>,
) -> (StatusCode, Json<LockResponse>) {
    let mut projects = state.lock().await;
    let app_state = match projects.get_mut(&project) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(LockResponse {
                    locked: false,
                    user: None,
                }),
            );
        }
    };

    if let Some(artifact) = app_state.artifacts.get_mut(&id) {
        if artifact.locked_by.is_none() {
            artifact.locked_by = Some(payload.user.clone());
            // Persist lock state
            if let Err(e) = persist_index(app_state) {
                eprintln!("Failed to persist lock state: {}", e);
            }
            (
                StatusCode::OK,
                Json(LockResponse {
                    locked: true,
                    user: Some(payload.user),
                }),
            )
        } else if artifact.locked_by == Some(payload.user.clone()) {
            // Already locked by the same user: treat as success
            (
                StatusCode::OK,
                Json(LockResponse {
                    locked: true,
                    user: Some(payload.user),
                }),
            )
        } else {
            (
                StatusCode::CONFLICT,
                Json(LockResponse {
                    locked: false,
                    user: artifact.locked_by.clone(),
                }),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(LockResponse {
                locked: false,
                user: None,
            }),
        )
    }
}

pub async fn unlock_handler(
    Path((project, id)): Path<(String, String)>,
    State(state): State<SharedState>,
    State(user_state): State<crate::users::SharedUserState>,
    Json(payload): Json<UnlockRequest>,
) -> (StatusCode, Json<LockResponse>) {
    let mut projects = state.lock().await;
    let app_state = match projects.get_mut(&project) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(LockResponse {
                    locked: false,
                    user: None,
                }),
            );
        }
    };

    if let Some(artifact) = app_state.artifacts.get_mut(&id) {
        if let Some(ref locked_by) = artifact.locked_by {
            let mut allow_unlock = false;

            if locked_by == &payload.user {
                allow_unlock = true;
            } else if payload.force {
                // Check if user has force-unlock permissions (admin)
                let users_data = user_state.lock().await;
                let user_id = users_data
                    .users
                    .iter()
                    .find(|u| u.name == payload.user)
                    .map(|u| u.id.clone());

                if let Some(uid) = user_id {
                    let is_global_admin = users_data
                        .users
                        .iter()
                        .find(|u| u.id == uid)
                        .map(|u| u.role == "admin")
                        .unwrap_or(false);

                    let has_project_admin = users_data
                        .permissions
                        .iter()
                        .any(|p| p.user_id == uid && p.project == project && p.access == "admin");

                    if is_global_admin || has_project_admin {
                        allow_unlock = true;
                    }
                }
            }

            if allow_unlock {
                artifact.locked_by = None;
                // Persist lock state
                if let Err(e) = persist_index(app_state) {
                    eprintln!("Failed to persist lock state: {}", e);
                }
                (
                    StatusCode::OK,
                    Json(LockResponse {
                        locked: false,
                        user: None,
                    }),
                )
            } else {
                (
                    StatusCode::FORBIDDEN,
                    Json(LockResponse {
                        locked: true,
                        user: Some(locked_by.clone()),
                    }),
                )
            }
        } else {
            // Already unlocked
            (
                StatusCode::OK,
                Json(LockResponse {
                    locked: false,
                    user: None,
                }),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(LockResponse {
                locked: false,
                user: None,
            }),
        )
    }
}

pub async fn get_lock_handler(
    Path((project, id)): Path<(String, String)>,
    State(state): State<SharedState>,
) -> Json<LockResponse> {
    let projects = state.lock().await;
    if let Some(app_state) = projects.get(&project)
        && let Some(artifact) = app_state.artifacts.get(&id)
    {
        return Json(LockResponse {
            locked: artifact.locked_by.is_some(),
            user: artifact.locked_by.clone(),
        });
    }

    Json(LockResponse {
        locked: false,
        user: None,
    })
}

pub async fn get_index_handler(
    Path(project): Path<String>,
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let projects = state.lock().await;
    if let Some(app_state) = projects.get(&project) {
        let index_path = app_state.project_dir.join("index.json");
        if let Ok(content) = fs::read_to_string(&index_path)
            && let Ok(value) = serde_json::from_str(&content)
        {
            return Json(value);
        }
    }

    Json(serde_json::json!({
        "error": "project not found",
    }))
}

pub async fn update_gitmodule_handler(
    Path((project, path)): Path<(String, String)>,
    State(state): State<SharedState>,
    Json(payload): Json<protocol::GitModule>,
) -> StatusCode {
    let mut projects = state.lock().await;
    let app_state = match projects.get_mut(&project) {
        Some(s) => s,
        None => return StatusCode::NOT_FOUND,
    };

    app_state.git_modules.insert(path, payload);

    if let Err(e) = persist_index(app_state) {
        eprintln!("Failed to persist gitmodule update: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

fn persist_index(app_state: &AppState) -> Result<(), String> {
    let index_path = app_state.project_dir.join("index.json");
    let mut index_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

    let mut artifacts_map = serde_json::Map::new();
    for (id, artifact) in &app_state.artifacts {
        let mut artifact_obj = serde_json::Map::new();
        artifact_obj.insert(
            "path".to_string(),
            serde_json::Value::String(artifact.path.clone()),
        );
        artifact_obj.insert(
            "latest".to_string(),
            serde_json::Value::Number(artifact.latest.into()),
        );
        artifact_obj.insert(
            "locked_by".to_string(),
            match &artifact.locked_by {
                Some(user) => serde_json::Value::String(user.clone()),
                None => serde_json::Value::Null,
            },
        );

        let revisions: Vec<serde_json::Value> = artifact
            .revisions
            .iter()
            .map(|r| {
                let mut m = serde_json::Map::new();
                m.insert("rev".to_string(), serde_json::Value::Number(r.rev.into()));
                m.insert(
                    "hash".to_string(),
                    serde_json::Value::String(r.hash.clone()),
                );
                serde_json::Value::Object(m)
            })
            .collect();

        artifact_obj.insert("revisions".to_string(), serde_json::Value::Array(revisions));
        artifacts_map.insert(id.clone(), serde_json::Value::Object(artifact_obj));
    }

    index_value["artifacts"] = serde_json::Value::Object(artifacts_map);

    let mut modules_map = serde_json::Map::new();
    for (id, module) in &app_state.git_modules {
        let mut module_obj = serde_json::Map::new();
        module_obj.insert(
            "path".to_string(),
            serde_json::Value::String(module.path.clone()),
        );
        module_obj.insert(
            "url".to_string(),
            serde_json::Value::String(module.url.clone()),
        );
        module_obj.insert(
            "commit".to_string(),
            serde_json::Value::String(module.commit.clone()),
        );
        modules_map.insert(id.clone(), serde_json::Value::Object(module_obj));
    }
    index_value["git_modules"] = serde_json::Value::Object(modules_map);

    fs::write(
        &index_path,
        serde_json::to_string_pretty(&index_value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Deserialize)]
pub struct PushRequest {
    pub commit: protocol::Commit,
    pub updated_artifacts: std::collections::HashMap<String, String>,
}

pub async fn push_handler(
    Path(project): Path<String>,
    State(state): State<SharedState>,
    Json(payload): Json<PushRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut projects = state.lock().await;
    let app_state = match projects.get_mut(&project) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Project not found"})),
            );
        }
    };

    let mut commits_map = std::collections::HashMap::new();

    // Read index.json to get current state
    let index_path = app_state.project_dir.join("index.json");
    if let Ok(index_content) = fs::read_to_string(&index_path)
        && let Ok(index_val) = serde_json::from_str::<serde_json::Value>(&index_content)
    {
        if let Ok(c) = serde_json::from_value::<std::collections::HashMap<String, protocol::Commit>>(
            index_val["commits"].clone(),
        ) {
            commits_map = c;
        }
    }

    // Process updated artifacts
    for (id, content_base64) in payload.updated_artifacts {
        let bytes = general_purpose::STANDARD
            .decode(&content_base64)
            .unwrap_or_default();

        if let Some(artifact) = app_state.artifacts.get_mut(&id) {
            let new_rev = artifact.latest + 1;
            let artifact_dir = app_state.project_dir.join("artifacts").join(&id);
            fs::create_dir_all(&artifact_dir).ok();

            let rev_path = artifact_dir.join(format!("rev{}.blend", new_rev));
            if fs::write(&rev_path, &bytes).is_err() {
                continue;
            }

            artifact.latest = new_rev;
            artifact.revisions.push(Revision::new(new_rev, &bytes));
            artifact.locked_by = None; // Auto-unlock on push
        } else {
            // New artifact created on client and pushed
            println!("Creating new artifact '{}' during push", id);
            let artifact_dir = app_state.project_dir.join("artifacts").join(&id);
            fs::create_dir_all(&artifact_dir).ok();

            let rev_path = artifact_dir.join("rev1.blend");
            if fs::write(&rev_path, &bytes).is_err() {
                continue;
            }

            app_state.artifacts.insert(
                id.clone(),
                Artifact {
                    path: id.clone(),
                    latest: 1,
                    locked_by: None,
                    revisions: vec![Revision::new(1, &bytes)],
                },
            );
        }
    }

    // Use the client's commit as the authoritative one
    let new_commit = payload.commit;
    let new_hash = new_commit.hash.clone();
    commits_map.insert(new_hash.clone(), new_commit.clone());

    // Update index.json
    let mut full_index: protocol::IndexFile = protocol::IndexFile {
        project: project.clone(),
        server_url: None,
        username: None,
        latest_commit: new_hash.clone(),
        artifacts: app_state.artifacts.clone(),
        git_modules: app_state.git_modules.clone(),
        commits: commits_map,
    };

    // Try to preserve server_url from existing index
    if let Ok(index_content) = fs::read_to_string(&index_path)
        && let Ok(index_val) = serde_json::from_str::<serde_json::Value>(&index_content)
    {
        full_index.server_url = index_val["server_url"].as_str().map(|s| s.to_string());
    }

    let _ = fs::write(
        &index_path,
        serde_json::to_string_pretty(&full_index).unwrap(),
    );

    (
        StatusCode::OK,
        Json(serde_json::to_value(new_commit).unwrap()),
    )
}

pub async fn download_artifact_handler(
    Path((project, id, filename)): Path<(String, String, String)>,
    State(state): State<SharedState>,
) -> (StatusCode, Vec<u8>) {
    let projects = state.lock().await;
    let app_state = match projects.get(&project) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, vec![]),
    };
    let file_path = app_state
        .project_dir
        .join("artifacts")
        .join(id)
        .join(filename);
    match fs::read(file_path) {
        Ok(bytes) => (StatusCode::OK, bytes),
        Err(_) => (StatusCode::NOT_FOUND, vec![]),
    }
}
