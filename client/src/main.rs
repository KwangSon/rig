use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

mod commands;
pub mod repository;
mod utils;

use crate::repository::{Index, Repository};

fn resolve_artifact_id(index: &Index, query: &str) -> Option<String> {
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
    /// Clones a rig repository
    Clone {
        /// The URL of the repository to clone
        url: String,
        /// The path to clone into. Defaults to the repository name.
        path: Option<PathBuf>,
        /// Optional username for this repository
        #[arg(short, long)]
        username: Option<String>,
    },
    /// Fetches metadata from the remote repository without downloading files
    Fetch,
    /// Shows commit history
    Log {
        /// Optional path to show history for a specific artifact
        path: Option<PathBuf>,
    },
    /// Pulls changes from the remote repository and updates local files
    Pull {
        /// The path to the artifact to pull (e.g., 'file.png', 'dir/', '*', or 'file.png@10')
        path: String,
        /// Optional revision (e.g., '@10')
        revision: Option<String>,
        /// Optional output path for the pulled artifact
        #[arg(short, long)]
        out: Option<PathBuf>,
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
    /// Shows the working tree status
    Status,
    /// Moves or renames an artifact
    Mv {
        /// The source path of the artifact to move
        src: PathBuf,
        /// The destination path of the artifact
        dst: PathBuf,
    },
    /// Adds a new artifact or updates an existing one (requires lock)
    Add {
        /// The path to the artifact to add
        path: PathBuf,
    },
    /// Lists, creates, or deletes branches
    Branch { name: Option<String> },
    /// Switches to a different branch
    Checkout {
        /// The name of the branch to checkout
        branch: String,
    },
    /// Stashes changes for temporary storage
    Stash,
    /// Locks an artifact to prevent others from editing
    Lock {
        /// The path to the artifact to lock
        path: PathBuf,
    },
    /// Unlocks an artifact to allow others to edit
    Unlock {
        /// The path to the artifact to unlock
        path: PathBuf,
        /// Force unlock even if locked by another user
        #[arg(short, long)]
        force: bool,
    },
    /// Manages git modules (snapshot of external git repositories)
    Gitmodule {
        #[command(subcommand)]
        subcommand: GitModuleCommands,
    },
}

#[derive(clap::Subcommand)]
pub enum GitModuleCommands {
    /// Adds a new git module
    Add {
        /// URL of the git repository
        url: String,
        /// Path where the module should be placed
        path: PathBuf,
        /// Specific commit hash (optional, defaults to HEAD of remote)
        #[arg(short, long)]
        commit: Option<String>,
    },
    /// Updates an existing git module to a specific commit
    Update {
        /// Path of the module to update
        path: PathBuf,
        /// Specific commit hash
        #[arg(short, long)]
        commit: String,
    },
    /// Lists all git modules and their configured commits
    Status,
    /// Synchronizes local git modules (clones and checks out configured commits)
    Sync,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Clone {
            url,
            path,
            username,
        } => {
            if let Err(e) = commands::clone::run(url, path, username).await {
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
        Commands::Log { path } => {
            if let Err(e) = commands::log::run(path.clone()).await {
                eprintln!("[error] Failed to show log: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Pull {
            path,
            revision,
            out,
        } => {
            if let Err(e) = commands::pull::run(path.clone(), revision.clone(), out.clone()).await {
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
        Commands::Mv { src, dst } => {
            if let Err(e) = commands::mv::run(src.clone(), dst.clone()).await {
                eprintln!("[error] Failed to move artifact: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Add { path } => {
            if let Err(e) = commands::add::run(path).await {
                eprintln!("[error] Failed to add: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Branch { name } => {
            if let Err(e) = commands::branch::run(name.clone()).await {
                eprintln!("[error] Branch command failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Checkout { branch } => {
            if let Err(e) = commands::checkout::run(branch).await {
                eprintln!("[error] Checkout command failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Stash => {
            if let Err(e) = commands::stash::run().await {
                eprintln!("[error] Stash command failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Lock { path } => {
            if let Err(e) = lock_artifact(path).await {
                eprintln!("[error] Failed to lock: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Unlock { path, force } => {
            if let Err(e) = unlock_artifact(path, *force).await {
                eprintln!("[error] Failed to unlock: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Gitmodule { subcommand } => {
            if let Err(e) = commands::gitmodule::run(subcommand).await {
                eprintln!("[error] Gitmodule command failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}

async fn lock_artifact(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Locking artifact: {}", path.display());

    // Get current dir, open repo
    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;

    // Read index and config
    let index = repo.read_index()?;
    let config = repo.read_config()?;

    let path_str = path.to_string_lossy().to_string();
    let artifact_id = resolve_artifact_id(&index, &path_str)
        .ok_or_else(|| format!("Artifact '{}' not found", path_str))?;

    let username = config.username.as_deref().unwrap_or("unknown");

    // Send POST to server (lock endpoint is namespaced by project)
    let client = reqwest::Client::new();
    let server_url = config
        .server_url
        .as_deref()
        .unwrap_or("http://localhost:3000");
    let url = format!(
        "{}/api/v1/{}/artifacts/{}/lock",
        server_url, config.project, artifact_id
    );
    let body = serde_json::json!({"user": username});
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
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        std::fs::set_permissions(&local_path, perms)?;
        println!(
            "Artifact '{}' is now writable",
            index.artifacts[&artifact_id].path
        );
    }

    Ok(())
}

async fn unlock_artifact(path: &Path, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Unlocking artifact: {}", path.display());

    // Get current dir, open repo
    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;

    // Read index and config
    let index = repo.read_index()?;
    let config = repo.read_config()?;

    let path_str = path.to_string_lossy().to_string();
    let artifact_id = resolve_artifact_id(&index, &path_str)
        .ok_or_else(|| format!("Artifact '{}' not found", path_str))?;

    let username = config.username.as_deref().unwrap_or("unknown");

    // Send DELETE to server (unlock endpoint is namespaced by project)
    let client = reqwest::Client::new();
    let server_url = config
        .server_url
        .as_deref()
        .unwrap_or("http://localhost:3000");
    let url = format!(
        "{}/api/v1/{}/artifacts/{}/lock",
        server_url, config.project, artifact_id
    );
    let body = serde_json::json!({
        "user": username,
        "force": force
    });
    let resp = client.delete(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            let resp_json: serde_json::Value = resp.json().await?;
            let locked_by = resp_json["user"].as_str().unwrap_or("another user");
            return Err(format!("Unlock denied: artifact is locked by {}", locked_by).into());
        }
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
