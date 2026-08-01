# Otto Orchestration Starter

This folder bootstraps a multi-repo Otto workspace and gives agents a shared execution contract for parallel work.

## What This Implements Now

- Repository access preflight for required Otto repos.
- Repo clone and sync flow into one canonical workspace folder.
- Parallel validation matrix for TypeScript and Rust checks.
- Agent lane definitions and context-intake template.

## Quick Start

1. Edit `config/repos.json` and set `defaultOwner` (and clone URLs if needed).
2. Run preflight:

   `powershell -NoProfile -ExecutionPolicy Bypass -File .\\scripts\\01-preflight-access.ps1`

3. Sync repositories:

   `powershell -NoProfile -ExecutionPolicy Bypass -File .\\scripts\\02-sync-repos.ps1`

4. Run validation matrix:

   `powershell -NoProfile -ExecutionPolicy Bypass -File .\\scripts\\03-run-matrix.ps1`

5. Or run all phases:

   `powershell -NoProfile -ExecutionPolicy Bypass -File .\\scripts\\00-bootstrap.ps1`

6. If `git` is not in PATH, pass `-GitPath`:

   `powershell -NoProfile -ExecutionPolicy Bypass -File .\\scripts\\00-bootstrap.ps1 -GitPath "C:\\Program Files\\Git\\cmd\\git.exe"`

## Notes

- Default workspace root is a sibling folder named `otto-workspace` under the parent of this repo.
- Logs are written to `logs/`.
- Matrix commands are defined in `config/matrix.json`.
