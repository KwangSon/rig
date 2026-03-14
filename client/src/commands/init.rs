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

    // Try to create the project on the server first
    let client = reqwest::Client::new();
    let create_url = format!("{}/create_project", server_url);
    let create_payload = serde_json::json!({
        "name": project_name
    });
    println!("Creating project '{}' on server...", project_name);
    let resp = client.post(&create_url).json(&create_payload).send().await;
    match resp {
        Ok(r) if r.status().is_success() => {
            println!("Project created on server successfully.");
        }
        Ok(r) if r.status() == reqwest::StatusCode::CONFLICT => {
            println!("Project already exists on server.");
        }
        Ok(r) => {
            return Err(
                format!("Failed to create project on server. Status: {}", r.status()).into(),
            );
        }
        Err(e) => {
            return Err(format!("Could not connect to server to create project: {}", e).into());
        }
    }

    // Now fetch index.json from server and save to .rig/index.json
    fs::create_dir_all(&rig_dir)?;
    let remote_index_url = format!("{}/{}/index.json", server_url, project_name);
    println!("Fetching index.json from server: {}", remote_index_url);
    let remote_resp = client.get(&remote_index_url).send().await;
    let remote_index_content = match remote_resp {
        Ok(r) if r.status().is_success() => r
            .text()
            .await
            .map_err(|e| format!("Failed to read server index.json: {}", e))?,
        Ok(r) => {
            return Err(format!(
                "Failed to fetch index.json from server. Status: {}",
                r.status()
            )
            .into());
        }
        Err(e) => {
            return Err(format!("Could not connect to server to fetch index.json: {}", e).into());
        }
    };
    // Validate and write
    let _: IndexFile = serde_json::from_str(&remote_index_content)
        .map_err(|e| format!("Server index.json is invalid: {}", e))?;
    let index_path = rig_dir.join("index.json");
    fs::write(&index_path, &remote_index_content)?;
    println!("Created .rig directory and downloaded index.json from server");
    Ok(())
}
