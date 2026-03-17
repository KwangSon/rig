use crate::repository::Repository;

pub async fn run(branch_name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let repo = Repository::open(&current_dir)?;

    if let Some(name) = branch_name {
        // Create new branch
        let head_hash = match repo.head_commit()? {
            Some(hash) => hash,
            None => {
                return Err("Cannot create a branch from an empty repository (no commits).".into());
            }
        };

        repo.write_ref(&format!("refs/heads/{}", name), &head_hash)?;
        println!("Created branch '{}'", name);
    } else {
        // List branches
        let branches = repo.list_branches()?;
        let head = repo.read_head().unwrap_or_default();
        let current_branch = head.strip_prefix("ref: refs/heads/").unwrap_or("");

        for branch in branches {
            if branch == current_branch {
                println!("* {}", branch);
            } else {
                println!("  {}", branch);
            }
        }
    }

    Ok(())
}
