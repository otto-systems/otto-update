# Command Service Rescan Report

Date: 2026-07-05

## Scope
- `otto-update`
- `otto-kernel`
- `otto-ui`
- `otto-extensions`
- `Maestro`

## Key Finding
- No embedded `otto-command-service` directory exists inside `otto-update` in the current Otto workspace.
- The command service layer is already located in the standalone sibling repo `Otto/otto-command-service`.

## `otto-update`
- Dependency wiring points to the standalone repo via `@otto/command-service: file:../otto-command-service`.
- Generator scripts invoke the standalone generators via `../otto-command-service/src/generators/...`.
- `src/main.ts` imports only generated surfaces from `src/generated_cli/index.ts` and `src/generated_api/index.ts`.
- Active source scan found no standalone CLI package, manual argument parsing, or manual command routing logic under `src/`.
- Generated API surface dynamically resolves handlers from the standalone repo path `../../otto-command-service/src/handlers/${schema.routing.handlerModule}`.

## `otto-kernel`
- Dependency wiring points to the standalone repo via `@otto/command-service: file:../otto-command-service`.
- `src/kernel/kernel.ts` imports `loadCommandSchemas` from `@otto/command-service`.
- No duplicate command schema definitions or standalone command handlers were found in kernel source.

## `otto-ui`
- `src/services/commandRoutingService.ts` declares `@otto/command-service` as the routing source of truth.
- Node graph routing maps UI nodes to standalone command names.
- No manual CLI parsing or standalone command definitions were found in UI source.

## `otto-extensions`
- Dependency wiring points to the standalone repo via `@otto/command-service: file:../otto-command-service`.
- Example module imports `loadCommandSchemas` from `@otto/command-service`.
- No standalone extension-local command schema definitions were found outside the command-service repo.

## `Maestro`
- Dependency wiring points to the standalone repo via `@otto/command-service: file:../Otto/otto-command-service`.
- `src/shared/utils/commandServiceContract.ts` validates that required Maestro commands are present in the standalone schema set.
- No stale references to an embedded `otto-update/otto-command-service` location were found in active Maestro source.

## Violations Check
- Embedded command-service folder inside `otto-update`: not present.
- Standalone CLI code under `otto-update/src`: not found.
- Manual routing or manual command handlers in audited repos: not found in active source.
- Imports targeting an old embedded command-service path: not found in active source.

## Conclusion
- The current workspace is already rewired to the standalone `otto-command-service` repo.
- Remaining work for this migration is documentation accuracy and ongoing validation, not code relocation.