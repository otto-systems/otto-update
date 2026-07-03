use std::net::SocketAddr;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub bearer_token: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:7430".to_string(),
            bearer_token: None,
        }
    }
}

impl ServerConfig {
    pub async fn load_or_default(config_path: &Path) -> Result<Self> {
        if let Ok(raw) = tokio::fs::read_to_string(config_path).await {
            let from_file: ServerConfig = toml::from_str(&raw)
                .with_context(|| format!("failed to parse {}", config_path.display()))?;
            return Ok(from_file.with_env_overrides());
        }

        Ok(Self::default().with_env_overrides())
    }

    pub fn socket_addr(&self) -> Result<SocketAddr> {
        self.bind
            .parse::<SocketAddr>()
            .with_context(|| format!("invalid bind address: {}", self.bind))
    }

    fn with_env_overrides(mut self) -> Self {
        if let Ok(bind) = std::env::var("OTTOUPDATE_BIND") {
            self.bind = bind;
        }

        if let Ok(token) = std::env::var("OTTOUPDATE_BEARER_TOKEN") {
            self.bearer_token = Some(token);
        }

        self
    }
}
