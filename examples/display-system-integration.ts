/**
 * Example: Otto Display System Self-Healing Integration
 * 
 * Shows how otto-display-system uses the self-healing framework from @otto/update
 * to ensure its auto-update script stays healthy across system updates.
 */

import {
  SelfHealingRegistry,
  createPreUpdateValidator,
  type ArtifactConfig,
} from "@otto/update";

/**
 * Initialize self-healing for otto-display-system
 * This would be called during application startup
 */
export async function initializeSelfHealing() {
  // Create registry for the display system
  const registry = new SelfHealingRegistry("/opt/otto-display-system");

  // Define the auto-update script as a critical artifact
  const autoUpdateScriptArtifact: ArtifactConfig = {
    id: "display-auto-update-script",
    name: "Display System Auto-Update Script",
    
    // Path relative to workspace root
    path: "../auto-update.sh",

    /**
     * Validator: Ensure script has all required functions for proper update behavior
     * This prevents running stale scripts that lack new features like fallback retrieval
     */
    validate: (content: string) => {
      // These 3 functions are essential for robust updating
      const requiredFunctions = [
        "run_command",           // Executes otto commands via Node
        "read_manifest_version", // Fetches version info from manifest
        "legacy_update_fallback", // Applies updates via manifest/package URLs
      ];

      const missingFunctions: string[] = [];

      for (const funcName of requiredFunctions) {
        // Check both POSIX and bash function definition styles
        const posixStyle = new RegExp(`${funcName}\\s*\\(\\s*\\)\\s*\\{`, "m");
        const bashStyle = new RegExp(`function\\s+${funcName}\\s*\\{`, "m");

        if (!posixStyle.test(content) && !bashStyle.test(content)) {
          missingFunctions.push(funcName);
        }
      }

      return {
        isHealthy: missingFunctions.length === 0,
        severity: missingFunctions.length === 0 ? "info" : "error",
        missingComponents: missingFunctions,
        details: {
          totalRequired: requiredFunctions.length,
          found: requiredFunctions.length - missingFunctions.length,
          path: "/opt/otto-display-system/auto-update.sh",
        },
      };
    },

    /**
     * Repair: Regenerate the script from canonical template
     * This ensures deployed systems always have the latest update logic
     */
    repair: async (options) => {
      try {
        console.log(`[REPAIR] Regenerating ${options.artifactId}...`);

        // Try to read canonical template from multiple locations
        let templateContent: string | undefined;

        // First priority: runtime template (deployed with package)
        try {
          templateContent = await options.readFile?.(
            options.workspaceRoot + "/runtime/auto-update.sh.template"
          );
        } catch {
          // Fallback: fallback location
          try {
            templateContent = await options.readFile?.(
              "/opt/otto-display-system/runtime/auto-update.sh.template"
            );
          } catch {
            // Final fallback: hardcoded fallback (if available)
            console.warn(
              "[REPAIR] Could not read canonical template, using fallback"
            );
            templateContent = getHardcodedFallbackTemplate();
          }
        }

        if (!templateContent) {
          return {
            success: false,
            repaired: false,
            severity: "error",
            reason: "repair-failed" as const,
            details: { error: "No template source available" },
          };
        }

        // Validate template before writing
        const templateValidation = validateAutoUpdateScript(templateContent);
        if (!templateValidation.isHealthy) {
          return {
            success: false,
            repaired: false,
            severity: "error",
            reason: "repair-failed" as const,
            details: {
              error: "Template validation failed",
              missingFunctions: templateValidation.missingComponents,
            },
          };
        }

        // Write repaired script to target location
        await options.writeFile?.(
          options.artifactPath,
          templateContent
        );

        console.log(`[REPAIR] Successfully regenerated ${options.artifactId}`);

        return {
          success: true,
          repaired: true,
          severity: "info",
          reason: "repaired" as const,
          details: {
            path: options.artifactPath,
            timestamp: new Date().toISOString(),
          },
        };
      } catch (error) {
        console.error(
          `[REPAIR] Failed to repair ${options.artifactId}:`,
          error
        );

        return {
          success: false,
          repaired: false,
          severity: "error",
          reason: "repair-failed" as const,
          details: {
            error: error instanceof Error ? error.message : String(error),
          },
        };
      }
    },

    // Auto-update script is critical - updates cannot proceed without it
    criticalityLevel: "error" as const,

    // Optional: Log when validation fails (stale script detected)
    onValidationFailed: (result) => {
      console.warn(
        `[VALIDATION] Auto-update script is stale. Missing: ${result.missingComponents?.join(", ")}`
      );
      // Could send telemetry, emit event, etc.
    },

    // Optional: Log when repair completes
    onRepairCompleted: (result) => {
      console.log(
        `[REPAIR] Repair attempt completed: ${result.reason} (success=${result.success})`
      );
      // Could send telemetry, update UI, etc.
    },
  };

  // Register the artifact
  registry.register(autoUpdateScriptArtifact);

  console.log("[INIT] Self-healing initialized for display system");
  return registry;
}

/**
 * Pre-update validation hook
 * Call this before initiating any update to ensure system health
 */
export async function validateBeforeUpdate(
  manifest: any,
  registry: SelfHealingRegistry
) {
  console.log("[UPDATE] Running pre-update validation...");

  const validator = createPreUpdateValidator(registry);
  const validation = await validator.validateBeforeUpdate(manifest, true);

  console.log(`[UPDATE] Health check: ${validation.healthCheck.issuesFound} issues found`);

  if (validation.blockingIssues.length > 0) {
    console.error("[UPDATE] BLOCKING ISSUES - Update cannot proceed:");
    validation.blockingIssues.forEach((issue) =>
      console.error(`  ✗ ${issue}`)
    );
    return false;
  }

  if (validation.warnings.length > 0) {
    console.warn("[UPDATE] Non-blocking warnings:");
    validation.warnings.forEach((warning) =>
      console.warn(`  ⚠ ${warning}`)
    );
  }

  if (validation.recommendations.length > 0) {
    console.info("[UPDATE] System recommendations:");
    validation.recommendations.forEach((rec) =>
      console.info(`  ℹ ${rec}`)
    );
  }

  console.log("[UPDATE] All critical artifacts healthy - update can proceed");
  return true;
}

/**
 * Shared validation function (used by both artifact validator and repair)
 */
function validateAutoUpdateScript(content: string) {
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
    missingComponents: missingFunctions,
  };
}

/**
 * Hardcoded fallback template (minimal but functional)
 * This ensures repair can always succeed even if all external templates are unavailable
 */
function getHardcodedFallbackTemplate(): string {
  return `#!/usr/bin/env bash
set -euo pipefail

INSTALL_ROOT="\${OTTO_INSTALL_ROOT:-/opt/otto-display-system}"
CURRENT_DIR="\${INSTALL_ROOT}/current"

run_command() {
  local command_name="$1"
  shift
  node "\${CURRENT_DIR}/tools/run-otto-command.mjs" "\${command_name}" "$@"
}

read_manifest_version() {
  curl -fsSL "\${OTTO_UPDATE_MANIFEST_URL:-http://192.168.2.23:8090/manifest.json}" | node -e 'let d="";process.stdin.on("data",c=>d+=c);process.stdin.on("end",()=>{try{console.log(JSON.parse(d).version);}catch{process.exit(1);}});'
}

legacy_update_fallback() {
  echo "Applying fallback update..."
  local pkg_url="\${OTTO_UPDATE_PACKAGE_URL:-http://192.168.2.23:8090/otto-display-system-latest.zip}"
  curl -fsSL "\${pkg_url}" -o "\${INSTALL_ROOT}/package.zip"
  rm -rf "\${CURRENT_DIR}"
  mkdir -p "\${CURRENT_DIR}"
  unzip -o "\${INSTALL_ROOT}/package.zip" -d "\${CURRENT_DIR}"
  systemctl restart otto-display-system.service || true
  echo "Fallback update applied"
}

echo "Auto-update script loaded"
`;
}

/**
 * Example: Application startup with self-healing
 */
export async function startApplication() {
  console.log("🚀 Starting Otto Display System...");

  // Initialize self-healing early in startup
  const registry = await initializeSelfHealing();

  // Later, when handling updates:
  // const manifest = await fetchUpdateManifest();
  // const canUpdate = await validateBeforeUpdate(manifest, registry);
  // if (canUpdate) {
  //   await performUpdate(manifest);
  // }

  console.log("✅ Application ready with self-healing enabled");
}

// Export for use in command-service handlers
export { initializeSelfHealing, validateBeforeUpdate };
