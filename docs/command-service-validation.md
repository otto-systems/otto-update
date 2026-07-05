# Command Service Validation

Date: 2026-07-05

## Validation Scope
- Standalone command-service extraction
- Cross-repo rewiring (`otto-update`, `otto-kernel`, `otto-ui`, `otto-extensions`, `Maestro`)
- CLI/API surface regeneration in `otto-update`
- Bootstrap dry-run execution

## Commands Executed

### `otto-command-service`
- `npm install`
- `npm run typecheck`
- `npm test`

Result: pass

### `otto-update`
- `npm install`
- `npm run typecheck`
- `npm test`
- `npm run build`

Result: pass

### `otto-kernel`
- `npm install`
- `npm run typecheck`
- `npm test`

Result: pass

### `otto-ui`
- `npm install`
- `npm run typecheck`
- `npm test`

Result: pass

### `otto-extensions`
- `npm install`
- `npm run typecheck`
- `npm test`

Result: pass

### `Maestro`
- `pnpm install`
- `pnpm typecheck`
- `pnpm test`
- `pnpm validate:command-service`

Result: pass

## Bootstrap Dry-Run
- `OTTO_DRY_RUN=1 ./bootstrap.sh`
- Executed in `/Users/dev-macbook/Documents/GitHub/otto-update` because the Otto workspace copy at `/Users/dev-macbook/Documents/GitHub/Otto/otto-update` does not include `bootstrap.sh`.

Result: completed successfully with expected non-Windows service operation warnings (`service.* is only supported on Windows`).

## Required Confirmations

### No CLI code exists outside generated CLI
- `otto-update/src` CLI/command file scan returns only:
  - `src/generated_cli/index.ts`
- `ottoupdate/ottoupdate-cli` directory removed from Otto workspace `otto-update` repo.

### All commands resolve through standalone command-service layer
- Generated API handler resolution path:
  - `../../otto-command-service/src/handlers/${schema.routing.handlerModule}`
- Command schema source of truth:
  - `otto-command-service/src/schemas/*.json`

### `otto-update` builds and runs correctly
- `npm run build` passes.
- Typecheck and tests pass.

### Maestro installer commands resolve through command-service
- Standalone schemas include:
  - `maestro.install`
  - `maestro.update`
  - `maestro.repair`
  - `maestro.uninstall`
- `pnpm validate:command-service` passes in Maestro.

## Notes
- Validation uncovered a kernel typing gap after introducing command-service imports; resolved by adding Node typings in `otto-kernel`.
- Validation artifacts (`tsbuildinfo`, generated config JS/DTS in `otto-ui`) were removed and not committed.
