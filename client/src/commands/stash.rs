use crate::commands::{add, commit, push};
use crate::repository::Repository;
use sha1::Digest;
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

    // Create a reverse map from path -> artifact
    let mut path_to_artifact = std::collections::HashMap::new();
    for artifact in local_index.artifacts.values() {
        path_to_artifact.insert(artifact.path.as_str(), artifact);
    }

    for rel_path in all_files {
        let path_str = rel_path.to_string_lossy().to_string();
        let mut needs_add = false;

        if let Some(artifact) = path_to_artifact.get(path_str.as_str()) {
            if artifact.latest == 0 {
                // Tracking an unpushed file
                println!("Condition A: latest==0 for {}", path_str);
                needs_add = true;
            } else {
                // Tracking a pushed file, check if modified (writable)
                let full_path = current_dir.join(&rel_path);
                if fs::metadata(&full_path).is_ok_and(|m| !m.permissions().readonly()) {
                    // It's writable, but did the content actually change?
                    if let Ok(file_data) = fs::read(&full_path) {
                        let mut hasher = sha1::Sha1::new();
                        sha1::Digest::update(&mut hasher, &file_data);
                        let file_hash = format!("{:x}", sha1::Digest::finalize(hasher));

                        let latest_rev =
                            artifact.revisions.iter().find(|r| r.rev == artifact.latest);
                        let is_modified = latest_rev.map_or(true, |r| r.hash != file_hash);
                        println!(
                            "Checking writable tracker file {}: hash={}, latest_hash={:?}, is_modified={}",
                            path_str,
                            file_hash,
                            latest_rev.map(|r| &r.hash),
                            is_modified
                        );
                        if is_modified {
                            println!("Condition B: modified hash for {}", path_str);
                            needs_add = true;
                        }
                    } else {
                        println!("Condition C: read err for {}", path_str);
                        needs_add = true; // Err reading, assume dirty
                    }
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
    if let Some(commit_hash) = repo.head_commit()? {
        if let Some(original_commit) = repo.read_commit(&commit_hash)? {
            let mut index = repo.read_index()?;
            // Remove ALL workspace files to ensure a completely clean slate
            let all_workspace_files = collect_files(&current_dir, &current_dir);
            for rel_path in all_workspace_files {
                let file_path = current_dir.join(&rel_path);
                if file_path.exists() {
                    if let Ok(m) = fs::metadata(&file_path) {
                        let mut perms = m.permissions();
                        #[allow(clippy::permissions_set_readonly_false)]
                        perms.set_readonly(false);
                        let _ = fs::set_permissions(&file_path, perms);
                    }
                    if file_path.is_dir() {
                        let _ = fs::remove_dir_all(&file_path);
                    } else {
                        let _ = fs::remove_file(&file_path);
                    }
                }
            }

            // Rebuild index.artifacts from the original commit's artifacts
            // We do not want to keep anything that was added during the stash
            let mut original_artifacts = std::collections::HashMap::new();

            for (artifact_id, rev) in &original_commit.artifacts {
                if let Some(mut artifact) = index.artifacts.get(artifact_id).cloned() {
                    artifact.latest = *rev;
                    original_artifacts.insert(artifact_id.clone(), artifact.clone());

                    let file_path = current_dir.join(&artifact.path);
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).ok();
                    }
                    if fs::write(&file_path, b"").is_ok() {
                        if let Ok(mut perms) = fs::metadata(&file_path).map(|m| m.permissions()) {
                            perms.set_readonly(true);
                            let _ = fs::set_permissions(&file_path, perms);
                        }
                    }
                }
            }

            index.artifacts = original_artifacts;
            repo.write_index(&index)?;
        }
    }

    println!("\nStash successful! Your changes are safely shelved on the server.");
    println!("Shelf branch: {}", shelf_ref);
    Ok(())
}
