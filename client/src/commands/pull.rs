use crate::repository::{Index, Repository};
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
        if let Some(stripped) = r.strip_prefix('#') {
            if let Ok(rev) = stripped.parse::<u32>() {
                requested_rev = Some(rev);
            }
        } else if let Some(stripped) = r.strip_prefix('@') {
            requested_commit = Some(stripped.to_string());
        } else if let Ok(rev) = r.parse::<u32>() {
            requested_rev = Some(rev);
        }
    }

    // Determine the current project root (where .rig is located)
    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;

    let local_index = repo.read_index()?;
    let config = repo.read_config()?;

    // Fetch latest index from server
    let client = reqwest::Client::new();
    let server_url = config
        .server_url
        .as_deref()
        .unwrap_or("http://localhost:3000");
    let remote_index_url = format!("{}/api/v1/{}/index.json", server_url, config.project);
    let remote_resp = client.get(&remote_index_url).send().await?;
    let remote_index: IndexFile = serde_json::from_str(&remote_resp.text().await?)?;

    // Resolution helpers
    fn resolve_artifact_id(index: &IndexFile, local_index: &Index, query: &str) -> Option<String> {
        if index.artifacts.contains_key(query) {
            return Some(query.to_string());
        }
        if let Some(id) = index
            .artifacts
            .iter()
            .find(|(_, details)| details.path == query)
            .map(|(id, _)| id.clone())
        {
            return Some(id);
        }
        // Fallback to local index for resolution
        if local_index.artifacts.contains_key(query) {
            return Some(query.to_string());
        }
        local_index
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
        let mut matches: Vec<String> = remote_index
            .artifacts
            .iter()
            .filter(|(_, details)| details.path.starts_with(&path_str))
            .map(|(id, _)| id.clone())
            .collect();

        // Also check local index for matches in case the file hasn't been fetched fully
        if matches.is_empty() {
            matches = local_index
                .artifacts
                .iter()
                .filter(|(_, details)| details.path.starts_with(&path_str))
                .map(|(id, _)| id.clone())
                .collect();
        }

        if !matches.is_empty() {
            for id in matches {
                let artifact = remote_index
                    .artifacts
                    .get(&id)
                    .or_else(|| local_index.artifacts.get(&id))
                    .unwrap();
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
        } else if let Some(id) = resolve_artifact_id(&remote_index, &local_index, &path_str) {
            let artifact = remote_index
                .artifacts
                .get(&id)
                .or_else(|| local_index.artifacts.get(&id))
                .unwrap();
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
            println!("[debug] Pull failed. Path searched: '{}'", path_str);
            println!("[debug] Remote Artifacts:");
            for (id, artifact) in &remote_index.artifacts {
                println!("  ID: {}, Path: '{}'", id, artifact.path);
            }
            println!("[debug] Local Artifacts:");
            for (id, artifact) in &local_index.artifacts {
                println!("  ID: {}, Path: '{}'", id, artifact.path);
            }
            return Err(format!("Path '{}' not found", path_str).into());
        }
    }

    // Pull artifacts
    for (artifact_id, rev) in targets {
        let artifact_details = remote_index
            .artifacts
            .get(&artifact_id)
            .or_else(|| local_index.artifacts.get(&artifact_id))
            .unwrap();

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
            server_url, config.project, artifact_id, remote_filename
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
        let mut final_index = local_index;
        final_index.artifacts = remote_index.artifacts;
        final_index.git_modules = remote_index.git_modules;
        repo.write_index(&final_index)?;

        for (_, commit) in remote_index.commits {
            repo.write_commit(&commit)?;
        }

        for (ref_name, hash) in &remote_index.refs {
            repo.write_ref(ref_name, hash)?;
        }
        if remote_index.refs.is_empty() && !remote_index.latest_commit.is_empty() {
            repo.write_ref("refs/heads/main", &remote_index.latest_commit)?;
        }

        println!("   Local repository and index updated.");
    }

    Ok(())
}
