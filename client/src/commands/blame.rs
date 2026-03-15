use std::fs;
use std::path::PathBuf;

use protocol::IndexFile;

pub async fn run(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository (no .rig directory found)".into());
    }

    let index_path = rig_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read local index.json: {}", e))?;
    let local_index: IndexFile = serde_json::from_str(&index_content)
        .map_err(|e| format!("Failed to parse local index.json: {}", e))?;

    let path_str = path.to_string_lossy().to_string();

    // Resolve artifact ID
    let artifact_id = if local_index.artifacts.contains_key(&path_str) {
        Some(path_str.clone())
    } else {
        local_index
            .artifacts
            .iter()
            .find(|(_, details)| details.path == path_str)
            .map(|(id, _)| id.clone())
    }
    .ok_or_else(|| format!("Artifact '{}' not found", path_str))?;

    let artifact = &local_index.artifacts[&artifact_id];
    println!("Blame for artifact: {} ({})", artifact_id, artifact.path);
    println!("{:<5} {:<40} {:<30}", "REV", "HASH", "COMMIT MESSAGE");
    println!("{}", "-".repeat(80));

    for rev in &artifact.revisions {
        // Find the commit that introduced this revision
        let mut commit_msg = "Unknown".to_string();
        for commit in local_index.commits.values() {
            if let Some(&r) = commit.artifacts.get(&artifact_id)
                && r == rev.rev
            {
                commit_msg = commit.message.clone();
                // We take the first one we find or we can look for the "oldest" commit?
                // Usually there's only one "first" commit that introduces a specific revision.
                break;
            }
        }

        println!("{:<5} {:<40} {:<30}", rev.rev, rev.hash, commit_msg);
    }

    Ok(())
}
