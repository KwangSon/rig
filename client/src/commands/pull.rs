use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use protocol::{Artifact, Commit, IndexFile, Revision};

pub async fn run(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig pull for path: {:?}", path);

    // Determine the current project root (where .rig is located)
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");

    if !rig_dir.exists() || !rig_dir.is_dir() {
        return Err(
            "Not a rig repository (or not in a rig repository). The .rig directory was not found."
                .into(),
        );
    }

    // 1. Read local .rig/index.json
    let local_index_path = rig_dir.join("index.json");
    let local_index_content = fs::read_to_string(&local_index_path)
        .map_err(|e| format!("Failed to read local index.json: {}", e))?;
    let local_index: IndexFile = serde_json::from_str(&local_index_content)
        .map_err(|e| format!("Failed to parse local index.json: {}", e))?;

    println!("Project: {}", local_index.project);

    // 2. Fetch latest index.json from the server
    let client = reqwest::Client::new();
    let server_url = "http://localhost:3000"; // Assuming local server
    let remote_index_url = format!("{}/{}/index.json", server_url, local_index.project);

    println!("-> Fetching latest metadata from {}...", remote_index_url);
    let remote_resp = client
        .get(&remote_index_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch remote index.json: {}", e))?;

    if !remote_resp.status().is_success() {
        return Err(format!(
            "Failed to fetch remote index.json: Server responded with status {}",
            remote_resp.status()
        )
        .into());
    }
    let remote_index_content = remote_resp.text().await?;
    let remote_index: IndexFile = serde_json::from_str(&remote_index_content)
        .map_err(|e| format!("Failed to parse remote index.json: {}", e))?;
    println!("   Remote metadata fetched successfully.");

    // 3. Determine which artifacts to pull
    fn resolve_artifact_id(index: &IndexFile, query: &str) -> Option<String> {
        if index.artifacts.contains_key(query) {
            return Some(query.to_string());
        }
        index
            .artifacts
            .iter()
            .find(|(_, details)| details.path == query)
            .map(|(id, _)| id.clone())
    }

    let artifacts_to_pull: Vec<String> = if path == PathBuf::from("*") {
        remote_index.artifacts.keys().cloned().collect()
    } else {
        let path_str = path.to_string_lossy().to_string();
        if let Some(id) = resolve_artifact_id(&remote_index, &path_str) {
            vec![id]
        } else {
            return Err(format!("Artifact '{}' not found on server", path_str).into());
        }
    };

    // 4. Pull each artifact
    for artifact_name in artifacts_to_pull {
        let artifact_details = &remote_index.artifacts[&artifact_name];
        let latest_rev = artifact_details.latest;
        println!("-> Pulling {} (rev {})", artifact_name, latest_rev);

        // Download URL: assuming /project/artifacts/artifact_name/rev{rev}.blend
        let download_url = format!(
            "{}/{}/artifacts/{}/rev{}.blend",
            server_url, local_index.project, artifact_name, latest_rev
        );
        println!("   Downloading from {}", download_url);

        let file_resp = client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| format!("Failed to download {}: {}", download_url, e))?;

        if !file_resp.status().is_success() {
            return Err(format!(
                "Failed to download {}: Server responded with status {}",
                download_url,
                file_resp.status()
            )
            .into());
        }

        let file_content = file_resp.bytes().await?;

        // Save to workspace root with path name
        let local_path = current_dir.join(&artifact_details.path);

        // Ensure parent directories exist (for nested paths)
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        // If file exists, make it writable first
        if local_path.exists() {
            let mut perms = fs::metadata(&local_path)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            fs::set_permissions(&local_path, perms)?;
        }

        fs::write(&local_path, &file_content)
            .map_err(|e| format!("Failed to write file {}: {}", local_path.display(), e))?;

        // Set to read-only
        let mut perms = fs::metadata(&local_path)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&local_path, perms)?;

        println!("   Saved {} as read-only", local_path.display());
    }

    println!("Pull completed successfully.");

    Ok(())
}
