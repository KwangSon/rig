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
    /// Fetches metadata from the remote repository without downloading files
    Fetch,
    /// Pulls changes from the remote repository and updates local files
    Pull,
    /// Pushes local changes to the remote repository, creating a new server revision
    Push {
        #[arg(short, long)]
        message: String,
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
        Commands::Fetch => {
            println!("Fetching metadata from remote repository...");
            // TODO: Add actual fetch logic
        }
        Commands::Pull => {
            println!("Pulling changes and updating local files...");
            // TODO: Add actual pull logic
        }
        Commands::Push { message } => {
            println!("Pushing changes with message: '{}'", message);
            // TODO: Add actual push logic
        }
        Commands::Status => {
            println!("Showing status...");
            // TODO: Add actual status logic
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
