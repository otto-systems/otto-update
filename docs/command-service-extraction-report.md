# Command Service Extraction Report

Date: 2026-07-05

## Status
- This document records the extraction outcome.
- A fresh rescan of the current Otto workspace confirms there is no embedded `otto-command-service` directory inside `otto-update`.
- The active source of truth now lives at `/Users/dev-macbook/Documents/GitHub/Otto/otto-command-service`.

## Extracted Repository Structure
```text
otto-command-service/
  src/
    command/
    schemas/
    handlers/
    generators/
      cli-generator/
      api-generator/
  tests/
  docs/
```

## Historical Embedded Layout
Prior to extraction, the command service logic was maintained as an embedded folder under `otto-update`. The current workspace no longer contains that layout, but the migrated content included:

```text
otto-command-service/
  commands/
    config.set.json
    config.show.json
    service.install.json
    service.start.json
    service.status.json
    service.stop.json
    service.uninstall.json
  handlers/
    configSet.mjs
    configShow.mjs
    serviceInstall.mjs
    serviceShared.mjs
    serviceStart.mjs
    serviceStatus.mjs
    serviceStop.mjs
    serviceUninstall.mjs
```

## Command Schemas Present
- `config.set`
- `config.show`
- `service.install`
- `service.start`
- `service.status`
- `service.stop`
- `service.uninstall`

Each schema includes routing metadata (`handlerModule`, `handlerExport`) and HTTP exposure metadata (`method`, `path`).

## Handlers Present
- Config handlers:
  - `configSet.mjs`
  - `configShow.mjs`
- Service handlers:
  - `serviceInstall.mjs`
  - `serviceStart.mjs`
  - `serviceStatus.mjs`
  - `serviceStop.mjs`
  - `serviceUninstall.mjs`
  - shared utility: `serviceShared.mjs`

## Generator Code Present
- CLI generator: `generators/cli-generator/generate.mjs`
- API generator: `generators/api-generator/generate.mjs`

Both generators are present and operational from the embedded source layout.

## Current Coupling
- Generator output is intentionally emitted into `otto-update/src/generated_cli/index.ts` and `otto-update/src/generated_api/index.ts`.
- Generator input is sourced from `otto-command-service/src/schemas` in the standalone repo.
- Generated API code dynamically imports handlers via:
  - `../../otto-command-service/src/handlers/${schema.routing.handlerModule}`

This keeps generated surfaces in `otto-update` while preserving the standalone command-service repo as the only command definition source.

## Completion Summary
- Standalone schema, handler, and generator structure is in place.
- Dedicated generated CLI/API surfaces are present in `otto-update`.
- Cross-repo wiring has been verified in `otto-update`, `otto-kernel`, `otto-ui`, `otto-extensions`, and `Maestro`.
- Validation artifacts now live alongside this report in `otto-update/docs`.

## Architecture Rule Compliance Assessment
The current state satisfies the architecture rule that the command service layer must remain a standalone repo and the single source of truth for CLI and API commands.

The extraction and rewiring work required for compliance has already been completed in the active workspace.

## Extraction Summary (Executed)
- Created standalone repo source of truth in `Otto/otto-command-service` with normalized structure:
  - `src/schemas`
  - `src/handlers`
  - `src/generators/cli-generator`
  - `src/generators/api-generator`
  - `tests`
  - `docs`
- Added Maestro lifecycle command schemas into standalone source:
  - `maestro.install`
  - `maestro.update`
  - `maestro.repair`
  - `maestro.uninstall`
- Regenerated command surfaces into `otto-update/src/generated_cli/index.ts` and `otto-update/src/generated_api/index.ts` using standalone generators.

## Rewired Imports and Integration Points
- `Otto/otto-update`
  - Added dependency on `@otto/command-service`.
  - Exports generated surfaces from `src/index.ts`.
  - Added `src/main.ts` entrypoint that imports only generated CLI/API surfaces.
- `Otto/otto-kernel`
  - Added dependency on `@otto/command-service`.
  - `Kernel.syncCommandSchemas()` now loads standalone schemas and enforces allowed command registration in router.
- `Otto/otto-ui`
  - Added `src/services/commandRoutingService.ts` with command-service routing source and node-graph command mapping.
  - Dashboard/module/update service models now reference command routing from standalone command layer contract.
- `Otto/otto-extensions`
  - Added dependency on `@otto/command-service`.
  - Example module exposes `getExtensionCommandSchemas()` sourced from standalone schemas.
- `Maestro`
  - Added dependency on `@otto/command-service`.
  - Added `src/shared/utils/commandServiceContract.ts` to validate required Maestro commands exist in standalone schemas.
  - Rewired `otto/maestro-commands.json` and `otto/maestro-payload.json` to declare standalone command-service as source of truth.

## Deleted Files
- Removed standalone Rust CLI package from `Otto/otto-update/ottoupdate/ottoupdate-cli`.
- Updated `Otto/otto-update/ottoupdate/Cargo.toml` workspace members to remove `ottoupdate-cli`.

## New Architecture
- The standalone `Otto/otto-command-service` repository is now the single source of truth for command schemas, handler routing, and generator inputs.
- All generated CLI/API surfaces in `otto-update` are emitted from that standalone source.
- Otto repos and Maestro now reference command contracts through the standalone layer rather than embedded command definitions.
