use crate::credentials::CredentialStore;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize)]
struct CreateSessionResponse {
    session_id: uuid::Uuid,
}

#[derive(Deserialize)]
struct SessionStatusResponse {
    status: String,
    token: Option<String>,
}

pub async fn ensure_authenticated(base_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let store = CredentialStore::new();
    let trimmed_base = base_url.trim_end_matches('/');

    if let Some(token) = store.get_token(trimmed_base) {
        return Ok(token);
    }

    println!("Authentication required for {}", trimmed_base);

    let client = reqwest::Client::new();

    // 1. Create a session
    let res = client
        .post(format!("{}/api/v1/auth/session", trimmed_base))
        .send()
        .await?
        .json::<CreateSessionResponse>()
        .await?;

    let session_id = res.session_id;
    let login_url = format!("{}/login?cli_session={}", trimmed_base, session_id);

    println!("Please log in via your browser:");
    println!("{}", login_url);

    // Attempt to open the browser
    if let Err(e) = open::that(&login_url) {
        eprintln!("Failed to open browser automatically: {}", e);
        println!("Please copy and paste the URL above into your browser.");
    }

    // 2. Poll for token
    println!("Waiting for login...");
    loop {
        let status = client
            .get(format!(
                "{}/api/v1/auth/token?session_id={}",
                trimmed_base, session_id
            ))
            .send()
            .await?
            .json::<SessionStatusResponse>()
            .await?;

        if status.status == "success" {
            if let Some(token) = status.token {
                println!("Login successful!");
                store.set_token(trimmed_base, &token)?;
                return Ok(token);
            }
        } else if status.status == "expired" {
            return Err("CLI session expired. Please try again.".into());
        }

        sleep(Duration::from_secs(2)).await;
    }
}
