import type { UpdateManifest } from "@otto/protocol";
import { ManifestResolver } from "./manifestResolver.js";
import { compareVersions } from "./versionComparator.js";

export type UpdateDecision = {
  shouldUpdate: boolean;
  currentVersion: string;
  targetVersion: string;
};

export class UpdateEngine {
  constructor(private readonly resolver = new ManifestResolver()) {}

  evaluate(manifest: UpdateManifest): UpdateDecision {
    const resolvedManifest = this.resolver.resolve(manifest);
    return {
      shouldUpdate: compareVersions(resolvedManifest.currentVersion, resolvedManifest.targetVersion) < 0,
      currentVersion: resolvedManifest.currentVersion,
      targetVersion: resolvedManifest.targetVersion
    };
  }
}
