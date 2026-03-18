use crate::commands::{add, commit, push};
use crate::repository::Repository;

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig stash (Remote Shelving)...");

    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;
    let config = repo.read_config()?;

    // 1. Remember original HEAD
    let original_head = repo.read_head()?;

    // 2. Add all files (dirty or untracked)
    println!("-> Staging all current changes...");

    // Find all dirty/untracked files to add
    fn collect_files(dir: &std::path::Path, base: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut results = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if file_name == ".rig" || file_name.starts_with('.') || file_name == "target" {
                    continue;
                }
                if path.is_dir() {
                    results.extend(collect_files(&path, base));
                } else if let Ok(rel_path) = path.strip_prefix(base) {
                    results.push(rel_path.to_path_buf());
                }
            }
        }
        results
    }

    let all_files = collect_files(&current_dir, &current_dir);
    let local_index = repo.read_index()?;

    for rel_path in all_files {
        let path_str = rel_path.to_string_lossy().to_string();
        let mut needs_add = false;

        if let Some(artifact) = local_index.artifacts.get(&path_str) {
            if artifact.revision == 0 {
                // Tracking an unpushed file
                println!("Condition A: revision==0 for {}", path_str);
                needs_add = true;
            } else {
                // Tracking a pushed file, check if modified (writable)
                let full_path = current_dir.join(&rel_path);
                if fs::metadata(&full_path).is_ok_and(|m| !m.permissions().readonly()) {
                    println!("Condition B: writable file for {}", path_str);
                    needs_add = true;
                }
            }
        } else {
            // Untracked file
            println!("Condition D: untracked for {}", path_str);
            needs_add = true;
        }

        if needs_add {
            add::run(&rel_path).await?;
        }
    }

    // 3. Create a unique shelf branch name
    let username = config.username.unwrap_or_else(|| "unknown".to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let shelf_ref = format!("refs/heads/shelves/{}/{}", username, timestamp);

    // If there's an original commit hash, we branch from there
    if let Some(parent_hash) = repo.head_commit()? {
        repo.write_ref(&shelf_ref, &parent_hash)?;
    }

    // Switch HEAD to the new shelf branch
    repo.write_head(&format!("ref: {}", shelf_ref))?;

    // 4. Commit the staged changes
    let stash_message = format!("Stash by {} at {}", username, timestamp);
    commit::run(&stash_message).await?;

    // 5. Push the shelf to the server
    println!("-> Pushing shelf to remote server...");
    push::run(None).await?;

    // 6. Restore HEAD and reset workspace to clean state (Placeholders)
    println!("-> Restoring original workspace state...");
    repo.write_head(&original_head)?;

    // Re-setup the working tree based on the original commit
    if let Some(commit_hash) = repo.head_commit()?
        && let Some(original_commit) = repo.read_commit(&commit_hash)?
    {
        use std::os::unix::fs::PermissionsExt;
        let mut index = repo.read_index()?;

        // Remove currently tracked files
        for path in index.artifacts.keys() {
            let file_path = current_dir.join(path);
            if file_path.exists() {
                let _ = fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644));
                let _ = fs::remove_file(&file_path);
            }
        }

        // Rebuild index.artifacts from the original commit's artifacts
        index.artifacts.clear();

        for commit_artifact in &original_commit.artifacts {
            let path = &commit_artifact.path;
            index.artifacts.insert(
                path.clone(),
                protocol::IndexArtifact {
                    artifact_id: commit_artifact.artifact_id.clone(),
                    revision: commit_artifact.revision_base,
                    local_state: "placeholder".to_string(),
                    stage: "none".to_string(),
                    locked: false,
                    lock_owner: None,
                    lock_generation: None,
                    staged: None,
                    moved_from: None,
                },
            );

            let file_path = current_dir.join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            if fs::write(&file_path, b"").is_ok() {
                let _ = fs::set_permissions(&file_path, fs::Permissions::from_mode(0o444));
            }
        }

        repo.write_index(&index)?;
    }

    println!("\nStash successful! Your changes are safely shelved on the server.");
    println!("Shelf branch: {}", shelf_ref);
    Ok(())
}
