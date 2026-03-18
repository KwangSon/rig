use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct Permission {
    pub user_id: Uuid,
    pub project_id: Uuid,
    pub access: String, // "read", "write", "admin"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Revision {
    pub rev: u32,
    pub hash: String,
    #[serde(default)]
    pub compressed: bool,
}

impl Revision {
    pub fn new(rev: u32, content: &[u8], compressed: bool) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(content);
        let hash = format!("{:x}", hasher.finalize());
        Revision {
            rev,
            hash,
            compressed,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Artifact {
    pub id: String,
    pub path: String,
    pub latest: u32,
    pub locked_by: Option<String>,
    pub revisions: Vec<Revision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moved_from: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommitArtifact {
    pub path: String,
    pub artifact_id: String,
    pub revision_base: u32,
    pub hash: String,
    pub op: String, // "upsert" or "delete"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Commit {
    pub id: String,
    pub parent: Option<String>,
    pub message: String,
    pub author: String,
    pub artifacts: Vec<CommitArtifact>,
    #[serde(default)]
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitModule {
    pub path: String,
    pub url: String,
    pub commit: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StagedInfo {
    pub mtime: u64,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexArtifact {
    pub artifact_id: String,
    pub revision: u32,
    pub local_state: String, // "placeholder" or "ready"
    pub stage: String,       // "none" or "staged"
    pub locked: bool,
    pub lock_owner: Option<String>,
    pub lock_generation: Option<String>,
    pub staged: Option<StagedInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moved_from: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Index {
    pub version: u32,
    pub branch: String,
    pub head: Option<String>,
    pub artifacts: HashMap<String, IndexArtifact>,
    #[serde(default)]
    pub git_modules: HashMap<String, GitModule>,
}

impl Default for Index {
    fn default() -> Self {
        Index {
            version: 1,
            branch: "main".to_string(),
            head: None,
            artifacts: HashMap::new(),
            git_modules: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexFile {
    pub project: String,
    pub server_url: Option<String>,
    pub username: Option<String>,
    pub latest_commit: String, // Kept for legacy compatibility
    #[serde(default)]
    pub refs: HashMap<String, String>, // branch_path -> commit_hash (e.g. "refs/heads/main" -> "abc1234")
    pub artifacts: HashMap<String, Artifact>,
    #[serde(default)]
    pub git_modules: HashMap<String, GitModule>,
    pub commits: HashMap<String, Commit>,
}
