# Changelog

## Unreleased

### Changed
- Extracted command service layer into standalone repo and rewired all Otto repos to use it.
- Refreshed migration reports, workspace cleanup evidence, and validation records for the standalone command-service wiring.

## 0.2.5 - 2026-08-01
- Bugfix release: bumped Otto update package and Rust crate versions to 0.2.5, aligned the installer workflow with CI-owned Linux/macOS artifacts, and cleaned generated report noise from source control.

## 0.2.4 - 2026-07-05
- Bugfix release: synchronized git state and bumped project/version manifests to 0.2.4.

## 0.2.2 - 2026-07-05
- Removed standalone `ottoupdate-cli` implementation and migrated all commands to the Command Service Layer.
- Added command schemas and handlers under `otto-command-service`.
- Added generated CLI/API surfaces under `src/generated_cli` and `src/generated_api`.
