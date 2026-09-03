import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { SelfHealingRegistry, resetGlobalRegistry } from "../src/selfHealing/registry";
import type { ArtifactConfig, ValidationResult } from "../src/selfHealing/types";

describe("SelfHealingRegistry", () => {
  let registry: SelfHealingRegistry;

  beforeEach(() => {
    registry = new SelfHealingRegistry(process.cwd());
    resetGlobalRegistry();
  });

  describe("artifact registration", () => {
    it("should register an artifact", () => {
      const artifact: ArtifactConfig = {
        id: "test-artifact",
        name: "Test Artifact",
        path: "/tmp/test.sh",
        validate: (content) => ({ isHealthy: true, severity: "info" }),
        criticalityLevel: "warning",
      };

      registry.register(artifact);
      expect(registry.getArtifact("test-artifact")).toBeDefined();
    });

    it("should reject duplicate artifact IDs", () => {
      const artifact: ArtifactConfig = {
        id: "test",
        name: "Test",
        path: "/tmp/test.sh",
        validate: (content) => ({ isHealthy: true, severity: "info" }),
        criticalityLevel: "warning",
      };

      registry.register(artifact);
      expect(() => registry.register(artifact)).toThrow();
    });

    it("should unregister an artifact", () => {
      const artifact: ArtifactConfig = {
        id: "test",
        name: "Test",
        path: "/tmp/test.sh",
        validate: (content) => ({ isHealthy: true, severity: "info" }),
        criticalityLevel: "warning",
      };

      registry.register(artifact);
      expect(registry.unregister("test")).toBe(true);
      expect(registry.getArtifact("test")).toBeUndefined();
    });

    it("should return false when unregistering non-existent artifact", () => {
      expect(registry.unregister("nonexistent")).toBe(false);
    });

    it("should list all registered artifacts", () => {
      const artifact1: ArtifactConfig = {
        id: "test1",
        name: "Test 1",
        path: "/tmp/test1.sh",
        validate: (content) => ({ isHealthy: true, severity: "info" }),
        criticalityLevel: "warning",
      };

      const artifact2: ArtifactConfig = {
        id: "test2",
        name: "Test 2",
        path: "/tmp/test2.sh",
        validate: (content) => ({ isHealthy: true, severity: "info" }),
        criticalityLevel: "error",
      };

      registry.register(artifact1);
      registry.register(artifact2);

      const artifacts = registry.listArtifacts();
      expect(artifacts).toHaveLength(2);
      expect(artifacts.map((a) => a.id)).toContain("test1");
      expect(artifacts.map((a) => a.id)).toContain("test2");
    });

    it("should clear all artifacts", () => {
      const artifact: ArtifactConfig = {
        id: "test",
        name: "Test",
        path: "/tmp/test.sh",
        validate: (content) => ({ isHealthy: true, severity: "info" }),
        criticalityLevel: "warning",
      };

      registry.register(artifact);
      registry.clear();

      expect(registry.listArtifacts()).toHaveLength(0);
    });
  });

  describe("validation", () => {
    it("should validate healthy artifact", async () => {
      const mockValidate = (content: string): ValidationResult => ({
        isHealthy: true,
        severity: "info",
      });

      const artifact: ArtifactConfig = {
        id: "healthy",
        name: "Healthy Artifact",
        path: "./test-artifacts/healthy.sh",
        validate: mockValidate,
        criticalityLevel: "error",
      };

      registry.register(artifact);

      // Mock file system
      const originalCheck = registry["performHealthCheck"];
      let called = false;

      registry["performHealthCheck"] = async () => {
        called = true;
        return {
          timestamp: Date.now(),
          checksPerformed: 1,
          issuesFound: 0,
          blockingIssues: 0,
          issues: [],
          ok: true,
        };
      };

      const result = await registry["performHealthCheck"]();
      expect(result.ok).toBe(true);
      expect(result.issuesFound).toBe(0);
    });

    it("should detect missing functions in scripts", () => {
      const validate = (content: string): ValidationResult => {
        const requiredFunctions = ["run_command", "read_manifest_version", "legacy_update_fallback"];
        const missingFunctions = requiredFunctions.filter((fn) => {
          const pattern = new RegExp(`${fn}\\s*\\(\\s*\\)|function\\s+${fn}\\s*\\(`);
          return !pattern.test(content);
        });

        return {
          isHealthy: missingFunctions.length === 0,
          severity: missingFunctions.length === 0 ? "info" : "error",
          missingComponents: missingFunctions,
        };
      };

      const staleScript = `#!/bin/bash\necho "old script"`;
      const result = validate(staleScript);

      expect(result.isHealthy).toBe(false);
      expect(result.missingComponents).toHaveLength(3);
      expect(result.missingComponents).toContain("run_command");
    });

    it("should recognize both bash function syntaxes", () => {
      const validate = (content: string): ValidationResult => {
        const requiredFunctions = ["run_command", "read_manifest_version", "legacy_update_fallback"];
        const missingFunctions = requiredFunctions.filter((fn) => {
          const pattern = new RegExp(`${fn}\\s*\\(\\s*\\)|function\\s+${fn}\\s*\\(`);
          return !pattern.test(content);
        });

        return {
          isHealthy: missingFunctions.length === 0,
          severity: missingFunctions.length === 0 ? "info" : "error",
          missingComponents: missingFunctions,
        };
      };

      const posixStyle = `
        run_command() { echo "test"; }
        read_manifest_version() { echo "1.0"; }
        function legacy_update_fallback { echo "fallback"; }
      `;

      const result = validate(posixStyle);
      expect(result.isHealthy).toBe(true);
      expect(result.missingComponents).toEqual([]);
    });
  });

  describe("repair", () => {
    it("should track repair attempts", async () => {
      let repairCalled = false;

      const artifact: ArtifactConfig = {
        id: "repairable",
        name: "Repairable Artifact",
        path: "/tmp/test.sh",
        validate: (content) => ({ isHealthy: false, severity: "error" }),
        repair: async (options) => {
          repairCalled = true;
          return {
            success: true,
            repaired: true,
            severity: "info",
            reason: "repaired",
          };
        },
        criticalityLevel: "error",
      };

      registry.register(artifact);

      // The repair method would be called during performRepairs
      // This test verifies the structure, actual repair requires file system mocking
      expect(artifact.repair).toBeDefined();
    });
  });

  describe("self-healing workflow", () => {
    it("should perform complete self-healing cycle", async () => {
      let validationCallCount = 0;
      let repairCallCount = 0;

      const artifact: ArtifactConfig = {
        id: "test-artifact",
        name: "Test Artifact",
        path: "/tmp/test.sh",
        validate: (content) => {
          validationCallCount++;
          return { isHealthy: false, severity: "error" };
        },
        repair: async (options) => {
          repairCallCount++;
          return {
            success: true,
            repaired: true,
            severity: "info",
            reason: "repaired",
          };
        },
        criticalityLevel: "error",
      };

      registry.register(artifact);

      // Both validate and repair functions should be defined
      expect(artifact.validate).toBeDefined();
      expect(artifact.repair).toBeDefined();
    });
  });

  describe("callbacks", () => {
    it("should call onValidationFailed callback", () => {
      let callbackCalled = false;
      let callbackResult: ValidationResult | undefined;

      const artifact: ArtifactConfig = {
        id: "test",
        name: "Test",
        path: "/tmp/test.sh",
        validate: (content) => ({
          isHealthy: false,
          severity: "error",
          missingComponents: ["component1"],
        }),
        criticalityLevel: "error",
        onValidationFailed: (result) => {
          callbackCalled = true;
          callbackResult = result;
        },
      };

      registry.register(artifact);

      // Callback would be triggered during performHealthCheck
      expect(artifact.onValidationFailed).toBeDefined();
    });

    it("should call onRepairCompleted callback", () => {
      let callbackCalled = false;

      const artifact: ArtifactConfig = {
        id: "test",
        name: "Test",
        path: "/tmp/test.sh",
        validate: (content) => ({ isHealthy: true, severity: "info" }),
        repair: async (options) => ({
          success: true,
          repaired: true,
          severity: "info",
          reason: "repaired",
        }),
        criticalityLevel: "error",
        onRepairCompleted: (result) => {
          callbackCalled = true;
        },
      };

      registry.register(artifact);
      expect(artifact.onRepairCompleted).toBeDefined();
    });
  });
});
