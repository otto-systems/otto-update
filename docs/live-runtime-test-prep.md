# Live Runtime Test Preparation

Date: 2026-07-05

## Objective

Prepare the next manual live runtime passes for:

- CourseForge
- Maestro

This preparation pass focused on producing buildable artifacts, validating scripted bootstrap entrypoints, and enumerating the exact checkpoints that still require an interactive desktop run.

## Preparation Status

### CourseForge

- `npm run build`: pass
- `npm run build:mac`: pass
- `npm test`: pass
- `npm run typecheck`: pass

Available artifacts:

- `release/CourseForge-0.1.1-arm64.dmg`
- `release/CourseForge-0.1.1-arm64-mac.zip`

Relevant runtime wiring already validated in source/tests:

- Otto bootstrap readiness and self-update state handling: `src/bootstrap/otto-bootstrap.ts`
- CourseForge handoff after Otto update completion: `src/bootstrap/courseforge-bootstrap.ts`
- Auth transition after Otto lifecycle completion: `courseforge-ui/services/app-flow-controller.ts`

### Maestro

- `npm run build`: pass
- `npm run test`: pass
- `npm run typecheck`: pass
- `npm run validate:command-service`: pass
- `OTTO_DRY_RUN=1 installer/linux/bootstrap.sh`: pass

Bootstrap sequence validated by dry-run:

1. `otto ui splash --product maestro`
2. `otto self-update --channel stable`
3. `otto maestro install --payload otto/maestro-payload.json --channel stable`

Relevant runtime wiring already validated in source/payloads:

- Installer bootstrap scripts: `installer/linux/bootstrap.sh`, `installer/macos/bootstrap.sh`, `installer/windows/bootstrap.ps1`
- Command-service contract guard: `src/shared/utils/commandServiceContract.ts`
- Otto-managed payload and telemetry definitions: `otto/maestro-payload.json`

## Remaining Manual Live Checks

### CourseForge live test checklist

1. Launch the built CourseForge installer or packaged app.
2. Confirm Otto splash screen appears.
3. Confirm Otto self-update runs.
4. Confirm Otto restart occurs if an update requires it.
5. Confirm Otto reads the CourseForge payload.
6. Confirm CourseForge components download and install.
7. Confirm the app transitions to the auth page successfully.

### Maestro live test checklist

1. Run the platform bootstrap installer script or equivalent packaged entrypoint.
2. Confirm Otto splash screen appears.
3. Confirm Otto self-update runs.
4. Confirm Otto restart occurs if an update requires it.
5. Confirm Otto reads the Maestro payload.
6. Confirm Maestro downloads and installs.
7. Confirm Maestro UI loads correctly.
8. Confirm node graphs appear.
9. Confirm telemetry appears.
10. Confirm logs appear.

## Known Limits From This Preparation Pass

- The active Otto workspace repo `/Users/dev-macbook/Documents/GitHub/Otto/otto-update` does not contain `bootstrap.sh`, so the requested central `OTTO_DRY_RUN=1 bootstrap.sh` check could not be executed there.
- This session did not perform interactive GUI observation, so splash rendering, restart UX, node-graph rendering, telemetry panels, and auth-page visuals remain pending manual execution.
- Maestro currently exposes bootstrap installer scripts rather than a packaged installer build command inside the repository.

## Recommendation

The codebase is prepared for manual live runtime testing. The next step is to run the packaged CourseForge artifact and the platform-specific Maestro bootstrap flow on a desktop session with Otto installed and visible UI access.