use crate::GitModuleCommands;
use crate::auth::ensure_authenticated;
use crate::repository::Repository;
use protocol::{GitModule, IndexFile};
use std::fs;
use std::process::Command;

pub async fn run(subcommand: &GitModuleCommands) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;

    let mut index = repo.read_index()?;
    let config = repo.read_config()?;

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
            let server_url = config
                .server_url
                .as_ref()
                .ok_or("Server URL not configured")?;

            let token = match ensure_authenticated(server_url).await {
                Ok(t) => t,
                Err(e) => return Err(format!("Authentication failed: {}", e).into()),
            };

            let api_url = format!(
                "{}/api/v1/{}/gitmodules/{}",
                server_url,
                config.project_key(),
                path_str
            );

            let resp = client
                .put(&api_url)
                .header("authorization", format!("Bearer {}", token))
                .json(&module)
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(
                    format!("Failed to update gitmodule on server: {}", resp.status()).into(),
                );
            }

            index.git_modules.insert(path_str, module);
            repo.write_index(&index)?;
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
            let server_url = config
                .server_url
                .as_ref()
                .ok_or("Server URL not configured")?;

            let token = match ensure_authenticated(server_url).await {
                Ok(t) => t,
                Err(e) => return Err(format!("Authentication failed: {}", e).into()),
            };

            let api_url = format!(
                "{}/api/v1/{}/gitmodules/{}",
                server_url,
                config.project_key(),
                path_str
            );

            let resp = client
                .put(&api_url)
                .header("authorization", format!("Bearer {}", token))
                .json(&module)
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(
                    format!("Failed to update gitmodule on server: {}", resp.status()).into(),
                );
            }

            index.git_modules.insert(path_str, module);
            repo.write_index(&index)?;
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
