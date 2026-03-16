use std::fs;
use std::path::PathBuf;

use protocol::IndexFile;

pub async fn run(path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig log...");

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

    if local_index.latest_commit.is_empty() {
        println!("No commits yet.");
        return Ok(());
    }

    // Resolve artifact ID if path is provided
    let artifact_id = if let Some(p) = &path {
        let path_str = p.to_string_lossy().to_string();
        if local_index.artifacts.contains_key(&path_str) {
            Some(path_str)
        } else {
            local_index
                .artifacts
                .iter()
                .find(|(_, details)| details.path == path_str)
                .map(|(id, _)| id.clone())
        }
    } else {
        None
    };

    if path.is_some() && artifact_id.is_none() {
        return Err(format!("Artifact '{}' not found", path.unwrap().display()).into());
    }

    if let Some(ref id) = artifact_id {
        println!("History for artifact: {}", id);
        println!(
            "{:<40} {:<10} {:<30} {:<15}",
            "HASH", "REV", "MESSAGE", "AUTHOR"
        );
        println!("{}", "-".repeat(95));
    }

    let mut current_hash = Some(local_index.latest_commit.clone());
    while let Some(hash) = current_hash {
        if let Some(commit) = local_index.commits.get(&hash) {
            if let Some(ref id) = artifact_id {
                // Filter by artifact
                if let Some(rev) = commit.artifacts.get(id) {
                    println!(
                        "{:<40} {:<10} {:<30} {:<15}",
                        commit.hash, rev, commit.message, commit.author
                    );
                }
            } else {
                println!("{} {} - {}", commit.hash, commit.message, commit.author);
            }
            current_hash = commit.parent.clone();
        } else {
            eprintln!("Error: Commit {} not found in index.", hash);
            break;
        }
    }

    Ok(())
}
