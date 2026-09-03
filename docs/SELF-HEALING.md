# OttoUpdate Self-Healing Framework

## Overview

The Self-Healing Framework is a built-in capability in OttoUpdate that enables any Otto-consuming program to automatically validate and repair critical artifacts before updates are applied.

Instead of manually handling script staleness, configuration drift, or other artifact health issues, programs can simply register their critical files with the framework and get:

- **Automatic pre-update validation** - detects stale or damaged artifacts
- **Automatic repair** - regenerates or fixes unhealthy artifacts if possible
- **Blocking safeguards** - prevents updates if critical artifacts are unhealthy
- **Detailed reporting** - comprehensive health check and repair results
- **Customizable behavior** - programs define their own validation and repair logic

## Architecture

### Three-Layer Design

```
Layer 1: ArtifactConfig Registration
├─ Programs register critical artifacts (scripts, configs, etc.)
├─ Each artifact has validation and optional repair functions
└─ Criticality levels control blocking behavior

Layer 2: SelfHealingRegistry
├─ Manages lifecycle of all registered artifacts
├─ Performs health checks on all artifacts
├─ Coordinates repair attempts
└─ Provides comprehensive results

Layer 3: PreUpdateValidator Integration
├─ Integrates health checks into update workflow
├─ Validates before update proceeds
├─ Makes repair recommendations
└─ Returns block/proceed decision
```

## Usage

### Step 1: Define Your Artifacts

For each critical file/script your program needs to monitor:

```typescript
import type { ArtifactConfig } from "@otto/update";

const autoUpdateScriptArtifact: ArtifactConfig = {
  id: "auto-update-script",
  name: "Auto-Update Script",
  path: "/opt/my-app/auto-update.sh",
  
  // Validation: Check if script has required functions
  validate: (content: string) => {
    const requiredFunctions = ["run_update", "check_health", "report_status"];
    const missingFunctions = [];

    for (const func of requiredFunctions) {
      const pattern = new RegExp(`${func}\\s*\\(\\s*\\)\\s*\\{|function\\s+${func}\\s*\\{`);
      if (!pattern.test(content)) {
        missingFunctions.push(func);
      }
    }

    return {
      isHealthy: missingFunctions.length === 0,
      severity: missingFunctions.length === 0 ? "info" : "error",
      missingComponents: missingFunctions,
      details: {
        totalFunctions: requiredFunctions.length,
        foundFunctions: requiredFunctions.length - missingFunctions.length,
      },
    };
  },

  // Repair: Regenerate from canonical template
  repair: async (options) => {
    try {
      // Read canonical template
      const templateContent = await options.readFile(
        "/opt/my-app/runtime/auto-update.sh.template"
      );

      // Write to target location
      await options.writeFile(options.artifactPath, templateContent);

      return {
        success: true,
        repaired: true,
        severity: "info",
        reason: "repaired",
        details: { path: options.artifactPath },
      };
    } catch (error) {
      return {
        success: false,
        repaired: false,
        severity: "error",
        reason: "repair-failed",
        details: { error: error instanceof Error ? error.message : String(error) },
      };
    }
  },

  // This artifact is critical for updates
  criticalityLevel: "error",

  // Optional callbacks
  onValidationFailed: (result) => {
    console.warn(`Script validation failed: ${result.missingComponents?.join(", ")}`);
  },

  onRepairCompleted: (result) => {
    console.log(`Repair result: ${result.reason}`);
  },
};
```

### Step 2: Register Artifacts

```typescript
import { SelfHealingRegistry } from "@otto/update";

// Create registry (or use global)
const registry = new SelfHealingRegistry("/opt/my-app");

// Register your artifacts
registry.register(autoUpdateScriptArtifact);
registry.register(configFileArtifact);
registry.register(libraryFileArtifact);
```

### Step 3: Integrate Into Update Flow

```typescript
import { createPreUpdateValidator } from "@otto/update";

async function performUpdate(manifest) {
  // Create validator
  const validator = createPreUpdateValidator(registry);

  // Run pre-update validation
  const validation = await validator.validateBeforeUpdate(manifest, autoRepair = true);

  // Check results
  if (!validation.canProceedWithUpdate) {
    console.error("Cannot update due to blocking issues:");
    validation.blockingIssues.forEach(issue => console.error(`  - ${issue}`));
    return { ok: false, reason: "pre-update validation failed" };
  }

  // Show warnings if any
  if (validation.warnings.length > 0) {
    console.warn("Pre-update warnings:");
    validation.warnings.forEach(w => console.warn(`  - ${w}`));
  }

  // Show recommendations
  if (validation.recommendations.length > 0) {
    console.info("Repair recommendations:");
    validation.recommendations.forEach(r => console.info(`  - ${r}`));
  }

  // Safe to proceed with update
  console.log("All artifacts healthy. Proceeding with update...");
  return await actualUpdateLogic(manifest);
}
```

## Example: otto-display-system Implementation

Here's how otto-display-system could use self-healing for its auto-update script:

```typescript
// In apps/display-runtime/src/initialization.ts

import {
  SelfHealingRegistry,
  createPreUpdateValidator,
  type ArtifactConfig,
} from "@otto/update";

const registry = new SelfHealingRegistry("/opt/otto-display-system");

// Register the auto-update script
const autoUpdateArtifact: ArtifactConfig = {
  id: "otto-display-auto-update-script",
  name: "Otto Display Auto-Update Script",
  path: "../auto-update.sh", // Relative to current/

  validate: (content) => {
    const requiredFunctions = [
      "run_command",
      "read_manifest_version",
      "legacy_update_fallback",
    ];
    const missingFunctions = requiredFunctions.filter((fn) => {
      const pattern = new RegExp(
        `${fn}\\s*\\(\\s*\\)|function\\s+${fn}\\s*\\(`
      );
      return !pattern.test(content);
    });

    return {
      isHealthy: missingFunctions.length === 0,
      severity: missingFunctions.length === 0 ? "info" : "error",
      missingComponents: missingFunctions,
    };
  },

  repair: async (options) => {
    try {
      const template = await options.readFile("runtime/auto-update.sh.template");
      await options.writeFile(options.artifactPath, template);

      return {
        success: true,
        repaired: true,
        severity: "info",
        reason: "repaired",
      };
    } catch (error) {
      return {
        success: false,
        repaired: false,
        severity: "error",
        reason: "repair-failed",
        details: {
          error: error instanceof Error ? error.message : String(error),
        },
      };
    }
  },

  criticalityLevel: "error",
};

registry.register(autoUpdateArtifact);

// Before any update operation
async function initializeUpdate() {
  const validator = createPreUpdateValidator(registry);
  const manifest = await fetchUpdateManifest();

  const validation = await validator.validateBeforeUpdate(manifest, true);

  if (!validation.canProceedWithUpdate) {
    throw new Error(
      `Update blocked by pre-flight checks: ${validation.blockingIssues.join("; ")}`
    );
  }

  return validation;
}
```

## API Reference

### ArtifactConfig

```typescript
type ArtifactConfig = {
  id: string;
  name: string;
  path: string;
  validate: (content: string) => ValidationResult;
  repair?: (options: RepairOptions) => Promise<RepairResult>;
  criticalityLevel: "error" | "warning" | "info";
  onValidationFailed?: (result: ValidationResult) => void;
  onRepairCompleted?: (result: RepairResult) => void;
};
```

### SelfHealingRegistry

```typescript
class SelfHealingRegistry {
  constructor(workspaceRoot: string);

  // Registration
  register(artifact: ArtifactConfig): void;
  unregister(artifactId: string): boolean;
  getArtifact(artifactId: string): ArtifactConfig | undefined;
  listArtifacts(): ArtifactConfig[];
  clear(): void;

  // Validation & Repair
  performHealthCheck(): Promise<SelfHealingCheckResult>;
  performRepairs(): Promise<SelfHealingRepairResult>;
  performSelfHealing(): Promise<{
    check: SelfHealingCheckResult;
    repair?: SelfHealingRepairResult;
    ok: boolean;
  }>;
}
```

### PreUpdateValidator

```typescript
class PreUpdateValidator {
  constructor(registry: SelfHealingRegistry);

  validateBeforeUpdate(
    manifest: UpdateManifest,
    autoRepair?: boolean
  ): Promise<PreUpdateValidationResult>;
}
```

## Result Formats

### SelfHealingCheckResult

```typescript
{
  timestamp: number;
  checksPerformed: number;
  issuesFound: number;
  blockingIssues: number;
  issues: Array<{
    artifactId: string;
    artifactName: string;
    isHealthy: boolean;
    severity: "error" | "warning" | "info";
    missingComponents?: string[];
    details?: Record<string, unknown>;
  }>;
  ok: boolean;
}
```

### SelfHealingRepairResult

```typescript
{
  timestamp: number;
  repairsAttempted: number;
  repairsSucceeded: number;
  repairsFailed: number;
  repairs: Array<{
    artifactId: string;
    artifactName: string;
    success: boolean;
    repaired: boolean;
    severity: "error" | "warning" | "info";
    reason: "already-healthy" | "repaired" | "missing" | "created" | "repair-failed";
    details?: Record<string, unknown>;
  }>;
  ok: boolean;
  shouldRetry: boolean;
}
```

### PreUpdateValidationResult

```typescript
{
  timestamp: number;
  healthCheck: SelfHealingCheckResult;
  repairAttempt?: SelfHealingRepairResult;
  canProceedWithUpdate: boolean;
  blockingIssues: string[];
  warnings: string[];
  recommendations: string[];
}
```

## Best Practices

1. **Keep validators simple and fast** - They run before updates, keep them efficient
2. **Make repairs idempotent** - Running repair twice should give same result
3. **Log validation failures** - Help operators understand what went wrong
4. **Use appropriate criticality levels**:
   - `error` = blocking (update won't proceed)
   - `warning` = non-blocking but reported
   - `info` = silent (only logged)
5. **Provide repair functions** - Otherwise artifacts can only be detected, not fixed
6. **Test both validation and repair paths** - Ensure both work correctly

## Global Registry Usage

For simpler applications, use the global registry:

```typescript
import { getGlobalSelfHealingRegistry } from "@otto/update";

const registry = getGlobalSelfHealingRegistry("/opt/my-app");
registry.register(myArtifact);
```

Then later:

```typescript
const registry = getGlobalSelfHealingRegistry();
const result = await registry.performHealthCheck();
```

Clean up when needed:

```typescript
import { resetGlobalRegistry } from "@otto/update";
resetGlobalRegistry();
```
