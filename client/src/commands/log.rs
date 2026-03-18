use crate::repository::Repository;
use std::path::PathBuf;

pub async fn run(path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig log...");

    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;
    let local_index = repo.read_index()?;

    let head_hash = match repo.head_commit()? {
        Some(h) => h,
        None => {
            println!("No commits yet.");
            return Ok(());
        }
    };

    // Resolve artifact ID if path is provided
    let artifact_id = if let Some(p) = &path {
        let path_str = p.to_string_lossy().to_string();
        if let Some(a) = local_index.artifacts.get(&path_str) {
            Some(a.artifact_id.clone())
        } else {
            // Check if it's already an ID
            local_index
                .artifacts
                .values()
                .find(|a| a.artifact_id == path_str)
                .map(|a| a.artifact_id.clone())
        }
    } else {
        None
    };

    if path.is_some() && artifact_id.is_none() {
        return Err(format!("Artifact '{}' not found", path.unwrap().display()).into());
    }

    if let Some(ref id) = artifact_id {
        println!("History for artifact ID: {}", id);
        println!(
            "{:<40} {:<10} {:<30} {:<15}",
            "HASH", "REV", "MESSAGE", "AUTHOR"
        );
        println!("{}", "-".repeat(95));
    }

    let mut current_hash = Some(head_hash);
    while let Some(hash) = current_hash {
        if let Ok(Some(commit)) = repo.read_commit(&hash) {
            if let Some(ref id) = artifact_id {
                // Filter by artifact in the CommitArtifact vector
                if let Some(commit_art) = commit.artifacts.iter().find(|a| &a.artifact_id == id) {
                    println!(
                        "{:<40} {:<10} {:<30} {:<15}",
                        commit.id, commit_art.revision_base, commit.message, commit.author
                    );
                }
            } else {
                println!("{} {} - {}", commit.id, commit.message, commit.author);
            }
            current_hash = commit.parent.clone();
        } else {
            eprintln!("Error: Commit {} not found in objects.", hash);
            break;
        }
    }

    Ok(())
}
