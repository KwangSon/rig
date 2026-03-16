use flate2::read::GzDecoder;
use protocol::IndexFile;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub async fn run(
    path_arg: String,
    rev_arg: Option<String>,
    out_arg: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Running rig pull for path: {} (rev: {:?}, out: {:?})",
        path_arg, rev_arg, out_arg
    );

    // 1. Parse path and revision/commit
    let mut path_str = path_arg.clone();
    let mut requested_rev: Option<u32> = None;
    let mut requested_commit: Option<String> = None;

    // Support '#' for revision and '@' for commit
    if let Some(pos) = path_str.find('#') {
        let rev_part = &path_str[pos + 1..];
        if let Ok(rev) = rev_part.parse::<u32>() {
            requested_rev = Some(rev);
        }
        path_str = path_str[..pos].to_string();
    } else if let Some(pos) = path_str.find('@') {
        let commit_part = &path_str[pos + 1..];
        requested_commit = Some(commit_part.to_string());
        path_str = path_str[..pos].to_string();
    }

    // Overrides from flags
    if let Some(ref r) = rev_arg {
        if r.starts_with('#') {
            if let Ok(rev) = r[1..].parse::<u32>() {
                requested_rev = Some(rev);
            }
        } else if r.starts_with('@') {
            requested_commit = Some(r[1..].to_string());
        } else if let Ok(rev) = r.parse::<u32>() {
            requested_rev = Some(rev);
        }
    }

    // Determine the current project root (where .rig is located)
    let current_dir = std::env::current_dir()?;
    let rig_dir = current_dir.join(".rig");

    if !rig_dir.exists() || !rig_dir.is_dir() {
        return Err("Not a rig repository".into());
    }

    // Read local index
    let local_index_path = rig_dir.join("index.json");
    let local_index: IndexFile = serde_json::from_str(&fs::read_to_string(&local_index_path)?)?;

    // Fetch latest index from server
    let client = reqwest::Client::new();
    let server_url = local_index
        .server_url
        .as_deref()
        .unwrap_or("http://localhost:3000");
    let remote_index_url = format!("{}/api/v1/{}/index.json", server_url, local_index.project);
    let remote_resp = client.get(&remote_index_url).send().await?;
    let remote_index: IndexFile = serde_json::from_str(&remote_resp.text().await?)?;

    // Resolution helpers
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

    // Determine artifacts and their target revisions
    let mut targets: Vec<(String, u32)> = Vec::new();

    if path_str == "*" {
        // All artifacts
        for (id, artifact) in &remote_index.artifacts {
            let rev = if let Some(ref commit_hash) = requested_commit {
                let commit = remote_index
                    .commits
                    .get(commit_hash)
                    .ok_or_else(|| format!("Commit {} not found", commit_hash))?;
                *commit.artifacts.get(id).unwrap_or(&artifact.latest)
            } else {
                requested_rev.unwrap_or(artifact.latest)
            };
            targets.push((id.clone(), rev));
        }
    } else {
        // Specific artifact or directory
        let matches: Vec<String> = remote_index
            .artifacts
            .iter()
            .filter(|(_, details)| details.path.starts_with(&path_str))
            .map(|(id, _)| id.clone())
            .collect();

        if !matches.is_empty() {
            for id in matches {
                let artifact = &remote_index.artifacts[&id];
                let rev = if let Some(ref commit_hash) = requested_commit {
                    let commit = remote_index
                        .commits
                        .get(commit_hash)
                        .ok_or_else(|| format!("Commit {} not found", commit_hash))?;
                    *commit.artifacts.get(&id).unwrap_or(&artifact.latest)
                } else {
                    requested_rev.unwrap_or(artifact.latest)
                };
                targets.push((id, rev));
            }
        } else if let Some(id) = resolve_artifact_id(&remote_index, &path_str) {
            let artifact = &remote_index.artifacts[&id];
            let rev = if let Some(ref commit_hash) = requested_commit {
                let commit = remote_index
                    .commits
                    .get(commit_hash)
                    .ok_or_else(|| format!("Commit {} not found", commit_hash))?;
                *commit.artifacts.get(&id).unwrap_or(&artifact.latest)
            } else {
                requested_rev.unwrap_or(artifact.latest)
            };
            targets.push((id, rev));
        } else {
            return Err(format!("Path '{}' not found", path_str).into());
        }
    }

    // Pull artifacts
    for (artifact_id, rev) in targets {
        let artifact_details = &remote_index.artifacts[&artifact_id];

        let revision_info = artifact_details
            .revisions
            .iter()
            .find(|r| r.rev == rev)
            .ok_or_else(|| format!("Revision {} not found for {}", rev, artifact_details.path))?;

        println!("-> Pulling {} (rev {})", artifact_details.path, rev);

        // Download
        let ext = Path::new(&artifact_details.path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{}", s))
            .unwrap_or_default();
        let remote_filename = format!("rev{}{}", rev, ext);
        let download_url = format!(
            "{}/api/v1/{}/artifacts/{}/{}",
            server_url, local_index.project, artifact_id, remote_filename
        );

        let mut file_content = client
            .get(&download_url)
            .send()
            .await?
            .bytes()
            .await?
            .to_vec();

        if revision_info.compressed {
            let mut decoder = GzDecoder::new(&file_content[..]);
            let mut decoded_data = Vec::new();
            decoder.read_to_end(&mut decoded_data)?;
            file_content = decoded_data;
        }

        // Local path
        let local_path = if let Some(ref out) = out_arg {
            current_dir.join(out)
        } else if requested_rev.is_some() || requested_commit.is_some() {
            current_dir.join(format!("{}@{}", artifact_details.path, rev))
        } else {
            current_dir.join(&artifact_details.path)
        };

        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        if local_path.exists() {
            let mut perms = fs::metadata(&local_path)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            fs::set_permissions(&local_path, perms)?;
        }

        fs::write(&local_path, &file_content)?;
        let mut perms = fs::metadata(&local_path)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&local_path, perms)?;

        println!("   Saved to {}", local_path.display());
    }

    // Update index if no specific revision/commit/out used
    if requested_rev.is_none() && requested_commit.is_none() && out_arg.is_none() {
        let mut final_index = remote_index;
        final_index.server_url = local_index.server_url;
        final_index.username = local_index.username;
        fs::write(
            &local_index_path,
            serde_json::to_string_pretty(&final_index)?,
        )?;
        println!("   Local index updated.");
    }

    Ok(())
}
