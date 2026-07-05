# Command Service Extraction Report

Date: 2026-07-05

## Discovery Scope
- Embedded source discovered at: `/Users/dev-macbook/Documents/GitHub/otto-update/otto-command-service`
- Target multi-repo workspace root: `/Users/dev-macbook/Documents/GitHub/Otto`

## Embedded Folder Structure
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
  generators/
    cli-generator/
      generate.mjs
    api-generator/
      generate.mjs
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

## References to otto-update Internals (Current Coupling)
- Generator output is hardwired to `src/generated_cli/index.ts` and `src/generated_api/index.ts` under the embedding repo root.
- Generator input path is hardwired to `otto-command-service/commands` under the embedding repo root.
- Generated API code dynamically imports handlers via:
  - `../../otto-command-service/handlers/${schema.routing.handlerModule}`

This directly couples command definitions and handler loading to the internal folder location of the embedding `otto-update` repository.

## Missing Components for a Proper Standalone Command Service Layer
- Missing stable standalone repository documentation for extraction lineage and integration contract.
- Missing standalone schema/handler/generator folder normalization to `src/schemas`, `src/handlers`, `src/generators/*`.
- Missing dedicated generated surface outputs consumed by other repos via workspace dependency wiring.
- Missing cross-repo import rewiring evidence in `otto-update`, `otto-kernel`, `otto-ui`, `otto-extensions`, and Maestro repos.
- Missing central validation artifact proving all repos consume command schemas from standalone layer.

## Architecture Rule Compliance Assessment
Current state (embedded folder inside `otto-update`) is not independently accessible as the canonical source for all Otto repositories and therefore violates the architecture rule:

> The Command Service Layer must be a standalone repo and the single source of truth for all CLI and API commands.

A standalone extraction and cross-repo rewiring is required for compliance.
