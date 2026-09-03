/**
 * Integration of self-healing into OttoUpdate's update flow
 * 
 * Provides pre-update validation and repair capabilities so that any program
 * using OttoUpdate automatically gets self-healing for its critical artifacts
 */

import type { UpdateManifest } from "@otto/protocol";
import { SelfHealingRegistry } from "./registry.js";
import type { SelfHealingCheckResult, SelfHealingRepairResult } from "./types.js";

export type PreUpdateValidationResult = {
  timestamp: number;
  healthCheck: SelfHealingCheckResult;
  repairAttempt?: SelfHealingRepairResult;
  canProceedWithUpdate: boolean;
  blockingIssues: string[];
  warnings: string[];
  recommendations: string[];
};

/**
 * Integrates self-healing validation into the update process
 * 
 * Programs should call this before attempting any update operation
 * to ensure their critical artifacts are healthy and will be repaired if needed
 */
export class PreUpdateValidator {
  constructor(private registry: SelfHealingRegistry) {}

  /**
   * Validate all registered artifacts before starting an update
   * Optionally attempts repairs if issues are found
   * 
   * @param manifest The update manifest being applied
   * @param autoRepair Whether to automatically attempt repairs
   * @returns Validation result indicating if update can proceed
   */
  async validateBeforeUpdate(
    manifest: UpdateManifest,
    autoRepair: boolean = true
  ): Promise<PreUpdateValidationResult> {
    const startTime = Date.now();
    const blockingIssues: string[] = [];
    const warnings: string[] = [];
    const recommendations: string[] = [];

    // Perform health check
    const healthCheck = await this.registry.performHealthCheck();

    // Collect issues
    for (const issue of healthCheck.issues) {
      const message = `${issue.artifactName} (${issue.artifactId}): ${issue.details?.reason || "unhealthy"}`;

      if (issue.severity === "error") {
        blockingIssues.push(message);
        recommendations.push(
          `Artifact '${issue.artifactId}' is critical. Run repair before updating.`
        );
      } else if (issue.severity === "warning") {
        warnings.push(message);
        recommendations.push(
          `Consider repairing artifact '${issue.artifactId}' before update.`
        );
      }
    }

    // Attempt repairs if enabled and issues found
    let repairAttempt: SelfHealingRepairResult | undefined;
    if (autoRepair && healthCheck.issuesFound > 0) {
      repairAttempt = await this.registry.performRepairs();

      for (const repair of repairAttempt.repairs) {
        if (repair.repaired) {
          recommendations.push(
            `Successfully repaired '${repair.artifactId}'. Update can proceed safely.`
          );
        } else if (!repair.success) {
          if (repair.severity === "error") {
            blockingIssues.push(
              `Failed to repair critical artifact '${repair.artifactId}': ${repair.reason}`
            );
          } else {
            warnings.push(
              `Failed to repair '${repair.artifactId}': ${repair.reason}`
            );
          }
        }
      }
    }

    const canProceedWithUpdate = blockingIssues.length === 0;

    return {
      timestamp: startTime,
      healthCheck,
      repairAttempt,
      canProceedWithUpdate,
      blockingIssues,
      warnings,
      recommendations,
    };
  }
}

/**
 * Helper to integrate self-healing into existing UpdateEngine workflows
 */
export function createPreUpdateValidator(
  registry: SelfHealingRegistry
): PreUpdateValidator {
  return new PreUpdateValidator(registry);
}
