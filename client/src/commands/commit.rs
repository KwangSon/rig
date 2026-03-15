use std::collections::HashMap;
use std::fs;

use protocol::{Commit, IndexFile};

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

    let next_commit_id = local_index.latest_commit + 1;
    local_index.latest_commit = next_commit_id;

    // Build the artifact -> revision map for this commit.
    // Use whatever `latest` is currently in the index.
    let artifacts: HashMap<String, u32> = local_index
        .artifacts
        .iter()
        .map(|(k, v)| (k.clone(), v.latest))
        .collect();

    local_index.commits.push(Commit {
        id: next_commit_id,
        artifacts,
    });

    fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;

    println!("Committed (id={}): {}", next_commit_id, message);

    Ok(())
}
