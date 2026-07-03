import { describe, expect, it } from "vitest";
import { createUpdateManifest } from "@otto/protocol";

import { ManifestResolver } from "./manifestResolver.js";
import { UpdateEngine } from "./updateEngine.js";
import { compareVersions } from "./versionComparator.js";

describe("compareVersions", () => {
  it("compares semantic-like dotted versions", () => {
    expect(compareVersions("1.2.0", "1.2.1")).toBe(-1);
    expect(compareVersions("1.2.1", "1.2.0")).toBe(1);
    expect(compareVersions("1.2", "1.2.0")).toBe(0);
  });

  it("treats missing segments as zero", () => {
    expect(compareVersions("1", "1.0.1")).toBe(-1);
  });
});

describe("ManifestResolver", () => {
  it("ensures artifacts array is always present", () => {
    const resolver = new ManifestResolver();
    const resolved = resolver.resolve(createUpdateManifest({ artifacts: undefined }));

    expect(resolved.artifacts).toEqual([]);
  });
});

describe("UpdateEngine", () => {
  it("returns shouldUpdate=true for newer target", () => {
    const engine = new UpdateEngine();
    const decision = engine.evaluate(createUpdateManifest({ currentVersion: "0.2.0", targetVersion: "0.2.1" }));

    expect(decision.shouldUpdate).toBe(true);
  });

  it("returns shouldUpdate=false when already current", () => {
    const engine = new UpdateEngine();
    const decision = engine.evaluate(createUpdateManifest({ currentVersion: "0.2.1", targetVersion: "0.2.1" }));

    expect(decision.shouldUpdate).toBe(false);
  });
});
