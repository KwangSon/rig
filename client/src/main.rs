use clap::{Parser, Subcommand};
use reqwest;
use serde_json;
use std::path::PathBuf;

mod commands;
use crate::commands::status::IndexFile;

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

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes a new rig repository
    Init,
    /// Clones a rig repository
    Clone {
        /// The URL of the repository to clone
        url: String,
        /// The path to clone into. Defaults to the repository name.
        path: Option<PathBuf>,
    },
    /// Fetches metadata from the remote repository without downloading files
    Fetch,
    /// Shows commit history
    Log,
    /// Pulls changes from the remote repository and updates local files
    Pull {
        /// The path to the artifact to pull
        path: PathBuf,
    },
    /// Pushes local changes to the remote repository, creating a new server revision
    Push {
        /// Optional commit message (defaults to the last local commit message if omitted)
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Creates a new local commit (does not push to server)
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },
    /// Adds a new artifact or updates an existing one (requires lock)
    Add {
        /// The path to the artifact to add
        path: PathBuf,
    },
    /// Shows the working tree status
    Status,
    /// Locks an artifact to prevent others from editing
    Lock {
        /// The path to the artifact to lock
        path: PathBuf,
    },
    /// Unlocks an artifact to allow others to edit
    Unlock {
        /// The path to the artifact to unlock
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            if let Err(e) = commands::init::run().await {
                eprintln!("[error] Failed to initialize repository: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Clone { url, path } => {
            if let Err(e) = commands::clone::run(url, path).await {
                eprintln!("[error] Failed to clone repository: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Fetch => {
            if let Err(e) = commands::fetch::run().await {
                eprintln!("[error] Failed to fetch: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Log => {
            if let Err(e) = commands::log::run().await {
                eprintln!("[error] Failed to show log: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Pull { path } => {
            if let Err(e) = commands::pull::run(path.clone()).await {
                eprintln!("[error] Failed to pull: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Push { message } => {
            if let Err(e) = commands::push::run(message.clone()).await {
                eprintln!("[error] Failed to push: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Commit { message } => {
            if let Err(e) = commands::commit::run(message).await {
                eprintln!("[error] Failed to commit: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Status => {
            if let Err(e) = commands::status::run().await {
                eprintln!("[error] Failed to get status: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Add { path } => {
            if let Err(e) = commands::add::run(path).await {
                eprintln!("[error] Failed to add: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Lock { path } => {
            if let Err(e) = lock_artifact(path).await {
                eprintln!("[error] Failed to lock: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Unlock { path } => {
            if let Err(e) = unlock_artifact(path).await {
                eprintln!("[error] Failed to unlock: {}", e);
                std::process::exit(1);
            }
        }
    }
}

async fn lock_artifact(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Locking artifact: {}", path.display());

    // Get current dir, check .rig
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository".into());
    }

    // Read index.json
    let index_path = rig_dir.join("index.json");
    let index_content = std::fs::read_to_string(&index_path)?;
    let index: IndexFile = serde_json::from_str(&index_content)?;

    let path_str = path.to_string_lossy().to_string();
    let artifact_id = resolve_artifact_id(&index, &path_str)
        .ok_or_else(|| format!("Artifact '{}' not found", path_str))?;

    // Send POST to server (lock endpoint is namespaced by project)
    let client = reqwest::Client::new();
    let url = format!(
        "http://localhost:3000/{}/artifacts/{}/lock",
        index.project, artifact_id
    );
    let body = serde_json::json!({"user": "alice"});
    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        return Err(format!("Lock request failed: {}", resp.status()).into());
    }
    let resp_json: serde_json::Value = resp.json().await?;
    if !resp_json["locked"].as_bool().unwrap_or(false) {
        return Err("Lock request denied by server".into());
    }

    // Change local file permission to writable (use the effective artifact path)
    let local_path = current_dir.join(&index.artifacts[&artifact_id].path);
    if local_path.exists() {
        let mut perms = std::fs::metadata(&local_path)?.permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&local_path, perms)?;
        println!(
            "Artifact '{}' is now writable",
            index.artifacts[&artifact_id].path
        );
    }

    Ok(())
}

async fn unlock_artifact(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Unlocking artifact: {}", path.display());

    // Get current dir, check .rig
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository".into());
    }

    // Read index.json
    let index_path = rig_dir.join("index.json");
    let index_content = std::fs::read_to_string(&index_path)?;
    let index: IndexFile = serde_json::from_str(&index_content)?;

    let path_str = path.to_string_lossy().to_string();
    let artifact_id = resolve_artifact_id(&index, &path_str)
        .ok_or_else(|| format!("Artifact '{}' not found", path_str))?;

    // Send DELETE to server (unlock endpoint is namespaced by project)
    let client = reqwest::Client::new();
    let url = format!(
        "http://localhost:3000/{}/artifacts/{}/lock",
        index.project, artifact_id
    );
    let resp = client.delete(&url).send().await?;
    if !resp.status().is_success() {
        return Err(format!("Unlock request failed: {}", resp.status()).into());
    }

    // Change local file permission to read-only (use the effective artifact path)
    let local_path = current_dir.join(&index.artifacts[&artifact_id].path);
    if local_path.exists() {
        let mut perms = std::fs::metadata(&local_path)?.permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&local_path, perms)?;
        println!(
            "Artifact '{}' is now read-only",
            index.artifacts[&artifact_id].path
        );
    }

    Ok(())
}
