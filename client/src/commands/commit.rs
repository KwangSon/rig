use std::collections::HashMap;
use std::fs;

use protocol::{Commit, IndexFile};
use serde_json;
use sha1::{Digest, Sha1};

pub async fn run(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig commit with message: '{}'", message);

    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository (no .rig directory found)".into());
    }

    let index_path = rig_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read local index.json: {}", e))?;
    let mut local_index: IndexFile = serde_json::from_str(&index_content)
        .map_err(|e| format!("Failed to parse local index.json: {}", e))?;

    let current_parent = if local_index.latest_commit.is_empty() {
        None
    } else {
        Some(local_index.latest_commit.clone())
    };

    // Build the artifact -> revision map for this commit.
    // Use whatever `latest` is currently in the index.
    let artifacts: HashMap<String, u32> = local_index
        .artifacts
        .iter()
        .map(|(k, v)| (k.clone(), v.latest))
        .collect();

    // Generate hash from parent, message, and artifacts
    let mut hasher = Sha1::new();
    hasher.update(
        current_parent
            .as_ref()
            .unwrap_or(&"".to_string())
            .as_bytes(),
    );
    hasher.update(message.as_bytes());
    hasher.update(serde_json::to_string(&artifacts)?.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let commit = Commit {
        hash: hash.clone(),
        parent: current_parent,
        message: message.to_string(),
        artifacts,
    };

    local_index.commits.insert(hash.clone(), commit);
    local_index.latest_commit = hash.clone();

    fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;

    println!("Committed (hash={}): {}", hash, message);

    Ok(())
}
