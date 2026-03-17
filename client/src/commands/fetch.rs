use crate::repository::{Index, Repository};
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
    let remote_index_url = format!("{}/api/v1/{}/index.json", server_url, config.project);

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
    let remote_index: IndexFile = serde_json::from_str(&remote_index_content)
        .map_err(|e| format!("Failed to parse remote index.json: {}", e))?;

    // Save fetched commits into local objects directory
    for (_, commit) in remote_index.commits {
        repo.write_commit(&commit)?;
    }

    // Update refs/remote/origin/main (conceptually, we just update index for now)
    let mut local_index = repo.read_index()?;
    local_index.artifacts = remote_index.artifacts;
    local_index.git_modules = remote_index.git_modules;
    repo.write_index(&local_index)?;

    if !remote_index.latest_commit.is_empty() {
        repo.write_ref("refs/heads/main", &remote_index.latest_commit)?;
    }

    println!("Fetch complete: local metadata updated.");
    Ok(())
}
