use std::fs;

use protocol::IndexFile;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut current_hash = Some(local_index.latest_commit.clone());
    while let Some(hash) = current_hash {
        if let Some(commit) = local_index.commits.get(&hash) {
            println!("{} {} - {}", commit.hash, commit.message, commit.author);
            current_hash = commit.parent.clone();
        } else {
            eprintln!("Error: Commit {} not found in index.", hash);
            break;
        }
    }

    Ok(())
}
