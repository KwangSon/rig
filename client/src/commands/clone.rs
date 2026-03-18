use crate::repository::{Config, Index, Repository};
use protocol::IndexFile;
use std::fs;
use std::path::PathBuf;

pub async fn run(
    url: &str,
    path: &Option<PathBuf>,
    provided_username: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed_url = url.trim_end_matches('/');

    // We expect URLs like:
    // ssh://rig@localhost:2222/username/project_name
    // http://localhost:3000/username/project_name
    let (project_name, username, base_url) = if trimmed_url.starts_with("ssh://") {
        let parts: Vec<&str> = trimmed_url.split('/').collect();
        if parts.len() < 5 {
            return Err("Invalid SSH URL format. Expected ssh://<user>@<host>:<port>/<username>/<project>".into());
        }
        let project = parts.last().unwrap().to_string();
        let username = parts[parts.len() - 2].to_string();
        // Discard SSH specific host part, fall back to API server directly (per user instruction to use HTTP fallback)
        let api_base = "http://localhost:3000".to_string();
        (project, Some(username), api_base)
    } else {
        let parts: Vec<&str> = trimmed_url.split('/').collect();
        if parts.len() < 5 {
            return Err("Invalid HTTP URL format. Expected http://<server>:<port>/<username>/<project>".into());
        }
        let project = parts.last().unwrap().to_string();
        let username = parts[parts.len() - 2].to_string();
        
        let mut base = String::new();
        // everything up to the username part is the base URL
        for i in 0..(parts.len() - 2) {
            base.push_str(parts[i]);
            if i < parts.len() - 3 {
                base.push('/');
            }
        }
        (project, Some(username), base)
    };

    println!(
        "Cloning project '{}/{}' from server '{}'",
        username.as_deref().unwrap_or("unknown"), project_name, base_url
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
    let metadata_url = format!("{}/api/v1/{}/index", base_url, project_name);
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
    let resolved_username = if let Some(u) = provided_username {
        u.clone()
    } else if let Some(u) = username {
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
    index.username = Some(resolved_username.clone());

    // 3. Create .rig folder and initialize via Repository
    let clone_path = match path {
        Some(p) => p.clone(),
        None => PathBuf::from(project_name.clone()),
    };
    println!("-> Cloning into {:?}...", clone_path.display());

    if clone_path.exists() && clone_path.read_dir()?.next().is_some() {
        return Err(format!(
            "Destination path '{:?}' already exists and is not an empty directory.",
            clone_path.display()
        )
        .into());
    }

    fs::create_dir_all(&clone_path)?;
    let repo = Repository::init(&clone_path)?;

    // 3.1 Write config
    let config = Config {
        project: project_name.to_string(),
        server_url: Some(base_url.to_string()),
        username: Some(resolved_username),
    };
    repo.write_config(&config)?;

    for (_hash, commit) in index.commits {
        repo.write_commit(&commit)?;
    }

    // 3.3 Set HEAD and refs/heads/main if there are commits
    if !index.latest_commit.is_empty() {
        repo.write_ref("refs/heads/main", &index.latest_commit)?;
    }

    // 3.4 Write tracking index
    let mut local_artifacts = std::collections::HashMap::new();
    for artifact in index.artifacts.values() {
        local_artifacts.insert(
            artifact.path.clone(),
            protocol::IndexArtifact {
                artifact_id: artifact.id.clone(),
                revision: artifact.latest,
                local_state: "placeholder".to_string(),
                stage: "none".to_string(),
                locked: false,
                lock_owner: None,
                lock_generation: None,
                staged: None,
                moved_from: None,
            },
        );
    }

    let local_index = Index {
        version: 1,
        branch: "main".to_string(),
        head: None, // Reset head to null (spec: null if no unpushed commits exist)
        artifacts: local_artifacts,
        git_modules: index.git_modules,
    };
    repo.write_index(&local_index)?;

    // 4. Create empty read-only files for each artifact
    for path in local_index.artifacts.keys() {
        let file_path = clone_path.join(path);
        println!("-> Creating placeholder for {}", path);

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
    println!("Created Git-like .rig directory structure.");

    Ok(())
}
