/**
 * Registry and coordinator for self-healing artifacts
 * 
 * Manages lifecycle of artifact validation and repair for programs using OttoUpdate
 */

import * as fs from "fs/promises";
import * as path from "path";
import type {
  ArtifactConfig,
  RepairOptions,
  SelfHealingCheckResult,
  SelfHealingRepairResult,
} from "./types.js";

export class SelfHealingRegistry {
  private artifacts = new Map<string, ArtifactConfig>();
  private workspaceRoot: string;

  constructor(workspaceRoot: string = process.cwd()) {
    this.workspaceRoot = workspaceRoot;
  }

  /**
   * Register an artifact for monitoring and potential repair
   */
  register(artifact: ArtifactConfig): void {
    if (this.artifacts.has(artifact.id)) {
      throw new Error(`Artifact with id '${artifact.id}' is already registered`);
    }
    this.artifacts.set(artifact.id, artifact);
  }

  /**
   * Unregister a previously registered artifact
   */
  unregister(artifactId: string): boolean {
    return this.artifacts.delete(artifactId);
  }

  /**
   * Get a registered artifact by ID
   */
  getArtifact(artifactId: string): ArtifactConfig | undefined {
    return this.artifacts.get(artifactId);
  }

  /**
   * List all registered artifacts
   */
  listArtifacts(): ArtifactConfig[] {
    return Array.from(this.artifacts.values());
  }

  /**
   * Perform pre-update validation on all registered artifacts
   * Returns a comprehensive health check report
   */
  async performHealthCheck(): Promise<SelfHealingCheckResult> {
    const startTime = Date.now();
    const issues = [];
    let blockingIssues = 0;

    for (const artifact of this.artifacts.values()) {
      try {
        const artifactPath = this.resolvePath(artifact.path);
        const exists = await this.pathExists(artifactPath);

        if (!exists) {
          issues.push({
            artifactId: artifact.id,
            artifactName: artifact.name,
            isHealthy: false,
            severity: artifact.criticalityLevel,
            details: { reason: "missing", path: artifactPath },
          });
          if (artifact.criticalityLevel === "error") {
            blockingIssues++;
          }
          continue;
        }

        const content = await this.readFile(artifactPath);
        const validation = artifact.validate(content);

        if (!validation.isHealthy) {
          issues.push({
            artifactId: artifact.id,
            artifactName: artifact.name,
            isHealthy: false,
            severity: validation.severity,
            missingComponents: validation.missingComponents,
            details: validation.details,
          });
          if (validation.severity === "error") {
            blockingIssues++;
          }
          artifact.onValidationFailed?.(validation);
        }
      } catch (error) {
        issues.push({
          artifactId: artifact.id,
          artifactName: artifact.name,
          isHealthy: false,
          severity: artifact.criticalityLevel,
          details: {
            reason: "validation-error",
            error: error instanceof Error ? error.message : String(error),
          },
        });
        if (artifact.criticalityLevel === "error") {
          blockingIssues++;
        }
      }
    }

    return {
      timestamp: startTime,
      checksPerformed: this.artifacts.size,
      issuesFound: issues.length,
      blockingIssues,
      issues,
      ok: blockingIssues === 0,
    };
  }

  /**
   * Attempt to repair all unhealthy artifacts
   * Only repairs artifacts that have a repair function defined
   */
  async performRepairs(): Promise<SelfHealingRepairResult> {
    const startTime = Date.now();
    const repairs = [];
    let repairsSucceeded = 0;
    let repairsFailed = 0;

    for (const artifact of this.artifacts.values()) {
      if (!artifact.repair) {
        continue; // No repair capability for this artifact
      }

      try {
        const artifactPath = this.resolvePath(artifact.path);
        const exists = await this.pathExists(artifactPath);
        const currentContent = exists ? await this.readFile(artifactPath) : "";

        const repairOptions: RepairOptions = {
          artifactId: artifact.id,
          workspaceRoot: this.workspaceRoot,
          artifactPath,
          currentContent,
          readFile: (filePath: string) => this.readFile(filePath),
          writeFile: (filePath: string, content: string) =>
            this.writeFile(filePath, content),
          pathExists: (filePath: string) => this.pathExists(filePath),
        };

        const repairResult = await artifact.repair(repairOptions);

        repairs.push({
          artifactId: artifact.id,
          artifactName: artifact.name,
          success: repairResult.success,
          repaired: repairResult.repaired,
          severity: repairResult.severity,
          reason: repairResult.reason,
          details: repairResult.details,
        });

        if (repairResult.success) {
          repairsSucceeded++;
        } else {
          repairsFailed++;
        }

        artifact.onRepairCompleted?.(repairResult);
      } catch (error) {
        repairs.push({
          artifactId: artifact.id,
          artifactName: artifact.name,
          success: false,
          repaired: false,
          severity: "error",
          reason: "repair-error",
          details: {
            error: error instanceof Error ? error.message : String(error),
          },
        });
        repairsFailed++;
      }
    }

    return {
      timestamp: startTime,
      repairsAttempted: repairs.length,
      repairsSucceeded,
      repairsFailed,
      repairs,
      ok: repairsFailed === 0,
      shouldRetry: repairsFailed > 0 && repairs.length > 0,
    };
  }

  /**
   * Combined check + repair workflow
   * 1. Perform health check on all artifacts
   * 2. If issues found and repairs available, attempt repairs
   * 3. Return combined result
   */
  async performSelfHealing(): Promise<{
    check: SelfHealingCheckResult;
    repair?: SelfHealingRepairResult;
    ok: boolean;
  }> {
    const check = await this.performHealthCheck();

    // Only attempt repairs if there are unhealthy artifacts
    const repair = check.issuesFound > 0 ? await this.performRepairs() : undefined;

    return {
      check,
      repair,
      ok: check.ok && (!repair || repair.ok),
    };
  }

  /**
   * Clear all registered artifacts
   */
  clear(): void {
    this.artifacts.clear();
  }

  // Helper methods for file operations
  private resolvePath(filePath: string): string {
    if (path.isAbsolute(filePath)) {
      return filePath;
    }
    return path.join(this.workspaceRoot, filePath);
  }

  private async readFile(filePath: string): Promise<string> {
    return await fs.readFile(filePath, "utf8");
  }

  private async writeFile(filePath: string, content: string): Promise<void> {
    await fs.writeFile(filePath, content, "utf8");
    // Make executable if it's a shell script
    if (filePath.endsWith(".sh")) {
      await fs.chmod(filePath, 0o755);
    }
  }

  private async pathExists(filePath: string): Promise<boolean> {
    try {
      await fs.access(filePath);
      return true;
    } catch {
      return false;
    }
  }
}

// Singleton instance for global registry (optional)
let globalRegistry: SelfHealingRegistry | undefined;

export function getGlobalSelfHealingRegistry(workspaceRoot?: string): SelfHealingRegistry {
  if (!globalRegistry) {
    globalRegistry = new SelfHealingRegistry(workspaceRoot);
  }
  return globalRegistry;
}

export function resetGlobalRegistry(): void {
  globalRegistry = undefined;
}
