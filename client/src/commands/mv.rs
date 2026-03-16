use protocol::IndexFile;
use std::fs;
use std::path::PathBuf;

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

pub async fn run(src: PathBuf, dst: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let src_str = src.to_string_lossy().to_string();
    let dst_str = dst.to_string_lossy().to_string();
    println!("Running rig mv: {} -> {}", src_str, dst_str);

    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");
    if !rig_dir.exists() {
        return Err("Not a rig repository".into());
    }

    let index_path = rig_dir.join("index.json");
    let mut local_index: IndexFile = serde_json::from_str(&fs::read_to_string(&index_path)?)?;

    if let Some(id) = resolve_artifact_id(&local_index, &src_str) {
        let artifact = local_index.artifacts.get_mut(&id).unwrap();

        // Update path and moved_from
        artifact.path = dst_str.clone();
        artifact.moved_from = Some(src_str.clone());

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

        fs::write(&index_path, serde_json::to_string_pretty(&local_index)?)?;
        println!("   Metadata updated in index.json (ID: {})", id);
    } else {
        return Err(format!("Artifact '{}' not found", src_str).into());
    }

    Ok(())
}
