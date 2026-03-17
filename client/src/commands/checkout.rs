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
