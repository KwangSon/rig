pub use protocol::{Artifact, Commit, GitModule, Index};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Config {
    pub project: String,
    pub server_url: Option<String>,
    pub username: Option<String>,
}

pub struct Repository {
    pub rig_dir: PathBuf,
}

impl Repository {
    pub fn init(base_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let rig_dir = base_path.join(".rig");
        if rig_dir.exists() {
            return Err("Repository already initialized".into());
        }

        fs::create_dir_all(&rig_dir)?;
        fs::create_dir_all(rig_dir.join("refs").join("heads"))?;
        fs::create_dir_all(rig_dir.join("objects"))?;

        // Initialize HEAD to refs/heads/main
        fs::write(rig_dir.join("HEAD"), "ref: refs/heads/main\n")?;

        // Initialize empty config and index
        let repo = Repository { rig_dir };
        repo.write_config(&Config::default())?;
        repo.write_index(&Index::default())?;

        Ok(repo)
    }

    pub fn open(base_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let rig_dir = base_path.join(".rig");
        if !rig_dir.exists() {
            return Err("Not a rig repository (no .rig directory found)".into());
        }
        Ok(Repository { rig_dir })
    }

    // --- Config ---

    pub fn read_config(&self) -> Result<Config, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(self.rig_dir.join("config"))?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn write_config(&self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(config)?;
        fs::write(self.rig_dir.join("config"), content)?;
        Ok(())
    }

    // --- Index ---

    pub fn read_index(&self) -> Result<Index, Box<dyn std::error::Error>> {
        let path = self.rig_dir.join("index");
        if !path.exists() {
            return Ok(Index::default());
        }
        let content = fs::read_to_string(&path)?;
        let index: Index = serde_json::from_str(&content)?;
        Ok(index)
    }

    pub fn write_index(&self, index: &Index) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(index)?;
        fs::write(self.rig_dir.join("index"), content)?;
        Ok(())
    }

    // --- HEAD and Refs ---

    pub fn read_head(&self) -> Result<String, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(self.rig_dir.join("HEAD"))?;
        Ok(content.trim().to_string())
    }

    pub fn write_head(&self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(self.rig_dir.join("HEAD"), format!("{}\n", content))?;
        Ok(())
    }

    /// Resolves HEAD to a commit hash.
    /// If HEAD points to a ref, resolves the ref.
    /// If HEAD is a detached hash, returns it directly.
    pub fn head_commit(&self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let head = self.read_head()?;
        if let Some(ref_path) = head.strip_prefix("ref: ") {
            self.resolve_ref(ref_path)
        } else {
            // Detached HEAD
            Ok(Some(head))
        }
    }

    pub fn resolve_ref(
        &self,
        ref_path: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let path = self.rig_dir.join(ref_path);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        Ok(Some(content.trim().to_string()))
    }

    pub fn write_ref(&self, ref_path: &str, hash: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.rig_dir.join(ref_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, format!("{}\n", hash))?;
        Ok(())
    }

    pub fn list_branches(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let heads_dir = self.rig_dir.join("refs").join("heads");
        if !heads_dir.exists() {
            return Ok(vec![]);
        }
        let mut branches = Vec::new();
        for entry in fs::read_dir(heads_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                branches.push(name.to_string());
            }
        }
        branches.sort();
        Ok(branches)
    }

    // --- Objects (Commits) ---

    pub fn write_commit(&self, commit: &Commit) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(commit)?;
        let path = self.rig_dir.join("objects").join(&commit.id);
        fs::write(path, content)?;
        Ok(())
    }

    pub fn read_commit(&self, hash: &str) -> Result<Option<Commit>, Box<dyn std::error::Error>> {
        let path = self.rig_dir.join("objects").join(hash);
        if !path.exists() {
            // Legacy fallback: try .json
            let legacy_path = self.rig_dir.join("objects").join(format!("{}.json", hash));
            if legacy_path.exists() {
                let content = fs::read_to_string(legacy_path)?;
                let commit: Commit = serde_json::from_str(&content)?;
                return Ok(Some(commit));
            }
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        let commit: Commit = serde_json::from_str(&content)?;
        Ok(Some(commit))
    }

    /// Read all commits (useful for some sync commands)
    pub fn read_all_commits(&self) -> Result<HashMap<String, Commit>, Box<dyn std::error::Error>> {
        let objects_dir = self.rig_dir.join("objects");
        let mut commits = HashMap::new();
        if !objects_dir.exists() {
            return Ok(commits);
        }
        for entry in fs::read_dir(objects_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path)?;
                if let Ok(commit) = serde_json::from_str::<Commit>(&content) {
                    commits.insert(commit.id.clone(), commit);
                }
            }
        }
        Ok(commits)
    }
}
