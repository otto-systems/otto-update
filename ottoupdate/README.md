# OttoUpdate Rust Workspace

This directory contains the Rust implementation scaffolding for OttoUpdate.

## Workspace Layout

- `ottoupdate-core` - Library crate for update business logic and state management.
- `ottoupdate-server` - Binary crate for the Axum HTTP server.
- `ottoupdate-client` - Library crate for async HTTP client integrations.
- `ottoupdate-cli` - Binary crate for administrative command-line operations.

## Notes

- Shared dependencies are centralized in the workspace `Cargo.toml`.
- Incremental compilation is enabled for dev builds in `.cargo/config.toml`.
- The workspace default build target is set to the current host triple.
