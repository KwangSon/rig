use crate::auth::ensure_authenticated;
use crate::repository::Repository;
use flate2::Compression;
use flate2::write::GzEncoder;
use reqwest::multipart;
use serde::Serialize;
use sha1::Digest;
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
    let commit = if let Some(commit_hash) = repo.head_commit()? {
        repo.read_commit(&commit_hash)?
            .ok_or("Latest commit not found locally")?
    } else {
        return Err("No local commits to push. Run 'rig commit' first.".into());
    };

    let config = repo.read_config()?;

    println!(
        "Preparing to push commit: {} ('{}')",
        commit.id, commit.message
    );

    let server_url = config
        .server_url
        .as_ref()
        .ok_or("Server URL not configured")?;

    // Build client with a longer timeout for large file uploads
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let token = match ensure_authenticated(server_url).await {
        Ok(t) => t,
        Err(e) => return Err(format!("Authentication failed: {}", e).into()),
    };

    // 1. Prepare Metadata and artifacts info
    let mut artifact_compression = HashMap::new();
    let mut artifact_paths_map = HashMap::new();
    let mut artifact_data = Vec::new();
    let mut artifact_paths = Vec::new();

    // Compression threshold (1MB)
    const COMPRESSION_THRESHOLD: usize = 1024 * 1024;

    for commit_artifact in &commit.artifacts {
        let path = &commit_artifact.path;
        let local_path = current_dir.join(path);
        if !local_path.exists() {
            return Err(
                format!("ERROR: Committed file not found: {}", local_path.display()).into(),
            );
        }

        // --- Stage 3 — rig push pre-flight ---
        let artifact_details = local_index
            .artifacts
            .get(path)
            .ok_or_else(|| format!("Artifact '{}' not found in index", path))?;

        let metadata = fs::metadata(&local_path)?;
        let current_size = metadata.len();
        let current_mtime = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let staged = artifact_details
            .staged
            .as_ref()
            .ok_or_else(|| format!("Artifact '{}' was not staged via 'rig add'", path))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let needs_recompute = current_size != staged.size
            || current_mtime != staged.mtime
            || (now.saturating_sub(staged.mtime)) < 1;

        let hash_to_check = if needs_recompute {
            let data = fs::read(&local_path)?;
            let mut hasher = sha1::Sha1::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        } else {
            commit_artifact.hash.clone()
        };

        if hash_to_check != commit_artifact.hash {
            return Err(format!(
                "ERROR: File '{}' changed after commit. Re-run 'rig add' and 'rig commit'.",
                path
            )
            .into());
        }
        // -------------------------------------

        println!(
            "-> Processing artifact for upload: {}",
            commit_artifact.artifact_id
        );
        let mut file_data = fs::read(&local_path)?;
        let mut is_compressed = false;

        // Compress if large enough
        if file_data.len() > COMPRESSION_THRESHOLD {
            print!(
                "   Compressing {} ({} bytes)... ",
                commit_artifact.artifact_id,
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

        artifact_compression.insert(commit_artifact.artifact_id.clone(), is_compressed);
        artifact_paths_map.insert(commit_artifact.artifact_id.clone(), path.clone());
        artifact_data.push((commit_artifact.artifact_id.clone(), file_data));
        artifact_paths.push(local_path);
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

    let resp = client
        .post(&push_url)
        .header("authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await?;
    let status = resp.status();

    if !status.is_success() {
        let err_text = resp.text().await?;
        return Err(format!("Push failed ({}): {}", status, err_text).into());
    }

    let authoritative_commit: Commit = resp.json().await?;
    println!(
        "-> Server accepted authoritative commit: {}",
        authoritative_commit.id
    );

    // 4. Synchronize local state
    let remote_index_url = format!("{}/api/v1/{}/index", server_url, config.project);
    let remote_resp = client
        .get(&remote_index_url)
        .header("authorization", format!("Bearer {}", token))
        .send()
        .await?;
    if remote_resp.status().is_success() {
        let remote_index: IndexFile = remote_resp.json().await?;

        // Transform remote artifacts to new local index structure
        let mut new_artifacts = std::collections::HashMap::new();
        for (id, remote_art) in remote_index.artifacts {
            let path = remote_art.path.clone();

            // Try to preserve some local state if it existed
            let (local_state, locked, lock_owner) =
                if let Some(la) = local_index.artifacts.get(&path) {
                    (la.local_state.clone(), la.locked, la.lock_owner.clone())
                } else {
                    ("placeholder".to_string(), false, None)
                };

            new_artifacts.insert(
                path,
                protocol::IndexArtifact {
                    artifact_id: id,
                    revision: remote_art.latest,
                    local_state,
                    stage: "none".to_string(), // Reset stage
                    locked,
                    lock_owner,
                    lock_generation: None,
                    staged: None, // Reset staged info (spec Stage 3)
                    moved_from: None,
                },
            );
        }

        local_index.artifacts = new_artifacts;
        local_index.git_modules = remote_index.git_modules;
        local_index.head = None; // Reset unpushed head (spec)

        repo.write_index(&local_index)?;

        for (_, c) in remote_index.commits {
            repo.write_commit(&c)?;
        }

        if !remote_index.latest_commit.is_empty() {
            for (ref_name, hash) in &remote_index.refs {
                repo.write_ref(ref_name, hash)?;
            }
            if remote_index.refs.is_empty() {
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
