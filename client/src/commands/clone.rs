use protocol::IndexFile;
use std::fs;
use std::path::PathBuf;

pub async fn run(
    url: &str,
    path: &Option<PathBuf>,
    provided_username: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed_url = url.trim_end_matches('/');
    let project_name = trimmed_url
        .rsplit('/')
        .next()
        .ok_or("Could not determine project name from URL")?;

    let base_url = if let Some(pos) = trimmed_url.rfind('/') {
        &trimmed_url[..pos]
    } else {
        return Err("Invalid URL format. Expected http://<server>/<project>".into());
    };

    println!(
        "Cloning project '{}' from server '{}'",
        project_name, base_url
    );

    let client = reqwest::Client::new();

    // 1. Check if server is alive
    let health_url = format!("{}/api/v1/health", base_url);
    println!("-> Checking server status at {}...", health_url);
    let health_resp = client
        .get(&health_url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to server at {}: {}", health_url, e))?;

    if !health_resp.status().is_success() {
        return Err(format!(
            "Server status check failed: Server responded with status {}",
            health_resp.status()
        )
        .into());
    }
    println!("   Server is alive.");

    // 2. Fetch metadata from the server
    let metadata_url = format!("{}/api/v1/{}/index.json", base_url, project_name);
    println!("-> Fetching metadata from {}...", metadata_url);
    let meta_resp = client
        .get(&metadata_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch metadata from {}: {}", metadata_url, e))?;

    if !meta_resp.status().is_success() {
        return Err(format!(
            "Failed to fetch metadata: Server responded with status {}",
            meta_resp.status()
        )
        .into());
    }
    let metadata = meta_resp.text().await?;
    println!("   Metadata fetched successfully.");

    let mut index: IndexFile =
        serde_json::from_str(&metadata).map_err(|e| format!("Failed to parse metadata: {}", e))?;

    // 2.5 Resolve username (interactive only if not provided)
    let username = if let Some(u) = provided_username {
        u.clone()
    } else {
        use std::io::{self, Write};
        let default_username =
            crate::utils::get_git_user_info().unwrap_or_else(|| "unknown".to_string());
        print!("Enter your username (default: {}): ", default_username);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            default_username
        } else {
            trimmed.to_string()
        }
    };
    index.username = Some(username);

    // 3. Create .rig folder and write index.json
    let clone_path = match path {
        Some(p) => p.clone(),
        None => PathBuf::from(project_name),
    };
    println!("-> Cloning into {:?}...", clone_path.display());

    if clone_path.exists() && clone_path.read_dir()?.next().is_some() {
        return Err(format!(
            "Destination path '{:?}' already exists and is not an empty directory.",
            clone_path.display()
        )
        .into());
    }

    let rig_path = clone_path.join(".rig");
    fs::create_dir_all(&rig_path)?;

    let index_path = rig_path.join("index.json");
    fs::write(&index_path, serde_json::to_string_pretty(&index)?)?;

    // 4. Create empty read-only files for each artifact
    for artifact in index.artifacts.values() {
        let file_path = clone_path.join(&artifact.path);
        println!("-> Creating placeholder for {}", artifact.path);

        // Ensure parent directories exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create empty file
        fs::write(&file_path, b"")?;

        // Set to read-only
        let mut perms = fs::metadata(&file_path)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&file_path, perms)?;
    }

    println!(
        "\nSuccessfully cloned '{}' into {:?}",
        project_name,
        clone_path.display()
    );
    println!("Created .rig directory and index.json");

    Ok(())
}
