use base64::{Engine as _, engine::general_purpose};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

use protocol::{Commit, IndexFile};

#[derive(Serialize)]
struct PushRequest {
    pub commit: Commit,
    pub updated_artifacts: HashMap<String, String>,
}

pub async fn run(_message_opt: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
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

    if local_index.latest_commit.is_empty() {
        return Err("No local commits to push. Run 'rig commit' first.".into());
    }

    let commit = local_index
        .commits
        .get(&local_index.latest_commit)
        .ok_or("Latest commit not found locally")?
        .clone();

    println!(
        "Preparing to push commit: {} ('{}')",
        commit.hash, commit.message
    );

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

    // 2. Send PushRequest to server
    let push_url = format!("{}/api/v1/{}/push", server_url, local_index.project);
    let payload = PushRequest {
        commit,
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

    println!("-> Server accepted authoritative commit: {}", new_hash);

    // 3. Synchronize local state with server
    // Fetch latest index from server to synchronize artifact revisions
    let remote_index_url = format!("{}/api/v1/{}/index.json", server_url, local_index.project);
    let remote_resp = client.get(&remote_index_url).send().await?;
    if remote_resp.status().is_success() {
        let remote_index: IndexFile = remote_resp.json().await?;
        local_index.artifacts = remote_index.artifacts;
        local_index.commits = remote_index.commits;
        local_index.latest_commit = remote_index.latest_commit;
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
