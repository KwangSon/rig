use crate::CombinedState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct FileListQuery {
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub locked_by: Option<String>,
}

pub async fn list_files_handler(
    Path(project): Path<String>,
    Query(query): Query<FileListQuery>,
    State(combined): State<CombinedState>,
) -> Result<Json<Vec<FileEntry>>, StatusCode> {
    let mut entries_map: HashMap<String, FileEntry> = HashMap::new();
    let prefix = query.path.unwrap_or_else(|| "".to_string());

    // Ensure prefix handles trailing slashes logic correctly
    let prefix_with_slash = if prefix.is_empty() {
        "".to_string()
    } else if !prefix.ends_with('/') {
        format!("{}/", prefix)
    } else {
        prefix.clone()
    };

    // 1. Get flat list of paths from in-memory state (our logical file tree)
    {
        let projects = combined.projects.lock().await;
        let app_state = projects.get(&project).ok_or(StatusCode::NOT_FOUND)?;

        for file_path in app_state.artifacts.keys() {
            if file_path.starts_with(&prefix_with_slash) {
                // Determine what the direct child is relative to the requested prefix
                let relative_path = file_path.strip_prefix(&prefix_with_slash).unwrap();

                if relative_path.is_empty() {
                    continue; // Skip the directory itself if exact match
                }

                let parts: Vec<&str> = relative_path.split('/').collect();

                if parts.len() == 1 {
                    // It's a file in this directory
                    entries_map.insert(
                        parts[0].to_string(),
                        FileEntry {
                            name: parts[0].to_string(),
                            path: file_path.clone(),
                            is_dir: false,
                            locked_by: None, // Will fill with DB query
                        },
                    );
                } else {
                    // It's a directory
                    let dir_name = parts[0].to_string();
                    let dir_path = format!("{}{}", prefix_with_slash, dir_name);
                    entries_map.insert(
                        dir_name.clone(),
                        FileEntry {
                            name: dir_name,
                            path: dir_path,
                            is_dir: true,
                            locked_by: None,
                        },
                    );
                }
            }
        }
    }

    // 2. Fetch lock metadata from PostgreSQL for the files in this directory
    // We only need to check locks for actual files, not directories in our map
    let file_paths: Vec<String> = entries_map
        .values()
        .filter(|e| !e.is_dir)
        .map(|e| e.path.clone())
        .collect();

    if !file_paths.is_empty() {
        // Warning: For extreme amounts of files in a single dir this might be a large query
        // but it scales fine for reasonable directory sizes
        let locks = sqlx::query(
            "SELECT file_path, locked_by FROM file_locks WHERE project = $1 AND file_path = ANY($2)",
        )
        .bind(&project)
        .bind(&file_paths)
        .fetch_all(&combined.db)
        .await
        .map_err(|e| {
            eprintln!("Error fetching locks: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        for lock in locks {
            let lock_file_path: String = lock.get("file_path");
            let locked_by: Option<String> = lock.get("locked_by");
            if let Some(entry) =
                entries_map.get_mut(lock_file_path.strip_prefix(&prefix_with_slash).unwrap())
            {
                entry.locked_by = locked_by;
            }
        }
    }

    let mut result: Vec<FileEntry> = entries_map.into_values().collect();

    // Sort directories first, then alphabetical
    result.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.cmp(&b.name)
        }
    });

    Ok(Json(result))
}
