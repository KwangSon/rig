use crate::repository::Repository;
use protocol::Commit;
use sha1::{Digest, Sha1};
use std::collections::HashMap;

pub async fn run(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig commit with message: '{}'", message);

    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;
    let local_index = repo.read_index()?;
    let config = repo.read_config()?;

    let current_parent = repo.head_commit()?;

    // Build the artifact -> revision map for this commit.
    // Use whatever `latest` is currently in the index.
    let artifacts: HashMap<String, u32> = local_index
        .artifacts
        .iter()
        .map(|(k, v)| (k.clone(), v.latest))
        .collect();

    // Generate hash from parent, message, author, and artifacts
    let author = config
        .username
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let mut hasher = Sha1::new();
    hasher.update(
        current_parent
            .as_ref()
            .unwrap_or(&"".to_string())
            .as_bytes(),
    );
    hasher.update(message.as_bytes());
    hasher.update(author.as_bytes());
    hasher.update(serde_json::to_string(&artifacts)?.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let commit = Commit {
        hash: hash.clone(),
        parent: current_parent,
        message: message.to_string(),
        author,
        artifacts,
    };

    repo.write_commit(&commit)?;

    // Update branch head if we are on a branch
    let head = repo.read_head()?;
    if let Some(ref_path) = head.strip_prefix("ref: ") {
        repo.write_ref(ref_path, &hash)?;
    } else {
        // Detached head, just update HEAD
        repo.write_head(&hash)?;
    }

    println!("Committed (hash={}): {}", hash, message);

    Ok(())
}
