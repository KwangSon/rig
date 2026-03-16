use flate2::read::GzDecoder;
use protocol::IndexFile;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

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
    let server_url = local_index
        .server_url
        .as_deref()
        .unwrap_or("http://localhost:3000");
    let remote_index_url = format!("{}/api/v1/{}/index.json", server_url, local_index.project);

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

        // Find compression info from revisions
        let is_compressed = artifact_details
            .revisions
            .iter()
            .find(|r| r.rev == latest_rev)
            .map(|r| r.compressed)
            .unwrap_or(false);

        println!(
            "-> Pulling {} (rev {}, compressed={})",
            artifact_name, latest_rev, is_compressed
        );

        // Determine filename based on extension
        let ext = std::path::Path::new(&artifact_name)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{}", s))
            .unwrap_or_default();
        let filename = format!("rev{}{}", latest_rev, ext);

        // Download URL: /{project}/artifacts/{artifact_id}/{filename}
        let download_url = format!(
            "{}/api/v1/{}/artifacts/{}/{}",
            server_url, local_index.project, artifact_name, filename
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

        let mut file_content = file_resp.bytes().await?.to_vec();

        // Decompress if needed
        if is_compressed {
            print!("   Decompressing... ");
            let mut decoder = GzDecoder::new(&file_content[..]);
            let mut decoded_data = Vec::new();
            decoder.read_to_end(&mut decoded_data)?;
            file_content = decoded_data;
            println!("done.");
        }

        // Save to workspace root with path name
        let local_path = current_dir.join(&artifact_details.path);

        // Ensure parent directories exist
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        // If file exists, make it writable first
        if local_path.exists() {
            let _ = fs::metadata(&local_path).map(|m| {
                let mut perms = m.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = fs::set_permissions(&local_path, perms);
            });
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
