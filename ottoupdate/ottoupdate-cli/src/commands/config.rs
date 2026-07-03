use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct ConfigCommand {
    #[arg(long, default_value = "./config/server.toml")]
    pub path: PathBuf,

    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    Show,
    Set {
        #[arg(long)]
        bind: Option<String>,
        #[arg(long)]
        bearer_token: Option<String>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ServerConfig {
    bind: String,
    bearer_token: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:7430".to_string(),
            bearer_token: None,
        }
    }
}

pub fn run(command: ConfigCommand) -> Result<()> {
    match command.action {
        ConfigAction::Show => show(command.path),
        ConfigAction::Set { bind, bearer_token } => set(command.path, bind, bearer_token),
    }
}

fn show(path: PathBuf) -> Result<()> {
    let cfg = load_or_default(&path)?;
    println!("{}", toml::to_string_pretty(&cfg)?);
    Ok(())
}

fn set(path: PathBuf, bind: Option<String>, bearer_token: Option<String>) -> Result<()> {
    let mut cfg = load_or_default(&path)?;
    if let Some(bind) = bind {
        cfg.bind = bind;
    }
    if bearer_token.is_some() {
        cfg.bearer_token = bearer_token;
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    std::fs::write(&path, toml::to_string_pretty(&cfg)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    println!("updated {}", path.display());
    Ok(())
}

fn load_or_default(path: &PathBuf) -> Result<ServerConfig> {
    if path.exists() {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        return toml::from_str(&raw)
            .with_context(|| format!("failed to parse {}", path.display()));
    }

    Ok(ServerConfig::default())
}
