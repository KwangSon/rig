use sha1::{Digest, Sha1};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use protocol::{Artifact, IndexFile, Revision};

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

pub async fn run(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig add (local) for path: {:?}", path);

    // Check for source code extensions and warn
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let source_extensions = [
            "rs", "py", "c", "cpp", "h", "hpp", "js", "ts", "go", "java", "rb", "sh", "lua",
        ];
        if source_extensions.contains(&ext) {
            println!(
                "\x1b[33mWarning: You are adding a source code file ({:?}).\x1b[0m",
                path
            );
            println!(
                "\x1b[33mFor source code, it is highly recommended to use 'rig gitmodule' to manage it via Git.\x1b[0m"
            );
        }
    }

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

    let path_str = path.to_string_lossy().to_string();

    // Read local file to verify it exists and compute hash
    let local_path = current_dir.join(&path_str);
    if !local_path.exists() {
        return Err(format!("File not found: {}", local_path.display()).into());
    }

    let file_data = fs::read(&local_path)?;
    let mut hasher = Sha1::new();
    hasher.update(&file_data);
    let hash = format!("{:x}", hasher.finalize());

    // Check if it's already tracked (look up by path)
    if let Some(id) = resolve_artifact_id(&local_index, &path_str) {
        let artifact = local_index.artifacts.get_mut(&id).unwrap();
        println!("Artifact '{}' (id: {}) is already tracked.", path_str, id);

        let already_has_hash = artifact.revisions.iter().any(|r| r.hash == hash);
        if !already_has_hash {
            println!("-> New local changes detected for existing artifact.");
        }
    } else {
        // Create a new unique ID
        let new_id = Uuid::new_v4().to_string();
        println!("-> Tracking new artifact: {} (id: {})", path_str, new_id);
        local_index.artifacts.insert(
            new_id.clone(),
            Artifact {
                id: new_id.clone(),
                path: path_str.clone(),
                latest: 0,
                locked_by: None,
                revisions: vec![Revision {
                    rev: 0,
                    hash: hash.clone(),
                    compressed: false,
                }],
                moved_from: None,
            },
        );
    }

    // Persist local index.json
    fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;

    println!(
        "Added '{}' to local index. It will be uploaded on next push.",
        path_str
    );
    Ok(())
}
