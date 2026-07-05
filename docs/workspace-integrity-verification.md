# Workspace Integrity Verification

Date: 2026-07-05

## Scope

- Otto repos audited: `otto-protocol`, `otto-server`, `otto-update`, `otto-command-service`, `otto-kernel`, `otto-extensions`, `otto-ui`
- Compatibility repos checked during the same pass: `Maestro`, `CourseForge`
- Central report location: `otto-update/docs` because the `Otto/` workspace root is not a Git repository

## Preflight

- Reconfirmed clean working trees across the audited repos before validation.
- Existing uncommitted changes in `CourseForge` were committed first as `Workspace cleanup: commit pending changes before integrity test` (`2b17952`).

## Static Integration Findings

### Command-service ownership

- `otto-update/src/generated_api/index.ts` is generated from the standalone command-service and dynamically imports handlers from `../../otto-command-service/src/handlers/${schema.routing.handlerModule}`.
- `otto-kernel/src/kernel/kernel.ts` loads schemas from `@otto/command-service` and syncs them into the runtime router.
- `otto-kernel/src/kernel/commandRouter.ts` rejects registration for commands not declared by the standalone command-service schemas.
- `otto-extensions/modules/example/index.ts` resolves extension command schemas through `loadCommandSchemas()` from `@otto/command-service`.
- `otto-ui/src/services/commandRoutingService.ts` declares `@otto/command-service` as the UI command source and routes node-graph actions through that catalog.

### Migration assertions

- CLI/API surface generation succeeded in both `otto-command-service` and `otto-update` via `npm run generate:surfaces`.
- Maestro command contracts are still present in generated Otto surfaces (`maestro.install`, `maestro.update`, `maestro.repair`, `maestro.uninstall`).
- No stale embedded `otto-update/otto-command-service` path was found in active Maestro or CourseForge source during the compatibility scans.

## Executed Commands

### Workspace validation matrix

- `npm run typecheck`
- `npm test`
- `npm run build`

Executed in:

- `otto-protocol`
- `otto-server`
- `otto-update`
- `otto-command-service`
- `otto-kernel`
- `otto-extensions`
- `otto-ui`
- `Maestro`
- `CourseForge`

### Additional migration checks

- `npm --prefix /Users/dev-macbook/Documents/GitHub/Otto/otto-command-service run generate:surfaces`
- `npm --prefix /Users/dev-macbook/Documents/GitHub/Otto/otto-update run generate:surfaces`
- `npm --prefix /Users/dev-macbook/Documents/GitHub/Maestro run validate:command-service`
- `cd /Users/dev-macbook/Documents/GitHub/Otto/otto-update && OTTO_DRY_RUN=1 ./bootstrap.sh`

## Results

| Repo | Typecheck | Tests | Build | Notes |
| --- | --- | --- | --- | --- |
| `otto-protocol` | Pass | Pass | Pass | No issues observed |
| `otto-server` | Pass | Pass | Pass | No issues observed |
| `otto-update` | Pass | Pass | Pass | Generated surfaces regenerated cleanly |
| `otto-command-service` | Pass | Pass | Pass | Generator outputs refreshed successfully |
| `otto-kernel` | Pass | Pass | Pass | Command schema sync path intact |
| `otto-extensions` | Pass | Pass | Pass | Extension lifecycle/tests intact |
| `otto-ui` | Pass | Pass | Pass | Build succeeded; transient build artifacts were removed after validation |
| `Maestro` | Pass | Pass | Pass | `validate:command-service` also passed |
| `CourseForge` | Pass | Pass | Pass | Added explicit `typecheck` script and normalized two suites to Vitest during verification |

## Bootstrap Dry-Run

- Command attempted: `cd /Users/dev-macbook/Documents/GitHub/Otto/otto-update && OTTO_DRY_RUN=1 ./bootstrap.sh`
- Result: blocked
- Failure: `zsh: no such file or directory: ./bootstrap.sh`

The active Otto workspace repo at `/Users/dev-macbook/Documents/GitHub/Otto/otto-update` does not contain a `bootstrap.sh` entrypoint, so bootstrap dry-run verification could not be completed there. This is an artifact gap, not a TypeScript build/test failure.

## Conclusion

- The Otto ecosystem passed typecheck, test, build, generator, and Maestro contract validation after the CourseForge compatibility fixes applied during this verification pass.
- The remaining workspace-level blocker is the missing `bootstrap.sh` artifact in the active `Otto/otto-update` repo.
- No evidence of standalone CLI or embedded command-service ownership was found in the audited active source paths.