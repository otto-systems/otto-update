# OttoUpdate

OttoUpdate is the update engine for Otto, responsible for managing versions, channels, deferrals, rollbacks, and safety policies.

## Responsibilities
- Define and enforce update channels
- Manage deferrals and scheduled updates
- Perform safety checks and rollbacks
- Expose update commands to the Command Service Layer

## Otto CLI Generation Model
Otto CLI is generated exclusively from the Command Service Layer.

- Command definitions are declared in `otto-command-service/commands/`.
- Command logic is implemented in `otto-command-service/handlers/`.
- Generated CLI surface is emitted to `src/generated_cli/`.
- Generated API surface is emitted to `src/generated_api/`.
- Runtime wiring in `src/main.ts` uses generated command surfaces only.

## Planned Structure
- `src/core/` – Rust core service
- `src/generated_cli/` – generated CLI command surface
- `src/generated_api/` – generated API command surface
- `docs/` – design and OpenAPI specs
- `prompts/` – Copilot prompt packs (added later)
