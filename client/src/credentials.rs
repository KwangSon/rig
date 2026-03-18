use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Credentials {
    // Maps host URL to token
    pub host_tokens: HashMap<String, String>,
}

pub struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    pub fn new() -> Self {
        #[cfg(windows)]
        let config_base = dirs::config_dir().expect("Could not find config directory");
        #[cfg(not(windows))]
        let config_base = dirs::home_dir()
            .expect("Could not find home directory")
            .join(".config");

        let mut path = config_base.join("rig");
        if !path.exists() {
            fs::create_dir_all(&path).expect("Could not create rig config directory");
        }
        path.push("credentials");
        Self { path }
    }

    pub fn load(&self) -> Credentials {
        if !self.path.exists() {
            return Credentials::default();
        }

        let content = fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self, credentials: &Credentials) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(credentials)?;

        // Create file if it doesn't exist to set permissions
        if !self.path.exists() {
            fs::File::create(&self.path)?;
        }

        // Set permissions to 600 (read/write by owner only)
        let mut perms = fs::metadata(&self.path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&self.path, perms)?;

        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        file.write_all(content.as_bytes())?;
        Ok(())
    }

    pub fn get_token(&self, host: &str) -> Option<String> {
        let creds = self.load();
        creds.host_tokens.get(host).cloned()
    }

    pub fn set_token(&self, host: &str, token: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut creds = self.load();
        creds
            .host_tokens
            .insert(host.to_string(), token.to_string());
        self.save(&creds)
    }
}
