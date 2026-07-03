mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ottoupdate-cli", version, about = "OttoUpdate administrative CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Config(commands::config::ConfigCommand),
    Service(commands::service::ServiceCommand),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config(config) => commands::config::run(config),
        Commands::Service(service) => commands::service::run(service.action),
    }
}
