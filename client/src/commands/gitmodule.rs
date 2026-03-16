use crate::GitModuleCommands;
use protocol::{GitModule, IndexFile};
use std::fs;
use std::process::Command;

pub async fn run(subcommand: &GitModuleCommands) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository".into());
    }

    let index_path = rig_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path)?;
    let mut index: IndexFile = serde_json::from_str(&index_content)?;

    match subcommand {
        GitModuleCommands::Add { url, path, commit } => {
            let path_str = path.to_string_lossy().to_string();

            let commit_hash = if let Some(c) = commit {
                c.clone()
            } else {
                // Fetch HEAD from remote
                println!("Fetching remote HEAD for {}...", url);
                let output = Command::new("git")
                    .args(["ls-remote", url, "HEAD"])
                    .output()?;
                if !output.status.success() {
                    return Err(format!(
                        "Failed to fetch remote HEAD: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .into());
                }
                String::from_utf8_lossy(&output.stdout)
                    .split_whitespace()
                    .next()
                    .ok_or("Could not parse remote HEAD hash")?
                    .to_string()
            };

            let module = GitModule {
                path: path_str.clone(),
                url: url.clone(),
                commit: commit_hash,
            };

            // Update server
            let client = reqwest::Client::new();
            let server_url = index
                .server_url
                .as_ref()
                .ok_or("Server URL not configured")?;
            let api_url = format!(
                "{}/api/v1/{}/gitmodules/{}",
                server_url, index.project, path_str
            );

            let resp = client.put(&api_url).json(&module).send().await?;
            if !resp.status().is_success() {
                return Err(
                    format!("Failed to update gitmodule on server: {}", resp.status()).into(),
                );
            }

            index.git_modules.insert(path_str, module);
            fs::write(&index_path, serde_json::to_string_pretty(&index)?)?;
            println!("Added gitmodule at {}", path.display());
        }
        GitModuleCommands::Update { path, commit } => {
            let path_str = path.to_string_lossy().to_string();
            let mut module = index
                .git_modules
                .get(&path_str)
                .ok_or_else(|| format!("Gitmodule at {} not found", path_str))?
                .clone();

            module.commit = commit.clone();

            // Update server
            let client = reqwest::Client::new();
            let server_url = index
                .server_url
                .as_ref()
                .ok_or("Server URL not configured")?;
            let api_url = format!(
                "{}/api/v1/{}/gitmodules/{}",
                server_url, index.project, path_str
            );

            let resp = client.put(&api_url).json(&module).send().await?;
            if !resp.status().is_success() {
                return Err(
                    format!("Failed to update gitmodule on server: {}", resp.status()).into(),
                );
            }

            index.git_modules.insert(path_str, module);
            fs::write(&index_path, serde_json::to_string_pretty(&index)?)?;
            println!(
                "Updated gitmodule at {} to commit {}",
                path.display(),
                commit
            );
        }
        GitModuleCommands::Status => {
            if index.git_modules.is_empty() {
                println!("No gitmodules configured.");
            } else {
                println!("{:<30} {:<50} {:<40}", "PATH", "URL", "COMMIT");
                println!("{}", "-".repeat(120));
                for module in index.git_modules.values() {
                    println!(
                        "{:<30} {:<50} {:<40}",
                        module.path, module.url, module.commit
                    );
                }
            }
        }
        GitModuleCommands::Sync => {
            for module in index.git_modules.values() {
                let module_path = current_dir.join(&module.path);
                if !module_path.exists() {
                    println!("Cloning {} into {}...", module.url, module.path);
                    let status = Command::new("git")
                        .args(["clone", &module.url, &module.path])
                        .status()?;
                    if !status.success() {
                        eprintln!("Error: Failed to clone {}", module.url);
                        continue;
                    }
                }

                println!(
                    "Checking out commit {} in {}...",
                    module.commit, module.path
                );
                let status = Command::new("git")
                    .current_dir(&module_path)
                    .args(["checkout", &module.commit])
                    .status()?;
                if !status.success() {
                    eprintln!(
                        "Error: Failed to checkout commit {} in {}",
                        module.commit, module.path
                    );
                }
            }
        }
    }

    Ok(())
}
