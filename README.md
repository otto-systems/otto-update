# OttoUpdate

OttoUpdate is the update engine for Otto, responsible for managing versions, channels, deferrals, rollbacks, and safety policies.

## Responsibilities
- Define and enforce update channels
- Manage deferrals and scheduled updates
- Perform safety checks and rollbacks
- Expose update commands to the Command Service Layer

## Planned Structure
- `src/core/` – Rust core service
- `src/cli/` – CLI interface
- `docs/` – design and OpenAPI specs
- `prompts/` – Copilot prompt packs (added later)
