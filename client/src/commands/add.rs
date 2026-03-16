use sha1::{Digest, Sha1};
use std::fs;
use std::path::PathBuf;

use protocol::{Artifact, IndexFile, Revision};

pub async fn run(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig add (local) for path: {:?}", path);

    // Determine project root (.rig)
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository (no .rig directory found)".into());
    }

    // Read local index
    let index_path = rig_dir.join("index.json");
    let index_content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read local index.json: {}", e))?;
    let mut local_index: IndexFile = serde_json::from_str(&index_content)
        .map_err(|e| format!("Failed to parse local index.json: {}", e))?;

    let artifact_name = path.to_string_lossy().to_string();

    // Read local file to verify it exists and compute hash
    let local_path = current_dir.join(&artifact_name);
    if !local_path.exists() {
        return Err(format!("File not found: {}", local_path.display()).into());
    }

    let file_data = fs::read(&local_path)?;
    let mut hasher = Sha1::new();
    hasher.update(&file_data);
    let hash = format!("{:x}", hasher.finalize());

    // Check if it's already tracked
    if let Some(artifact) = local_index.artifacts.get_mut(&artifact_name) {
        println!("Artifact '{}' is already tracked.", artifact_name);

        // If it's already in the index, rig add can be used to "stage" the current hash
        // In our current simple implementation, being writable is the "staged" state for push.
        // We'll just ensure it's recorded correctly.
        let already_has_hash = artifact.revisions.iter().any(|r| r.hash == hash);
        if !already_has_hash {
            println!("-> New local changes detected for existing artifact.");
        }
    } else {
        println!("-> Tracking new artifact: {}", artifact_name);
        local_index.artifacts.insert(
            artifact_name.clone(),
            Artifact {
                path: artifact_name.clone(),
                latest: 0, // 0 means it hasn't been pushed to server yet
                locked_by: None,
                revisions: vec![Revision {
                    rev: 0,
                    hash: hash.clone(),
                }],
            },
        );
    }

    // Persist local index.json
    fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;

    println!(
        "Added '{}' to local index. It will be uploaded on next push.",
        artifact_name
    );
    Ok(())
}
