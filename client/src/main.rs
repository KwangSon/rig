use clap::{Parser, Subcommand};
use reqwest;
use serde_json;
use std::path::PathBuf;

mod commands;

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
            if let Err(e) = commands::push::run(message).await {
                eprintln!("[error] Failed to push: {}", e);
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
    let index: serde_json::Value = serde_json::from_str(&index_content)?;
    let _project = index["project"].as_str().ok_or("Invalid index.json")?;
    let path_str = path.to_string_lossy();

    // Check if artifact exists
    if !index["artifacts"].get(&*path_str).is_some() {
        return Err(format!("Artifact '{}' not found", path_str).into());
    }

    // Send POST to server (lock endpoint is not namespaced by project)
    let client = reqwest::Client::new();
    let url = format!("http://localhost:3000/artifacts/{}/lock", path_str);
    let body = serde_json::json!({"user": "alice"});
    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        return Err(format!("Lock request failed: {}", resp.status()).into());
    }
    let resp_json: serde_json::Value = resp.json().await?;
    if !resp_json["locked"].as_bool().unwrap_or(false) {
        return Err("Lock request denied by server".into());
    }

    // Change local file permission to writable
    let local_path = current_dir.join(&*path_str);
    if local_path.exists() {
        let mut perms = std::fs::metadata(&local_path)?.permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&local_path, perms)?;
        println!("Artifact '{}' is now writable", path_str);
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
    let index: serde_json::Value = serde_json::from_str(&index_content)?;
    let _project = index["project"].as_str().ok_or("Invalid index.json")?;
    let path_str = path.to_string_lossy();

    // Check if artifact exists
    if !index["artifacts"].get(&*path_str).is_some() {
        return Err(format!("Artifact '{}' not found", path_str).into());
    }

    // Send DELETE to server (unlock endpoint is not namespaced by project)
    let client = reqwest::Client::new();
    let url = format!("http://localhost:3000/artifacts/{}/lock", path_str);
    let resp = client.delete(&url).send().await?;
    if !resp.status().is_success() {
        return Err(format!("Unlock request failed: {}", resp.status()).into());
    }

    // Change local file permission to read-only
    let local_path = current_dir.join(&*path_str);
    if local_path.exists() {
        let mut perms = std::fs::metadata(&local_path)?.permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&local_path, perms)?;
        println!("Artifact '{}' is now read-only", path_str);
    }

    Ok(())
}
