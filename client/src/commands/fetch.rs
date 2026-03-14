use std::fs;

use crate::commands::status::IndexFile;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig fetch...");

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

    let client = reqwest::Client::new();
    let server_url = "http://localhost:3000";
    let remote_index_url = format!("{}/{}/index.json", server_url, local_index.project);

    println!("-> Fetching remote metadata from {}...", remote_index_url);
    let resp = client
        .get(&remote_index_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch remote index.json: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Failed to fetch remote index.json: Server responded with status {}",
            resp.status()
        )
        .into());
    }

    let remote_index_content = resp.text().await?;
    // Validate it can be parsed before writing it out
    let _remote_index: IndexFile = serde_json::from_str(&remote_index_content)
        .map_err(|e| format!("Failed to parse remote index.json: {}", e))?;

    fs::write(&index_path, remote_index_content)?;

    println!("Fetch complete: local metadata updated.");
    Ok(())
}
