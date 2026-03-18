use crate::repository::{Index, Repository};
use std::fs;
use std::path::PathBuf;

fn resolve_artifact_id(index: &Index, query: &str) -> Option<String> {
    if index.artifacts.contains_key(query) {
        return Some(query.to_string());
    }
    index
        .artifacts
        .iter()
        .find(|(path, _)| path == &query)
        .map(|(path, _)| path.clone())
}

pub async fn run(src: PathBuf, dst: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let src_str = src.to_string_lossy().to_string();
    let dst_str = dst.to_string_lossy().to_string();
    println!("Running rig mv: {} -> {}", src_str, dst_str);

    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;
    let mut local_index = repo.read_index()?;

    if let Some(mut artifact) = local_index.artifacts.remove(&src_str) {
        // Update moved_from
        artifact.moved_from = Some(src_str.clone());

        // Re-insert with new path
        local_index.artifacts.insert(dst_str.clone(), artifact);

        // Physical rename
        let src_full = current_dir.join(&src);
        let dst_full = current_dir.join(&dst);

        if let Some(parent) = dst_full.parent() {
            fs::create_dir_all(parent).ok();
        }

        if src_full.exists() {
            let mut perms = fs::metadata(&src_full)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            fs::set_permissions(&src_full, perms)?;

            fs::rename(&src_full, &dst_full)?;

            let mut perms = fs::metadata(&dst_full)?.permissions();
            perms.set_readonly(true);
            fs::set_permissions(&dst_full, perms)?;
            println!("   Renamed local file.");
        }

        repo.write_index(&local_index)?;
        println!("   Metadata updated in local index.");
    } else {
        return Err(format!("Artifact '{}' not found", src_str).into());
    }

    Ok(())
}
