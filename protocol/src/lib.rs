use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Permission {
    pub user_id: Uuid,
    pub project: String,
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
pub struct Commit {
    pub hash: String,
    pub parent: Option<String>,
    pub message: String,
    pub author: String,
    pub artifacts: HashMap<String, u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitModule {
    pub path: String,
    pub url: String,
    pub commit: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexFile {
    pub project: String,
    pub server_url: Option<String>,
    pub username: Option<String>,
    pub latest_commit: String,
    pub artifacts: HashMap<String, Artifact>,
    #[serde(default)]
    pub git_modules: HashMap<String, GitModule>,
    pub commits: HashMap<String, Commit>,
}
