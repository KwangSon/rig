use crate::auth::ensure_authenticated;
use crate::repository::Repository;
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

    let token = match ensure_authenticated(server_url).await {
        Ok(t) => t,
        Err(e) => return Err(format!("Authentication failed: {}", e).into()),
    };

    let remote_index_url = format!("{}/api/v1/{}/index", server_url, config.project_key());
    let remote_resp = client
        .get(&remote_index_url)
        .header("authorization", format!("Bearer {}", token))
        .send()
        .await?;
    let remote_text = remote_resp.text().await?;
    let remote_index: IndexFile = serde_json::from_str(&remote_text)?;

    // Resolution helpers

    // Determine artifacts and their target revisions
    let mut targets: Vec<(String, String, u32)> = Vec::new(); // (path, id, rev)

    if path_str == "*" {
        // All artifacts
        for (path, artifact) in &remote_index.artifacts {
            let rev = if let Some(ref commit_hash) = requested_commit {
                let commit = remote_index
                    .commits
                    .get(commit_hash)
                    .ok_or_else(|| format!("Commit {} not found", commit_hash))?;
                commit
                    .artifacts
                    .iter()
                    .find(|a| a.artifact_id == artifact.id)
                    .map(|a| a.revision_base)
                    .unwrap_or(artifact.latest)
            } else {
                requested_rev.unwrap_or(artifact.latest)
            };
            targets.push((path.clone(), artifact.id.clone(), rev));
        }
    } else {
        // Find matches in remote index (keyed by ID in IndexFile, wait!)
        // Wait, protocol::IndexFile.artifacts is HashMap<ID, Artifact>
        let mut matches: Vec<(String, String)> = remote_index
            .artifacts
            .iter()
            .filter(|(_, a)| a.path.starts_with(&path_str))
            .map(|(id, a)| (a.path.clone(), id.clone()))
            .collect();

        if matches.is_empty() {
            // Check local index (keyed by Path)
            matches = local_index
                .artifacts
                .iter()
                .filter(|(path, _)| path.starts_with(&path_str))
                .map(|(path, a)| (path.clone(), a.artifact_id.clone()))
                .collect();
        }

        if !matches.is_empty() {
            for (path, id) in matches {
                let latest_rev = remote_index
                    .artifacts
                    .get(&id)
                    .map(|a| a.latest)
                    .or_else(|| local_index.artifacts.get(&path).map(|a| a.revision))
                    .unwrap_or(0);

                let rev = if let Some(ref commit_hash) = requested_commit {
                    let commit = remote_index
                        .commits
                        .get(commit_hash)
                        .ok_or_else(|| format!("Commit {} not found", commit_hash))?;
                    commit
                        .artifacts
                        .iter()
                        .find(|a| a.artifact_id == id)
                        .map(|a| a.revision_base)
                        .unwrap_or(latest_rev)
                } else {
                    requested_rev.unwrap_or(latest_rev)
                };
                targets.push((path, id, rev));
            }
        } else {
            return Err(format!("Path '{}' not found", path_str).into());
        }
    }

    // Pull artifacts
    let mut mut_local_index = local_index;

    for (path, artifact_id, rev) in &targets {
        if requested_rev.is_none() && requested_commit.is_none() && out_arg.is_none() {
            if let Some(local_art) = mut_local_index.artifacts.get(path)
                && local_art.locked
                && local_art.lock_owner == config.username
            {
                return Err(format!(
                    "ERROR: File '{}' is locked by you. Push or unlock before pulling to avoid losing local changes.",
                    path
                )
                .into());
            }
        }

        let artifact_details = remote_index
            .artifacts
            .get(artifact_id)
            .ok_or_else(|| format!("Artifact details for {} not found on server", path))?;

        let revision_info = artifact_details
            .revisions
            .iter()
            .find(|r| r.rev == *rev)
            .ok_or_else(|| format!("Revision {} not found for {}", rev, path))?;

        let is_compressed = revision_info.compressed;

        println!(
            "-> Pulling {} (rev {}) (compressed={})",
            path, rev, is_compressed
        );

        // Download
        let ext = Path::new(&path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{}", s))
            .unwrap_or_default();
        let remote_filename = format!("rev{}{}", rev, ext);
        let download_url = format!(
            "{}/api/v1/{}/artifacts/{}/{}",
            server_url,
            config.project_key(),
            artifact_id,
            remote_filename
        );

        let mut file_content = client
            .get(&download_url)
            .header("authorization", format!("Bearer {}", token))
            .send()
            .await?
            .bytes()
            .await?
            .to_vec();

        if is_compressed {
            let mut decoder = GzDecoder::new(&file_content[..]);
            let mut decoded_data = Vec::new();
            decoder.read_to_end(&mut decoded_data)?;
            file_content = decoded_data;
        }

        // Local path
        let local_path = if let Some(ref out) = out_arg {
            if path_str == "*" || out.is_dir() || targets.len() > 1 {
                current_dir.join(out).join(&path)
            } else {
                current_dir.join(out)
            }
        } else if requested_rev.is_some() || requested_commit.is_some() {
            current_dir.join(format!("{}@{}", path, rev))
        } else {
            current_dir.join(&path)
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

        // Update local state if it's a standard pull
        if requested_rev.is_none() && requested_commit.is_none() && out_arg.is_none() {
            if let Some(local_art) = mut_local_index.artifacts.get_mut(path) {
                local_art.local_state = "ready".to_string();
                local_art.revision = *rev;
            } else {
                mut_local_index.artifacts.insert(
                    path.clone(),
                    protocol::IndexArtifact {
                        artifact_id: artifact_id.clone(),
                        revision: *rev,
                        local_state: "ready".to_string(),
                        stage: "none".to_string(),
                        locked: false,
                        lock_owner: None,
                        lock_generation: None,
                        staged: None,
                        moved_from: None,
                    },
                );
            }
        }
    }

    // Update index and commits
    if requested_rev.is_none() && requested_commit.is_none() && out_arg.is_none() {
        mut_local_index.git_modules = remote_index.git_modules;
        repo.write_index(&mut_local_index)?;

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
