use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs;
use std::path::PathBuf;

use protocol::{Artifact, IndexFile, Revision};

#[derive(Serialize, Deserialize)]
struct AddPayload {
    path: String,
    content_base64: String,
    message: Option<String>,
}

pub async fn run(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig add for path: {:?}", path);

    // Determine project root (.rig)
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository (no .rig directory found)".into());
    }

    // Read local index
    let index_path = rig_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read local index.json: {}", e))?;
    let mut local_index: IndexFile = serde_json::from_str(&index_content)
        .map_err(|e| format!("Failed to parse local index.json: {}", e))?;

    let project = &local_index.project;
    let artifact_name = path.to_string_lossy().to_string();

    // Read local file content
    let local_path = current_dir.join(&artifact_name);
    let file_data = fs::read(&local_path)
        .map_err(|e| format!("Failed to read local file {}: {}", local_path.display(), e))?;
    let content_base64 = general_purpose::STANDARD.encode(&file_data);

    // Compute hash
    let mut hasher = Sha1::new();
    hasher.update(&file_data);
    let hash = format!("{:x}", hasher.finalize());

    // Fetch remote index
    let client = reqwest::Client::new();
    let server_url = local_index
        .server_url
        .as_ref()
        .ok_or("Server URL not configured")?;
    let remote_index_url = format!("{}/{}/index.json", server_url, project);
    println!("-> Fetching latest metadata from {}...", remote_index_url);
    let remote_index_resp = client.get(&remote_index_url).send().await?;
    if !remote_index_resp.status().is_success() {
        return Err(format!(
            "Failed to fetch remote index.json: Server responded with status {}",
            remote_index_resp.status()
        )
        .into());
    }

    let remote_index_content = remote_index_resp.text().await?;
    let remote_index: IndexFile = serde_json::from_str(&remote_index_content)
        .map_err(|e| format!("Failed to parse remote index.json: {}", e))?;

    let already_exist = remote_index.artifacts.contains_key(&artifact_name);

    // Build payload
    let payload = AddPayload {
        path: artifact_name.clone(),
        content_base64,
        message: None,
    };

    if already_exist {
        println!(
            "Artifact '{}' exists on server; ensuring lock...",
            artifact_name
        );

        // Confirm lock
        let lock_url = format!(
            "{}/{}/artifacts/{}/lock",
            server_url, project, artifact_name
        );
        let lock_resp = client.get(&lock_url).send().await?;
        if !lock_resp.status().is_success() {
            return Err(format!("Failed to query lock state: {}", lock_resp.status()).into());
        }
        let lock_json: serde_json::Value = lock_resp.json().await?;
        let locked = lock_json["locked"].as_bool().unwrap_or(false);
        let locked_by = lock_json["user"].as_str().unwrap_or("");

        let current_user = local_index.username.as_deref().unwrap_or("unknown");

        if !locked || locked_by != current_user {
            return Err(format!(
                "Artifact '{}' must be locked by you before adding (locked_by={:?}, you={:?})",
                artifact_name, locked_by, current_user
            )
            .into());
        }

        println!("-> Uploading revised artifact to server...");
        let rev_url = format!(
            "{}/{}/artifacts/{}/revisions",
            server_url, project, artifact_name
        );
        let resp = client.post(&rev_url).json(&payload).send().await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to add revision: {}", resp.status()).into());
        }

        // Update local index.json
        if let Some(artifact) = local_index.artifacts.get_mut(&artifact_name) {
            artifact.latest += 1;
            artifact.revisions.push(Revision {
                rev: artifact.latest,
                hash: hash.clone(),
            });
        }
    } else {
        println!("Artifact '{}' is new; creating on server...", artifact_name);
        let create_url = format!("{}/{}/artifacts", server_url, project);
        let resp = client.post(&create_url).json(&payload).send().await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to create artifact: {}", resp.status()).into());
        }

        local_index.artifacts.insert(
            artifact_name.clone(),
            Artifact {
                path: artifact_name.clone(),
                latest: 1,
                locked_by: None,
                revisions: vec![Revision {
                    rev: 1,
                    hash: hash.clone(),
                }],
            },
        );
    }

    // Persist local index.json
    fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;

    // Set local file to read-only after add
    let mut perms = fs::metadata(&local_path)?.permissions();
    perms.set_readonly(true);
    fs::set_permissions(&local_path, perms)?;

    println!("Add completed successfully.");
    Ok(())
}
