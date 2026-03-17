use crate::repository::Repository;
use std::fs;

pub async fn run(branch_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;

    let ref_path = format!("refs/heads/{}", branch_name);

    // Check if branch exists
    let commit_hash = match repo.resolve_ref(&ref_path)? {
        Some(hash) => hash,
        None => return Err(format!("Branch '{}' does not exist.", branch_name).into()),
    };

    // Before proceeding, check for dirty workspace
    let local_index = repo.read_index()?;
    let current_parent = repo.head_commit()?;
    let _latest_commit = if let Some(hash) = &current_parent {
        repo.read_commit(hash).unwrap_or(None)
    } else {
        None
    };

    // Fast-path: If the branch we are checking out is exactly the branch we are already on,
    // we don't need to do any work or check for dirtiness.
    let current_head = repo.read_head()?;
    let target_head_ref = format!("ref: {}", ref_path);
    if current_head == target_head_ref {
        println!("Already on '{}'", branch_name);
        return Ok(());
    }

    let mut is_dirty = false;

    // Scan all files in the current directory (ignoring .rig and target)
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

    // Create a reverse map from path -> artifact
    let mut path_to_artifact = std::collections::HashMap::new();
    for artifact in local_index.artifacts.values() {
        path_to_artifact.insert(artifact.path.as_str(), artifact);
    }

    for rel_path in all_files {
        let path_str = rel_path.to_string_lossy().to_string();
        if let Some(artifact) = path_to_artifact.get(path_str.as_str()) {
            if artifact.latest == 0 {
                // Tracking an unpushed file
                println!("Checkout blocked by unpushed file: {}", path_str);
                is_dirty = true;
                break;
            } else {
                // Tracking a pushed file, check if modified
                let full_path = current_dir.join(&rel_path);
                if fs::metadata(full_path).is_ok_and(|m| !m.permissions().readonly()) {
                    println!("Checkout blocked by modified tracked file: {}", path_str);
                    is_dirty = true;
                    break;
                }
            }
        } else {
            // Untracked file
            println!("Checkout blocked by untracked file: {}", path_str);
            is_dirty = true;
            break;
        }
    }

    if is_dirty {
        return Err("Checkout aborted: You have modified or unpushed tracked files.\nPlease 'rig commit' and 'rig push', or use 'rig stash' to shelve your changes remotely.".into());
    }

    // Update HEAD
    repo.write_head(&format!("ref: {}", ref_path))?;

    // Setup working tree for the new branch's commit
    if let Some(commit) = repo.read_commit(&commit_hash)? {
        let mut index = repo.read_index()?;

        // Remove currently tracked files from the workspace
        for artifact in index.artifacts.values() {
            let file_path = current_dir.join(&artifact.path);
            if file_path.exists() {
                // Ignore errors (user might have deleted it, or permissions)
                let mut perms = fs::metadata(&file_path)?.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = fs::set_permissions(&file_path, perms);
                let _ = fs::remove_file(&file_path);
            }
        }

        // For the new branch, write empty placeholders and update index.latest
        for (artifact_id, rev) in &commit.artifacts {
            if let Some(artifact) = index.artifacts.get_mut(artifact_id) {
                artifact.latest = *rev;

                let file_path = current_dir.join(&artifact.path);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).ok();
                }

                // Write empty file as placeholder (Rig only downloads on pull)
                if fs::write(&file_path, b"").is_ok() {
                    if let Ok(mut perms) = fs::metadata(&file_path).map(|m| m.permissions()) {
                        perms.set_readonly(true);
                        let _ = fs::set_permissions(&file_path, perms);
                    }
                }
            }
        }

        repo.write_index(&index)?;
    }

    println!("Switched to branch '{}'", branch_name);
    println!("(Run 'rig pull' to download actual file contents for this branch)");

    Ok(())
}
