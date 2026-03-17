use crate::repository::{Index, Repository};
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
    pub artifact_paths: HashMap<String, String>,     // id -> path
    pub ref_name: Option<String>,
}

pub async fn run(_message_opt: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;

    let mut local_index = repo.read_index()?;
    let config = repo.read_config()?;

    let head_hash = match repo.head_commit()? {
        Some(hash) => hash,
        None => return Err("No local commits to push. Run 'rig commit' first.".into()),
    };

    let commit = repo
        .read_commit(&head_hash)?
        .ok_or("Latest commit not found locally")?;

    println!(
        "Preparing to push commit: {} ('{}')",
        commit.hash, commit.message
    );

    let server_url = config
        .server_url
        .as_ref()
        .ok_or("Server URL not configured")?;

    // Build client with a longer timeout for large file uploads
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    // 1. Prepare Metadata and artifacts info
    let mut artifact_compression = HashMap::new();
    let mut artifact_paths_map = HashMap::new();
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
            artifact_paths_map.insert(artifact_id.clone(), artifact_details.path.clone());
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
    let head_ref = repo
        .read_head()?
        .strip_prefix("ref: ")
        .map(|s| s.to_string());

    let push_metadata = PushMetadata {
        commit: commit.clone(),
        artifact_compression,
        artifact_paths: artifact_paths_map,
        ref_name: head_ref,
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
    let push_url = format!("{}/api/v1/{}/push", server_url, config.project);
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

    // 4. Synchronize local state
    let remote_index_url = format!("{}/api/v1/{}/index.json", server_url, config.project);
    let remote_resp = client.get(&remote_index_url).send().await?;
    if remote_resp.status().is_success() {
        let remote_index: IndexFile = remote_resp.json().await?;

        local_index.artifacts = remote_index.artifacts;
        local_index.git_modules = remote_index.git_modules;

        repo.write_index(&local_index)?;

        for (_, c) in remote_index.commits {
            repo.write_commit(&c)?;
        }

        if !remote_index.latest_commit.is_empty() {
            // Wait, we need to sync remote refs!
            for (ref_name, hash) in &remote_index.refs {
                repo.write_ref(ref_name, hash)?;
            }
            if remote_index.refs.is_empty() {
                // Fallback for legacy pushed projects
                repo.write_ref("refs/heads/main", &remote_index.latest_commit)?;
            }
        }
    }

    // 5. Set files back to read-only
    for path in artifact_paths {
        if let Ok(mut perms) = fs::metadata(&path).map(|m| m.permissions()) {
            perms.set_readonly(true);
            let _ = fs::set_permissions(&path, perms);
        }
    }
    Ok(())
}
