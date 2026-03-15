use base64::{Engine as _, engine::general_purpose};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

use protocol::{Commit, IndexFile};

#[derive(Serialize)]
struct PushRequest {
    message: String,
    username: String,
    updated_artifacts: HashMap<String, String>,
}

pub async fn run(message: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository (no .rig directory found)".into());
    }

    // Read local index.json
    let index_path = rig_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read local index.json: {}", e))?;
    let mut local_index: IndexFile = serde_json::from_str(&index_content)
        .map_err(|e| format!("Failed to parse local index.json: {}", e))?;

    // Determine message: prefer provided, fallback to last local commit message.
    let message = if let Some(m) = message {
        m
    } else {
        if local_index.latest_commit.is_empty() {
            "Automatic push".to_string()
        } else {
            local_index
                .commits
                .get(&local_index.latest_commit)
                .map(|c| c.message.clone())
                .unwrap_or_else(|| "Automatic push".to_string())
        }
    };

    println!("Preparing to push with message: '{}'", message);

    let server_url = local_index
        .server_url
        .as_ref()
        .ok_or("Server URL not configured")?;
    let client = reqwest::Client::new();

    let mut updated_artifacts = HashMap::new();
    let mut artifact_paths = Vec::new();

    // 1. Collect all modified (writable) artifacts
    for (artifact_id, artifact_details) in &local_index.artifacts {
        let local_path = current_dir.join(&artifact_details.path);
        if !local_path.exists() {
            continue;
        }

        let metadata = fs::metadata(&local_path)?;
        if !metadata.permissions().readonly() {
            println!("-> Found modified artifact: {}", artifact_id);
            let file_data = fs::read(&local_path)?;
            updated_artifacts.insert(
                artifact_id.clone(),
                general_purpose::STANDARD.encode(&file_data),
            );
            artifact_paths.push(local_path);
        }
    }

    if updated_artifacts.is_empty() {
        println!("No local changes (writable files) found to push.");
        return Ok(());
    }

    // 2. Send PushRequest to server
    let push_url = format!("{}/{}/push", server_url, local_index.project);
    let payload = PushRequest {
        message: message.clone(),
        username: local_index
            .username
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        updated_artifacts,
    };

    println!("-> Sending push request to server...");
    let resp = client.post(&push_url).json(&payload).send().await?;
    let status = resp.status();

    if !status.is_success() {
        let err_text = resp.text().await?;
        return Err(format!("Push failed ({}): {}", status, err_text).into());
    }

    let authoritative_commit: Commit = resp.json().await?;
    let new_hash = authoritative_commit.hash.clone();
    let new_parent = authoritative_commit.parent.clone().unwrap_or_default();

    println!("-> Server created authoritative commit: {}", new_hash);

    // 3. Update local state
    // Synchronize local commits: remove any commits between the old latest_commit and the new parent
    let mut current_to_remove = local_index.latest_commit.clone();
    while !current_to_remove.is_empty() && current_to_remove != new_parent {
        if let Some(c) = local_index.commits.remove(&current_to_remove) {
            current_to_remove = c.parent.unwrap_or_default();
        } else {
            break;
        }
    }

    // Update commits
    local_index
        .commits
        .insert(new_hash.clone(), authoritative_commit.clone());
    local_index.latest_commit = new_hash;

    // Fetch latest index from server to synchronize artifact revisions
    let remote_index_url = format!("{}/{}/index.json", server_url, local_index.project);
    let remote_resp = client.get(&remote_index_url).send().await?;
    if remote_resp.status().is_success() {
        let remote_index: IndexFile = remote_resp.json().await?;
        local_index.artifacts = remote_index.artifacts;
    }

    // 4. Set files back to read-only
    for path in artifact_paths {
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&path, perms)?;
    }

    // Persist local index
    fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;

    println!("Push completed and synchronized with server.");
    Ok(())
}
