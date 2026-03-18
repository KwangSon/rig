use crate::auth::ensure_authenticated;
use crate::repository::Repository;
use protocol::IndexFile;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig fetch...");

    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;
    let config = repo.read_config()?;

    let client = reqwest::Client::new();
    let server_url = config
        .server_url
        .as_deref()
        .unwrap_or("http://localhost:3000");
    let remote_index_url = format!("{}/api/v1/{}/index", server_url, config.project);

    let token = match ensure_authenticated(server_url).await {
        Ok(t) => t,
        Err(e) => return Err(format!("Authentication failed: {}", e).into()),
    };

    println!("-> Fetching remote metadata from {}...", remote_index_url);
    let resp = client
        .get(&remote_index_url)
        .header("authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch remote index: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Failed to fetch remote index: Server responded with status {}",
            resp.status()
        )
        .into());
    }

    let remote_index_content = resp.text().await?;
    let remote_index: IndexFile = serde_json::from_str(&remote_index_content)
        .map_err(|e| format!("Failed to parse remote index: {}", e))?;

    // Save fetched commits into local objects directory
    for (_, commit) in remote_index.commits {
        repo.write_commit(&commit)?;
    }

    // Merge remote artifacts into local index
    let mut local_index = repo.read_index()?;

    for (id, artifact) in remote_index.artifacts {
        let path = artifact.path.clone();

        if let Some(local_art) = local_index.artifacts.get_mut(&path) {
            // Update existing artifact metadata
            local_art.revision = artifact.latest;
            local_art.locked = artifact.locked_by.is_some();
            local_art.lock_owner = artifact.locked_by;
            // Note: we'd ideally have lock_generation in protocol::Artifact too
        } else {
            // Add new artifact from remote
            local_index.artifacts.insert(
                path,
                protocol::IndexArtifact {
                    artifact_id: id,
                    revision: artifact.latest,
                    local_state: "placeholder".to_string(),
                    stage: "none".to_string(),
                    locked: artifact.locked_by.is_some(),
                    lock_owner: artifact.locked_by,
                    lock_generation: None,
                    staged: None,
                    moved_from: None,
                },
            );
        }
    }

    local_index.git_modules = remote_index.git_modules;
    repo.write_index(&local_index)?;

    for (ref_name, hash) in &remote_index.refs {
        repo.write_ref(ref_name, hash)?;
    }
    if remote_index.refs.is_empty() && !remote_index.latest_commit.is_empty() {
        repo.write_ref("refs/heads/main", &remote_index.latest_commit)?;
    }

    println!("Fetch complete: local metadata updated.");
    Ok(())
}
