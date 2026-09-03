# OttoUpdate

OttoUpdate is the update engine for Otto, responsible for update policies, release orchestration, and generated command execution surfaces.

**v0.3.0 — NEW: Self-Healing Framework**

Programs using OttoUpdate can now automatically validate and repair critical artifacts (scripts, configs, etc.) before applying updates, ensuring system health and preventing stale deployments from interfering with upgrades.

## Current Release Line

- Version: 0.3.0
- Major feature: Self-Healing Framework for artifact validation and automatic repair
- Backward compatible: Existing update workflows unchanged, self-healing is opt-in

## Responsibilities
- Resolve and evaluate update manifests.
- Execute update orchestration through generated command surfaces.
- Provide generated CLI/API integration points to the rest of the Otto platform.
- **NEW: Enable programs to validate and repair critical artifacts before updates** (Self-Healing Framework)

## Self-Healing Framework

### Overview
Any Otto program can now register critical artifacts (shell scripts, configuration files, etc.) for automatic pre-update validation and repair. This prevents issues where deployed systems run stale versions of critical files.

**Example: Auto-Update Script Staleness**
- Your app ships version N of `/opt/app/auto-update.sh`
- Version N+1 adds fallback retrieval and environment variable support
- Deployed system still runs version N (old installer never re-run)
- Without self-healing: Update fails because script lacks new features
- With self-healing: Script is auto-detected as stale, repaired with v N+1, update succeeds

### Key Features
- **Artifact Registration**: Simple API to register files for monitoring
- **Validation**: Customizable validation logic (check for required functions, configs, etc.)
- **Automatic Repair**: Regenerate stale artifacts from canonical templates
- **Pre-Update Integration**: Validates before update starts, blocks if critical issues found
- **Comprehensive Reporting**: Detailed health checks and repair results
- **Callback Support**: React to validation failures and repair completions

### Quick Start
```typescript
import { SelfHealingRegistry, createPreUpdateValidator } from "@otto/update";

// Register artifacts
const registry = new SelfHealingRegistry("/opt/my-app");
registry.register({
  id: "auto-update-script",
  name: "Auto-Update Script",
  path: "../auto-update.sh",
  validate: (content) => {
    // Check for required functions
    const hasFunctions = ["run_command", "read_manifest_version"]
      .every(fn => content.includes(fn));
    return { isHealthy: hasFunctions, severity: "error" };
  },
  repair: async (options) => {
    // Regenerate from template
    const template = await options.readFile("runtime/auto-update.sh.template");
    await options.writeFile(options.artifactPath, template);
    return { success: true, repaired: true, severity: "info", reason: "repaired" };
  },
  criticalityLevel: "error",
});

// Validate before update
const validator = createPreUpdateValidator(registry);
const result = await validator.validateBeforeUpdate(manifest, autoRepair = true);

if (result.canProceedWithUpdate) {
  // Safe to proceed - all artifacts healthy
}
```

For complete documentation, see [SELF-HEALING.md](docs/SELF-HEALING.md)

## Command Surface Ownership

Otto CLI and API surfaces are generated exclusively from the standalone `otto-command-service` repository.

`otto-update` does not define embedded command schemas or manual command routing logic.

The current workspace wiring uses:
- `@otto/command-service` as a sibling file dependency
- `../otto-command-service/src/generators/*` as the generation source
- `src/generated_cli/` and `src/generated_api/` as the only command execution surfaces consumed by `src/main.ts`

## Structure
- `src/update/` – update domain logic
- `src/selfHealing/` – **NEW: Self-healing framework for artifact validation and repair**
- `src/generated_cli/` – generated CLI command surface
- `src/generated_api/` – generated API command surface
- `src/main.ts` – generated surface entrypoint
- `docs/` – migration and validation reports, **NEW: SELF-HEALING.md**
- `examples/` – **NEW: Example integrations (e.g., display-system-integration.ts)**
- `tests/` – unit tests, **NEW: selfHealing.test.ts**

## Migration Evidence
- `docs/workspace-clean-state.md` – workspace cleanup and git-baseline audit
- `docs/command-service-rescan-report.md` – cross-repo rescan and wiring verification
- `docs/command-service-validation.md` – typecheck, tests, and contract validation results

## Installer Workflow

- GitHub Actions is the authoritative build path for Windows, Linux, and macOS installers.
- Linux and macOS artifacts are owned by CI and are no longer cross-built locally on the Windows host.
- The release manifests and generated reports are kept in sync with the current bugfix line.

## Minimal Otto Payload Checklist

To install, deploy, and retrieve payloads from manifest data correctly, include all of the following in your release package flow.

### 1. Manifest Files

- `manifests/latest.json`
- `manifests/release-0.2.4.json` (or the current release file for your target version)

Required manifest fields:

- `product`
- `currentVersion`
- `targetVersion`
- `channel`
- `publishedAt`
- `artifacts[]` with `name`, `url`, and `checksum` (`sha256:<64-hex>`)

### 2. Payload Retrieval and Apply Modules

These modules are the minimum contract for fetching manifests, downloading payloads, and applying updates:

- `ottoupdate/ottoupdate-core/src/manifest_fetcher.rs`
- `ottoupdate/ottoupdate-core/src/downloader.rs`
- `ottoupdate/ottoupdate-core/src/applier.rs`

### 3. Generated Command Surfaces

The update flow must be accessible through generated command surfaces:

- `src/generated_cli/index.ts`
- `src/generated_api/index.ts`

### 4. Platform Installer Payload Contents

Windows payload (`.zip`) must contain:

- `ottoupdate-server.exe`
- `install.ps1`

Linux payload (`.tar.gz`) should contain:

- `ottoupdate-server`
- `install.sh`
- `ottoupdate.service`

macOS payload (`.tar.gz`) should contain:

- `ottoupdate-server`
- `install.sh`
- `com.otto.ottoupdate.plist`

## Example Installer References

Use these as the canonical minimal examples for future work:

- Workflow (cross-platform build + verification): [.github/workflows/macos-installer-validation.yml](.github/workflows/macos-installer-validation.yml)
- Windows installer script: [ottoupdate/deploy/windows/install.ps1](ottoupdate/deploy/windows/install.ps1)
- Linux installer script and service unit: [ottoupdate/deploy/linux/install.sh](ottoupdate/deploy/linux/install.sh), [ottoupdate/deploy/linux/ottoupdate.service](ottoupdate/deploy/linux/ottoupdate.service)
- macOS installer script and launchd plist: [ottoupdate/deploy/macos/install.sh](ottoupdate/deploy/macos/install.sh), [ottoupdate/deploy/macos/com.otto.ottoupdate.plist](ottoupdate/deploy/macos/com.otto.ottoupdate.plist)
- Example packaged artifacts: [artifacts/releases/0.2.4/otto-system-0.2.4-windows-x64.zip](artifacts/releases/0.2.4/otto-system-0.2.4-windows-x64.zip), [artifacts/releases/0.2.4/otto-system-0.2.4-macos-arm64.tar.gz](artifacts/releases/0.2.4/otto-system-0.2.4-macos-arm64.tar.gz)

When preparing a new bugfix release, keep the workflow, manifests, checksums, and payload file structure aligned to this baseline.
