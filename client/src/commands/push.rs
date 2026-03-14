use base64::{Engine as _, engine::general_purpose};
use serde::Serialize;
use std::fs;

use crate::commands::status::{IndexFile, Revision};

#[derive(Serialize)]
struct PushPayload {
    content_base64: String,
    message: Option<String>,
}

pub async fn run(message: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Determine message: prefer provided, fallback to last local commit message.
    let message = if let Some(m) = message {
        m
    } else {
        // Read local index to find last commit message
        let current_dir = std::env::current_dir()?;
        let rig_dir = current_dir.join(".rig");
        let index_path = rig_dir.join("index.json");
        let index_content = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("Failed to read local index.json: {}", e))?;
        let local_index: super::status::IndexFile = serde_json::from_str(&index_content)
            .map_err(|e| format!("Failed to parse local index.json: {}", e))?;
        local_index
            .commits
            .last()
            .map(|c| c.message.clone())
            .unwrap_or_default()
    };

    println!("Running rig push with message: '{}'", message);

    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository (no .rig directory found)".into());
    }

    // Read local index.json
    let index_path = rig_dir.join("index.json");
    let local_index_content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read local index.json: {}", e))?;
    let mut local_index: IndexFile = serde_json::from_str(&local_index_content)
        .map_err(|e| format!("Failed to parse local index.json: {}", e))?;

    let server_url = "http://localhost:3000";
    let client = reqwest::Client::new();

    let mut pushed_any = false;

    for (artifact_name, artifact_details) in local_index.artifacts.iter_mut() {
        let local_path = current_dir.join(&artifact_details.path);
        if !local_path.exists() {
            continue;
        }

        let metadata = fs::metadata(&local_path)?;
        if metadata.permissions().readonly() {
            continue; // not modified/locked by us
        }

        pushed_any = true;
        println!("-> Pushing artifact '{}'...", artifact_name);

        let file_data = fs::read(&local_path)
            .map_err(|e| format!("Failed to read file {}: {}", local_path.display(), e))?;
        let payload = PushPayload {
            content_base64: general_purpose::STANDARD.encode(&file_data),
            message: if message.is_empty() {
                None
            } else {
                Some(message.to_string())
            },
        };

        // Push new revision
        let rev_url = format!(
            "{}/{}/artifacts/{}/revisions",
            server_url, local_index.project, artifact_name
        );
        let resp = client.post(&rev_url).json(&payload).send().await?;
        if !resp.status().is_success() {
            return Err(format!("Failed to push revision: {}", resp.status()).into());
        }
        let resp_json: serde_json::Value = resp.json().await?;
        let new_rev = resp_json["revision"].as_u64().unwrap_or(0) as u32;

        // Update local index
        artifact_details.latest = new_rev;
        artifact_details.revisions.push(Revision {
            rev: new_rev,
            hash: "".to_string(),
        });

        // Unlock on server
        let unlock_url = format!(
            "{}/{}/artifacts/{}/lock",
            server_url, local_index.project, artifact_name
        );
        let unlock_resp = client.delete(&unlock_url).send().await?;
        if !unlock_resp.status().is_success() {
            return Err(format!("Failed to unlock after push: {}", unlock_resp.status()).into());
        }

        // Set local file to read-only again
        let mut perms = fs::metadata(&local_path)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&local_path, perms)?;

        println!("   pushed rev {} and unlocked", new_rev);
    }

    if !pushed_any {
        println!("No locked (writable) artifacts found to push.");
        return Ok(());
    }

    // Persist local index
    fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;

    println!("Push completed.");
    Ok(())
}
