use flate2::Compression;
use flate2::write::GzEncoder;
use reqwest::multipart;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

use protocol::{Commit, IndexFile};

#[derive(Serialize)]
struct PushMetadata {
    pub commit: Commit,
    pub artifact_compression: HashMap<String, bool>, // id -> is_compressed
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

    // Build client with a longer timeout for large file uploads
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    // 1. Prepare Metadata and artifacts info
    let mut artifact_compression = HashMap::new();
    let mut artifact_data = Vec::new();
    let mut artifact_paths = Vec::new();

    // Compression threshold (1MB)
    const COMPRESSION_THRESHOLD: usize = 1024 * 1024;

    for (artifact_id, artifact_details) in &local_index.artifacts {
        let local_path = current_dir.join(&artifact_details.path);
        if !local_path.exists() {
            continue;
        }

        let metadata = fs::metadata(&local_path)?;
        if !metadata.permissions().readonly() {
            println!("-> Processing artifact for upload: {}", artifact_id);
            let mut file_data = fs::read(&local_path)?;
            let mut is_compressed = false;

            // Compress if large enough
            if file_data.len() > COMPRESSION_THRESHOLD {
                print!(
                    "   Compressing {} ({} bytes)... ",
                    artifact_id,
                    file_data.len()
                );
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&file_data)?;
                let compressed_data = encoder.finish()?;

                if compressed_data.len() < file_data.len() {
                    println!("done. ({} bytes)", compressed_data.len());
                    file_data = compressed_data;
                    is_compressed = true;
                } else {
                    println!("skipped (compression didn't reduce size).");
                }
            }

            artifact_compression.insert(artifact_id.clone(), is_compressed);
            artifact_data.push((artifact_id.clone(), file_data));
            artifact_paths.push(local_path);
        }
    }

    if artifact_data.is_empty() {
        println!("No local changes found to push.");
        return Ok(());
    }

    // 2. Build Multipart Form (Metadata FIRST)
    let mut form = multipart::Form::new();

    // Add metadata part
    let push_metadata = PushMetadata {
        commit: commit.clone(),
        artifact_compression,
    };
    form = form.part(
        "metadata",
        multipart::Part::text(serde_json::to_string(&push_metadata)?),
    );

    // Add file parts
    for (id, data) in artifact_data {
        let part = multipart::Part::bytes(data).file_name(id.clone());
        form = form.part(format!("file_{}", id), part);
    }

    // 3. Send PushRequest to server
    let push_url = format!("{}/api/v1/{}/push", server_url, local_index.project);
    println!("-> Sending push request (multipart) to server...");

    let resp = client.post(&push_url).multipart(form).send().await?;
    let status = resp.status();

    if !status.is_success() {
        let err_text = resp.text().await?;
        return Err(format!("Push failed ({}): {}", status, err_text).into());
    }

    let authoritative_commit: Commit = resp.json().await?;
    println!(
        "-> Server accepted authoritative commit: {}",
        authoritative_commit.hash
    );

    // 3. Synchronize local state
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
        if let Ok(mut perms) = fs::metadata(&path).map(|m| m.permissions()) {
            perms.set_readonly(true);
            let _ = fs::set_permissions(&path, perms);
        }
    }

    // Persist local index
    fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;

    println!("Push completed and synchronized with server.");
    Ok(())
}
