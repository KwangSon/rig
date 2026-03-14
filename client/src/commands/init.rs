use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

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

    // Prompt for server URL
    print!("Enter server URL (default: http://localhost:3000): ");
    io::stdout().flush()?;
    let mut server_url = String::new();
    io::stdin().read_line(&mut server_url)?;
    let server_url = server_url.trim();
    let server_url = if server_url.is_empty() {
        "http://localhost:3000".to_string()
    } else {
        server_url.to_string()
    };

    let initial_index = IndexFile {
        project: project_name.to_string(),
        server_url,
        latest_commit: 0,
        artifacts: HashMap::new(),
        commits: Vec::new(),
    };

    let index_path = rig_dir.join("index.json");
    fs::write(&index_path, serde_json::to_string_pretty(&initial_index)?)?;

    println!("Created .rig directory and initialized index.json");

    Ok(())
}
