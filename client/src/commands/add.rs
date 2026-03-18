use crate::repository::Repository;

use std::fs;
use uuid::Uuid;

pub async fn run(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let path_str = path.to_string_lossy().to_string();
    println!("Running rig add (local) for path: {}", path_str);

    // Check for source code extensions and warn
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let source_extensions = [
            "rs", "py", "c", "cpp", "h", "hpp", "js", "ts", "go", "java", "rb", "sh", "lua",
        ];
        if source_extensions.contains(&ext) {
            println!(
                "\x1b[33mWarning: You are adding a source code file ({}).\x1b[0m",
                path_str
            );
            println!(
                "\x1b[33mFor source code, it is highly recommended to use 'rig gitmodule' to manage it via Git.\x1b[0m"
            );
        }
    }

    // Read local index
    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;
    let mut local_index = repo.read_index()?;

    // Verify file exists
    let local_path = current_dir.join(&path_str);
    if !local_path.exists() {
        return Err(format!("File not found: {}", local_path.display()).into());
    }

    let metadata = fs::metadata(&local_path)?;
    let size = metadata.len();
    let mtime = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    // Check if it's already tracked
    if let Some(artifact) = local_index.artifacts.get_mut(&path_str) {
        println!(
            "Artifact '{}' (id: {}) is already tracked.",
            path_str, artifact.artifact_id
        );

        // Verify lock requirement
        if !artifact.locked {
            return Err(format!(
                "ERROR: File '{}' is read-only. Use 'rig lock' first.",
                path_str
            )
            .into());
        }

        // Verify it's not a placeholder
        if artifact.local_state == "placeholder" {
            return Err(format!(
                "ERROR: File '{}' not downloaded. Run 'rig pull' first.",
                path_str
            )
            .into());
        }

        artifact.stage = "staged".to_string();
        artifact.staged = Some(protocol::StagedInfo { mtime, size });
    } else {
        // Create a new unique ID for new file
        let new_id = Uuid::new_v4().to_string();
        println!("-> Tracking new artifact: {} (id: {})", path_str, new_id);
        local_index.artifacts.insert(
            path_str.clone(),
            protocol::IndexArtifact {
                artifact_id: new_id,
                revision: 0,
                local_state: "ready".to_string(),
                stage: "staged".to_string(),
                locked: true, // New files are effectively locked by creator
                lock_owner: None,
                lock_generation: None,
                staged: Some(protocol::StagedInfo { mtime, size }),
                moved_from: None,
            },
        );
    }

    // Persist local index
    repo.write_index(&local_index)?;

    println!(
        "Added '{}' to local index. It will be uploaded on next push.",
        path_str
    );
    Ok(())
}
