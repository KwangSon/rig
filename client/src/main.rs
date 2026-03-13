use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Synchronizes the local workspace with the remote repository
    Sync,
    /// Shows the working tree status
    Status,
    /// Submits changes to the remote repository
    Submit {
        #[arg(short, long)]
        message: String,
    },
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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            println!("Initializing new rig repository...");
            // TODO: Add actual initialization logic
        }
        Commands::Clone { url, path } => {
            println!("Cloning repository from: {}", url);
            if let Some(p) = path {
                println!("Cloning into: {}", p.display());
            } else {
                println!("Cloning into a directory derived from the URL.");
            }
            // TODO: Add actual cloning logic
        }
        Commands::Sync => {
            println!("Syncing workspace...");
            // TODO: Add actual sync logic
        }
        Commands::Status => {
            println!("Showing status...");
            // TODO: Add actual status logic
        }
        Commands::Submit { message } => {
            println!("Submitting changes with message: '{}'", message);
            // TODO: Add actual submit logic
        }
        Commands::Lock { path } => {
            println!("Locking artifact at: {}", path.display());
            // TODO: Add actual lock logic
        }
        Commands::Unlock { path } => {
            println!("Unlocking artifact at: {}", path.display());
            // TODO: Add actual unlock logic
        }
    }
}
