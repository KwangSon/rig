use crate::repository::Repository;
use std::fs;
use std::path::PathBuf;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running rig status (local)...");

    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;

    // 1. Read local index, head commit, and config
    let local_index = repo.read_index()?;
    let config = repo.read_config()?;

    // Read the actual Commit object string from the hash stored in HEAD
    let latest_commit = if let Ok(Some(hash)) = repo.head_commit() {
        repo.read_commit(&hash).unwrap_or(None)
    } else {
        None
    };

    println!("Project: {}", config.project);

    // 2. Scan workspace for files
    let mut untracked_files = Vec::new();
    let mut modified_files = Vec::new(); // Writable files
    let mut staged_files = Vec::new(); // Added but not in any commit yet
    let mut committed_files = Vec::new(); // In index, in latest commit, but latest is 0 (not pushed)

    // Helper to scan recursively
    fn collect_files(dir: &PathBuf, base: &PathBuf) -> Vec<PathBuf> {
        let mut results = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if file_name == ".rig" || file_name.starts_with('.') || file_name == "target" {
                    continue;
                }
                if path.is_dir() {
                    results.extend(collect_files(&path, base));
                } else if let Ok(rel_path) = path.strip_prefix(base) {
                    results.push(rel_path.to_path_buf());
                }
            }
        }
        results
    }

    let all_workspace_files = collect_files(&current_dir, &current_dir);

    // 3. Compare with local index
    let mut path_to_artifact = std::collections::HashMap::new();
    let mut artifact_id_for_path = std::collections::HashMap::new();
    for (id, artifact) in &local_index.artifacts {
        path_to_artifact.insert(artifact.path.as_str(), artifact);
        artifact_id_for_path.insert(artifact.path.as_str(), id.clone());
    }

    for rel_path in &all_workspace_files {
        let path_str = rel_path.to_string_lossy().to_string();

        if let Some(artifact) = path_to_artifact.get(path_str.as_str()) {
            if artifact.latest == 0 {
                // If it's in the latest commit, it's "committed but not pushed"
                let artifact_id = artifact_id_for_path.get(path_str.as_str()).unwrap();
                let in_commit = latest_commit
                    .as_ref()
                    .is_some_and(|c| c.artifacts.contains_key(artifact_id));
                if in_commit {
                    committed_files.push(path_str);
                } else {
                    staged_files.push(path_str);
                }
            } else {
                // Tracked on server - check if modified (writable)
                let full_path = current_dir.join(rel_path);
                if fs::metadata(full_path).is_ok_and(|m| !m.permissions().readonly()) {
                    modified_files.push(path_str);
                }
            }
        } else {
            untracked_files.push(path_str);
        }
    }

    // Check for missing files
    let mut missing_files = Vec::new();
    for (id, artifact) in &local_index.artifacts {
        let full_path = current_dir.join(&artifact.path);
        if !full_path.exists() {
            missing_files.push(id.clone());
        }
    }

    // Output results
    if !staged_files.is_empty() {
        println!("\nChanges to be committed (staged):");
        for file in &staged_files {
            println!("  (new)      {}", file);
        }
    }

    if !committed_files.is_empty() {
        println!("\nChanges committed but not pushed:");
        for file in &committed_files {
            println!("  (committed){}", file);
        }
    }

    if !modified_files.is_empty() {
        println!("\nChanges not staged for commit (modified):");
        for file in &modified_files {
            println!("  (modified) {}", file);
        }
    }

    if !untracked_files.is_empty() {
        println!("\nUntracked files:");
        for file in &untracked_files {
            println!("             {}", file);
        }
    }

    if !missing_files.is_empty() {
        println!("\nMissing files:");
        for file in &missing_files {
            println!("  (missing)  {}", file);
        }
    }

    if staged_files.is_empty()
        && committed_files.is_empty()
        && modified_files.is_empty()
        && untracked_files.is_empty()
        && missing_files.is_empty()
    {
        println!("\nNothing to commit, working tree clean.");
    }

    Ok(())
}
