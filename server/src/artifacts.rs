use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{AppState, Artifact, Lock, LockRequest, LockResponse, Revision, SharedState};

#[derive(Deserialize)]
pub struct CreateArtifactRequest {
    path: String,
    content_base64: String,
    message: Option<String>,
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
    message: Option<String>,
}

#[derive(Serialize)]
pub struct CreateRevisionResponse {
    revision: u32,
}

// Placeholder implementations

pub async fn create_artifact_handler(
    State(state): State<SharedState>,
    Json(payload): Json<CreateArtifactRequest>,
) -> (StatusCode, Json<CreateArtifactResponse>) {
    let mut app_state = state.lock().await;

    if app_state.artifacts.iter().any(|a| a.id == payload.path) {
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
    if let Err(_e) = fs::write(&rev_path, &bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateArtifactResponse {
                artifact_id: payload.path,
            }),
        );
    }

    app_state.artifacts.push(Artifact {
        id: payload.path.clone(),
        path: payload.path.clone(),
        latest_revision: 1,
        revisions: vec![Revision {
            revision: 1,
            message: payload.message.unwrap_or_default(),
        }],
        lock: None,
    });

    // Persist index.json (best-effort)
    if let Err(e) = persist_index(&app_state) {
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
    State(_state): State<SharedState>,
) -> Json<Vec<ArtifactShortResponse>> {
    Json(vec![ArtifactShortResponse {
        id: "a1".to_string(),
        path: "A".to_string(),
    }])
}

pub async fn get_artifact_info_handler(
    Path(_id): Path<String>,
    State(_state): State<SharedState>,
) -> Json<ArtifactInfoResponse> {
    Json(ArtifactInfoResponse {
        id: "a1".to_string(),
        path: "A".to_string(),
        latest_revision: 3,
    })
}

pub async fn create_revision_handler(
    Path(id): Path<String>,
    State(state): State<SharedState>,
    Json(payload): Json<CreateRevisionRequest>,
) -> (StatusCode, Json<CreateRevisionResponse>) {
    let mut app_state = state.lock().await;
    let project_dir = app_state.project_dir.clone();

    let artifact = match app_state.artifacts.iter_mut().find(|a| a.id == id) {
        Some(a) => a,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(CreateRevisionResponse { revision: 0 }),
            );
        }
    };

    // Must be locked by someone to allow revision
    if artifact.lock.is_none() {
        return (
            StatusCode::CONFLICT,
            Json(CreateRevisionResponse { revision: 0 }),
        );
    }

    let new_rev = artifact.latest_revision + 1;
    let artifact_dir = project_dir.join("artifacts").join(&artifact.id);
    fs::create_dir_all(&artifact_dir).ok();

    let rev_path = artifact_dir.join(format!("rev{}.blend", new_rev));
    let bytes = general_purpose::STANDARD
        .decode(&payload.content_base64)
        .unwrap_or_default();
    if let Err(_e) = fs::write(&rev_path, &bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateRevisionResponse { revision: 0 }),
        );
    }

    artifact.latest_revision = new_rev;
    artifact.revisions.push(Revision {
        revision: new_rev,
        message: payload.message.unwrap_or_default(),
    });

    // Persist index.json (best-effort)
    if let Err(e) = persist_index(&app_state) {
        eprintln!("Failed to persist index.json: {}", e);
    }

    (
        StatusCode::CREATED,
        Json(CreateRevisionResponse { revision: new_rev }),
    )
}

pub async fn get_revisions_handler(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Json<Vec<RevisionShortResponse>> {
    let app_state = state.lock().await;
    if let Some(artifact) = app_state.artifacts.iter().find(|a| a.id == id) {
        Json(
            artifact
                .revisions
                .iter()
                .map(|r| RevisionShortResponse {
                    revision: r.revision,
                })
                .collect(),
        )
    } else {
        Json(vec![])
    }
}

pub async fn lock_handler(
    Path(id): Path<String>,
    State(state): State<SharedState>,
    Json(payload): Json<LockRequest>,
) -> (StatusCode, Json<LockResponse>) {
    let mut app_state = state.lock().await;
    if let Some(artifact) = app_state.artifacts.iter_mut().find(|a| a.id == id) {
        if artifact.lock.is_none() {
            artifact.lock = Some(Lock {
                user: payload.user.clone(),
            });
            // Persist lock state
            if let Err(e) = persist_index(&app_state) {
                eprintln!("Failed to persist lock state: {}", e);
            }
            (
                StatusCode::OK,
                Json(LockResponse {
                    locked: true,
                    user: Some(payload.user),
                }),
            )
        } else if artifact.lock.as_ref().map(|l| l.user.clone()) == Some(payload.user.clone()) {
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
                    user: artifact.lock.as_ref().map(|l| l.user.clone()),
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
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> (StatusCode, Json<LockResponse>) {
    let mut app_state = state.lock().await;
    if let Some(artifact) = app_state.artifacts.iter_mut().find(|a| a.id == id) {
        artifact.lock = None;
        // Persist lock state
        if let Err(e) = persist_index(&app_state) {
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
            StatusCode::NOT_FOUND,
            Json(LockResponse {
                locked: false,
                user: None,
            }),
        )
    }
}

pub async fn get_lock_handler(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Json<LockResponse> {
    let app_state = state.lock().await;
    if let Some(artifact) = app_state.artifacts.iter().find(|a| a.id == id) {
        Json(LockResponse {
            locked: artifact.lock.is_some(),
            user: artifact.lock.as_ref().map(|l| l.user.clone()),
        })
    } else {
        Json(LockResponse {
            locked: false,
            user: None,
        })
    }
}

fn persist_index(app_state: &AppState) -> Result<(), String> {
    let index_path = app_state.project_dir.join("index.json");
    let mut index_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

    let mut artifacts_map = serde_json::Map::new();
    for artifact in &app_state.artifacts {
        let mut artifact_obj = serde_json::Map::new();
        artifact_obj.insert(
            "path".to_string(),
            serde_json::Value::String(artifact.path.clone()),
        );
        artifact_obj.insert(
            "latest".to_string(),
            serde_json::Value::Number(artifact.latest_revision.into()),
        );
        artifact_obj.insert(
            "locked_by".to_string(),
            match &artifact.lock {
                Some(lock) => serde_json::Value::String(lock.user.clone()),
                None => serde_json::Value::Null,
            },
        );

        let revisions: Vec<serde_json::Value> = artifact
            .revisions
            .iter()
            .map(|r| {
                let mut m = serde_json::Map::new();
                m.insert(
                    "rev".to_string(),
                    serde_json::Value::Number(r.revision.into()),
                );
                m.insert(
                    "hash".to_string(),
                    serde_json::Value::String("".to_string()),
                );
                serde_json::Value::Object(m)
            })
            .collect();

        artifact_obj.insert("revisions".to_string(), serde_json::Value::Array(revisions));
        artifacts_map.insert(artifact.id.clone(), serde_json::Value::Object(artifact_obj));
    }

    index_value["artifacts"] = serde_json::Value::Object(artifacts_map);

    fs::write(
        &index_path,
        serde_json::to_string_pretty(&index_value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
