# OttoUpdate

OttoUpdate is the update engine for Otto, responsible for update policies, release orchestration, and generated command execution surfaces.

## Responsibilities
- Resolve and evaluate update manifests.
- Execute update orchestration through generated command surfaces.
- Provide generated CLI/API integration points to the rest of the Otto platform.

## Command Surface Ownership

Otto CLI and API surfaces are generated exclusively from the standalone `otto-command-service` repository.

`otto-update` does not define embedded command schemas or manual command routing logic.

## Structure
- `src/update/` – update domain logic
- `src/generated_cli/` – generated CLI command surface
- `src/generated_api/` – generated API command surface
- `src/main.ts` – generated surface entrypoint
- `docs/` – migration and validation reports
