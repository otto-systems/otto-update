# OttoUpdate CLI Architecture Violation Report

## Architecture Rule
All CLI and API commands must be defined in the Command Service Layer and generated from it.

## Violation Summary
The repository contains a standalone Rust CLI crate (`ottoupdate-cli`) with direct argument parsing, command routing, and command handlers implemented outside a command service definition/generation flow.

## Directory and Module Violations
- `ottoupdate/ottoupdate-cli`
  - Standalone binary crate that defines and executes CLI commands directly.
- `ottoupdate/ottoupdate-cli/src/commands`
  - Manual command modules (`config`, `service`) containing direct handlers.

## Parser, Routing, and Handler Violations

### 1) Top-level parser and routing
- File: `ottoupdate/ottoupdate-cli/src/main.rs`
- Parser implementation:
  - `use clap::{Parser, Subcommand};`
  - `#[derive(Parser)] struct Cli`
  - `#[derive(Subcommand)] enum Commands`
  - `let cli = Cli::parse();`
- Routing implementation:
  - `match cli.command { ... }`
- Why this violates the rule:
  - Command shape and routing are authored in a standalone CLI binary rather than generated from command service schemas.

### 2) Config parser, routing, and handlers
- File: `ottoupdate/ottoupdate-cli/src/commands/config.rs`
- Parser implementation:
  - `#[derive(Parser)] pub struct ConfigCommand`
  - `#[derive(Subcommand)] pub enum ConfigAction`
- Routing implementation:
  - `pub fn run(command: ConfigCommand) -> Result<()>`
  - `match command.action { ... }`
- Handler functions:
  - `fn show(path: PathBuf) -> Result<()>`
  - `fn set(path: PathBuf, bind: Option<String>, bearer_token: Option<String>) -> Result<()>`
  - `fn load_or_default(path: &PathBuf) -> Result<ServerConfig>`
- Why this violates the rule:
  - Command parsing and execution logic is implemented manually in CLI modules, not defined as command service schemas/handlers with generated CLI and API surfaces.

### 3) Service parser, routing, and handlers
- File: `ottoupdate/ottoupdate-cli/src/commands/service.rs`
- Parser implementation:
  - `#[derive(Parser)] pub struct ServiceCommand`
  - `#[derive(Subcommand)] pub enum ServiceAction`
- Routing implementation:
  - `pub fn run(action: ServiceAction) -> Result<()>`
  - `match action { ... }`
- Handler functions:
  - `fn run_sc<const N: usize>(args: [&str; N]) -> Result<()>` (Windows)
  - `fn run_sc<const N: usize>(_args: [&str; N]) -> Result<()>` (non-Windows)
- Additional direct process execution:
  - `Command::new("sc")...`
- Why this violates the rule:
  - Manual command actions and process orchestration are embedded in standalone CLI handlers instead of command service handlers exposed through generated surfaces.

### 4) Manual command module wiring
- File: `ottoupdate/ottoupdate-cli/src/commands/mod.rs`
- Manual routing surface:
  - `pub mod config;`
  - `pub mod service;`
- Why this violates the rule:
  - Command registration is done manually in CLI code instead of being generated from command service metadata.

## Scope of Required Migration
To satisfy the architecture rule, all command definitions and logic currently in `ottoupdate-cli` must move into a command service layer, then regenerate CLI/API surfaces from that layer.
