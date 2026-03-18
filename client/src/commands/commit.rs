use crate::repository::Repository;
use protocol::{Commit, CommitArtifact};
use sha1::{Digest, Sha1};
use std::fs;

pub async fn run(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig commit with message: '{}'", message);

    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;
    let mut local_index = repo.read_index()?;
    let config = repo.read_config()?;

    let current_parent = repo.head_commit()?;

    // 1. Find staged artifacts
    let mut commit_artifacts = Vec::new();

    for (path, artifact) in &local_index.artifacts {
        if artifact.stage == "staged" {
            println!("-> Committing changes for {}", path);

            let local_path = current_dir.join(path);
            if !local_path.exists() {
                return Err(
                    format!("ERROR: Staged file not found: {}", local_path.display()).into(),
                );
            }

            // 2. Stage 2 — Double Hash (spec Stage 2)
            let data1 = fs::read(&local_path)?;
            let mut hasher1 = Sha1::new();
            hasher1.update(&data1);
            let hash1 = format!("{:x}", hasher1.finalize());

            let data2 = fs::read(&local_path)?;
            let mut hasher2 = Sha1::new();
            hasher2.update(&data2);
            let hash2 = format!("{:x}", hasher2.finalize());

            if hash1 != hash2 {
                return Err(format!("ERROR: File '{}' changed during commit. Retry.", path).into());
            }

            commit_artifacts.push(CommitArtifact {
                path: path.clone(),
                artifact_id: artifact.artifact_id.clone(),
                revision_base: artifact.revision,
                hash: hash1,
                op: "upsert".to_string(), // we don't handle delete in add.rs yet
            });
        }
    }

    if commit_artifacts.is_empty() {
        return Err("ERROR: Nothing to commit. Stage changes with 'rig add' first.".into());
    }

    // 3. Create the commit object
    let author = config
        .username
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let mut hasher = Sha1::new();
    hasher.update(
        current_parent
            .as_ref()
            .unwrap_or(&"".to_string())
            .as_bytes(),
    );
    hasher.update(message.as_bytes());
    hasher.update(author.as_bytes());
    hasher.update(serde_json::to_string(&commit_artifacts)?.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let commit = Commit {
        id: hash.clone(),
        parent: current_parent,
        message: message.to_string(),
        author,
        artifacts: commit_artifacts,
        timestamp,
    };

    repo.write_commit(&commit)?;

    // 4. Update index and head
    local_index.head = Some(hash.clone());
    for artifact in local_index.artifacts.values_mut() {
        if artifact.stage == "staged" {
            artifact.stage = "none".to_string();
        }
    }
    repo.write_index(&local_index)?;

    // Update branch head if we are on a branch
    let head = repo.read_head()?;
    if let Some(ref_path) = head.strip_prefix("ref: ") {
        repo.write_ref(ref_path, &hash)?;
    } else {
        // Detached head, just update HEAD
        repo.write_head(&hash)?;
    }

    println!("Committed (id={}): {}", hash, message);

    Ok(())
}
