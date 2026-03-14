use std::fs;

use crate::commands::status::IndexFile;

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

    for commit in local_index.commits {
        println!("{} {}", commit.id, commit.message);
    }

    Ok(())
}
