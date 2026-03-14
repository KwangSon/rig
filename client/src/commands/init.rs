use std::collections::HashMap;
use std::fs;

use crate::commands::status::IndexFile;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing rig repository...");

    let current_dir = std::env::current_dir()?;
    let project_name = current_dir
        .file_name()
        .and_then(|os| os.to_str())
        .ok_or("Could not determine project name")?;

    let rig_dir = current_dir.join(".rig");
    if rig_dir.exists() {
        return Err("A .rig directory already exists in this location".into());
    }

    fs::create_dir_all(&rig_dir)?;

    let initial_index = IndexFile {
        project: project_name.to_string(),
        latest_commit: 0,
        artifacts: HashMap::new(),
        commits: Vec::new(),
    };

    let index_path = rig_dir.join("index.json");
    fs::write(&index_path, serde_json::to_string_pretty(&initial_index)?)?;

    println!("Created .rig directory and initialized index.json");

    Ok(())
}
