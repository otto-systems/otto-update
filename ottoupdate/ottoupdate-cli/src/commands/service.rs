use std::process::Command;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct ServiceCommand {
    #[command(subcommand)]
    pub action: ServiceAction,
}

#[derive(Debug, Subcommand)]
pub enum ServiceAction {
    Install,
    Start,
    Stop,
    Status,
    Uninstall,
}

pub fn run(action: ServiceAction) -> Result<()> {
    match action {
        ServiceAction::Install => run_sc(["create", "OttoUpdate", "binPath=", "\"C:\\Program Files\\OttoUpdate\\ottoupdate-server.exe\""] as [&str; 4]),
        ServiceAction::Start => run_sc(["start", "OttoUpdate"]),
        ServiceAction::Stop => run_sc(["stop", "OttoUpdate"]),
        ServiceAction::Status => run_sc(["query", "OttoUpdate"]),
        ServiceAction::Uninstall => run_sc(["delete", "OttoUpdate"]),
    }
}

#[cfg(target_os = "windows")]
fn run_sc<const N: usize>(args: [&str; N]) -> Result<()> {
    let status = Command::new("sc")
        .args(args)
        .status()
        .map_err(|e| anyhow!("failed invoking sc: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("service command failed with status {status}"))
    }
}

#[cfg(not(target_os = "windows"))]
fn run_sc<const N: usize>(_args: [&str; N]) -> Result<()> {
    Err(anyhow!("service subcommands are only supported on Windows"))
}
