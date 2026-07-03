import type { UpdateManifest } from "@otto/protocol";

export class ManifestResolver {
  resolve(manifest: UpdateManifest): UpdateManifest {
    return {
      ...manifest,
      artifacts: manifest.artifacts ?? []
    };
  }
}
