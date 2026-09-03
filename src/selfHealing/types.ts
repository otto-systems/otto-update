/**
 * Self-Healing Framework for OttoUpdate
 * 
 * Enables any Otto-consuming program to register critical artifacts (scripts, configs, etc.)
 * for automatic validation and repair before updates are applied.
 */

export type SeverityLevel = "error" | "warning" | "info";

export type ValidationResult = {
  isHealthy: boolean;
  severity: SeverityLevel;
  missingComponents?: string[];
  details?: Record<string, unknown>;
};

export type RepairResult = {
  success: boolean;
  repaired: boolean;
  severity: SeverityLevel;
  reason: "already-healthy" | "repaired" | "missing" | "created" | "repair-failed";
  details?: Record<string, unknown>;
};

/**
 * Configuration for a single artifact to be monitored and potentially repaired
 */
export type ArtifactConfig = {
  /** Unique identifier for this artifact */
  id: string;

  /** Display name for logging and reporting */
  name: string;

  /** File path to monitor (absolute or relative to workspaceRoot) */
  path: string;

  /** 
   * Validation function that checks if the artifact is healthy
   * @param content File content as string
   * @returns ValidationResult indicating health status
   */
  validate: (content: string) => ValidationResult;

  /**
   * Optional repair function that can fix stale or damaged artifacts
   * If not provided, artifact can only be validated, not repaired
   * @param options Repair options including paths and file system access
   * @returns RepairResult indicating what was done
   */
  repair?: (options: RepairOptions) => Promise<RepairResult>;

  /**
   * Whether to block updates if this artifact is unhealthy
   * "error" = blocking, "warning" = non-blocking but reported, "info" = silent
   */
  criticalityLevel: SeverityLevel;

  /**
   * Optional callback when validation detects issues
   */
  onValidationFailed?: (result: ValidationResult) => void;

  /**
   * Optional callback when repair completes
   */
  onRepairCompleted?: (result: RepairResult) => void;
};

export type RepairOptions = {
  artifactId: string;
  workspaceRoot: string;
  artifactPath: string;
  currentContent: string;
  readFile?: (path: string) => Promise<string>;
  writeFile?: (path: string, content: string) => Promise<void>;
  pathExists?: (path: string) => Promise<boolean>;
};

/**
 * Result from validating all registered artifacts
 */
export type SelfHealingCheckResult = {
  timestamp: number;
  checksPerformed: number;
  issuesFound: number;
  blockingIssues: number;
  issues: Array<{
    artifactId: string;
    artifactName: string;
    isHealthy: boolean;
    severity: SeverityLevel;
    missingComponents?: string[];
    details?: Record<string, unknown>;
  }>;
  ok: boolean;
};

/**
 * Result from attempting to repair all unhealthy artifacts
 */
export type SelfHealingRepairResult = {
  timestamp: number;
  repairsAttempted: number;
  repairsSucceeded: number;
  repairsFailed: number;
  repairs: Array<{
    artifactId: string;
    artifactName: string;
    success: boolean;
    repaired: boolean;
    severity: SeverityLevel;
    reason: string;
    details?: Record<string, unknown>;
  }>;
  ok: boolean;
  shouldRetry: boolean;
};
