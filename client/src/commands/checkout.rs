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

    // Check for staged changes
    let has_staged = local_index.artifacts.values().any(|a| a.stage == "staged");
    // Check for unpushed commits
    let has_unpushed = local_index.head.is_some();

    if has_staged || has_unpushed {
        return Err("ERROR: Uncommitted changes. Commit or stash first.".into());
    }

    // Fast-path...
    let current_head = repo.read_head()?;
    let target_head_ref = format!("ref: {}", ref_path);
    if current_head == target_head_ref {
        println!("Already on '{}'", branch_name);
        return Ok(());
    }

    // Update HEAD
    repo.write_head(&format!("ref: {}", ref_path))?;

    // Setup working tree for the new branch's commit
    if let Some(commit) = repo.read_commit(&commit_hash)? {
        use std::os::unix::fs::PermissionsExt;
        let mut index = local_index;

        // Remove currently tracked files from the workspace
        for path in index.artifacts.keys() {
            let file_path = current_dir.join(path);
            if file_path.exists() {
                let _ = fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644));
                let _ = fs::remove_file(&file_path);
            }
        }

        // Reset artifacts map for the new branch
        index.artifacts.clear();

        // For the new branch, write empty placeholders and update index
        for commit_artifact in &commit.artifacts {
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

            // Write empty file as placeholder
            if fs::write(&file_path, b"").is_ok() {
                let _ = fs::set_permissions(&file_path, fs::Permissions::from_mode(0o444));
            }
        }

        repo.write_index(&index)?;
    }

    println!("Switched to branch '{}'", branch_name);
    println!("(Run 'rig pull' to download actual file contents for this branch)");

    Ok(())
}
