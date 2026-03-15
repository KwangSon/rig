use protocol::IndexFile;
use std::fs;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig status...");

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
    let server_url = "http://localhost:3000"; // Assuming local server for now
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
    let remote_index: IndexFile = match serde_json::from_str(&remote_index_content) {
        Ok(idx) => idx,
        Err(e) => {
            return Err(format!(
                "Failed to parse remote index.json: {}\n\
This project does not exist on the server. You must run the 'init' command in an empty folder and ensure the project is successfully created on the server before proceeding.",
                e
            ).into());
        }
    };
    println!("   Remote metadata fetched successfully.");

    // 3. Compare local and server state
    let mut local_files: Vec<String> = Vec::new();
    let mut missing_files: Vec<String> = Vec::new();
    let mut outdated_files: Vec<(String, u32, u32)> = Vec::new(); // (artifact_name, local_rev, server_rev)

    for (artifact_name, remote_artifact_details) in &remote_index.artifacts {
        let artifact_path = current_dir.join(&remote_artifact_details.path);
        if artifact_path.exists() {
            // File exists locally
            local_files.push(artifact_name.clone());

            // Check if outdated
            if let Some(local_artifact_details) = local_index.artifacts.get(artifact_name) {
                // Here, we're comparing based on the `latest` field in the index.json.
                // In a real scenario, we might want to compare hashes of the actual files
                // or specific revision information. For this task, `latest` comparison is sufficient.
                if local_artifact_details.latest < remote_artifact_details.latest {
                    outdated_files.push((
                        artifact_name.clone(),
                        local_artifact_details.latest,
                        remote_artifact_details.latest,
                    ));
                }
            } else {
                // This case implies a local file exists but isn't in local_index.json,
                // which shouldn't happen if the local_index.json accurately reflects the cloned state.
                // For now, we'll assume local_index.json is authoritative for local knowledge.
            }
        } else {
            // File does not exist locally, but is on server
            missing_files.push(artifact_name.clone());
        }
    }

    // Output the results
    println!("\nLocal files:");
    if local_files.is_empty() {
        println!("  (none)");
    } else {
        for file in local_files {
            let rev = local_index.artifacts.get(&file).map_or(0, |a| a.latest);
            println!("  {} (rev {})", file, rev);
        }
    }

    println!("\nMissing files:");
    if missing_files.is_empty() {
        println!("  (none)");
    } else {
        for file in missing_files {
            let rev = remote_index.artifacts.get(&file).map_or(0, |a| a.latest);
            println!("  {} (rev {})", file, rev);
        }
    }

    println!("\nOutdated files:");
    if outdated_files.is_empty() {
        println!("  (none)");
    } else {
        for (file, local_rev, server_rev) in outdated_files {
            println!(
                "  {} (local: rev{}, server: rev{})",
                file, local_rev, server_rev
            );
        }
    }

    Ok(())
}
