use anyhow::{Context, Result};
use reqwest::Client;

pub struct QbtClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
    authenticated: bool,
}

impl QbtClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        let client = Client::builder()
            .cookie_store(true)
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
            authenticated: false,
        }
    }

    async fn login(&mut self) -> Result<()> {
        let resp = self
            .client
            .post(format!("{}/api/v2/auth/login", self.base_url))
            .form(&[
                ("username", self.username.as_str()),
                ("password", self.password.as_str()),
            ])
            .send()
            .await
            .context("login request failed")?;

        let body = resp.text().await?;
        if body.contains("Ok") {
            self.authenticated = true;
            Ok(())
        } else {
            anyhow::bail!("qBittorrent login failed: {body}")
        }
    }

    pub async fn set_listen_port(&mut self, port: u16) -> Result<()> {
        if !self.authenticated {
            self.login().await?;
        }

        let json_val = format!(r#"{{"listen_port":{port}}}"#);

        let resp = self
            .client
            .post(format!("{}/api/v2/app/setPreferences", self.base_url))
            .form(&[("json", json_val.as_str())])
            .send()
            .await
            .context("failed to set preferences")?;

        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("setPreferences returned {}", resp.status())
        }
    }
}
