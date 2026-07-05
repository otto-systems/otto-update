use std::path::{Path, PathBuf};

use async_trait::async_trait;
use anyhow::Result;
use mockall::mock;

use crate::applier::Applier;
use crate::downloader::Downloader;
use crate::manifest_fetcher::ManifestFetcher;
use crate::traits::{Artifact, PolicyDecision, PolicyEngine, ReleaseManifest};
use crate::verifier::Verifier;

mock! {
    pub ManifestFetcher {}

    #[async_trait]
    impl ManifestFetcher for ManifestFetcher {
        async fn fetch(&self, url: &str) -> Result<ReleaseManifest>;
    }
}

mock! {
    pub PolicyEngine {}

    #[async_trait]
    impl PolicyEngine for PolicyEngine {
        async fn evaluate(&self, manifest: &ReleaseManifest) -> Result<PolicyDecision>;
    }
}

mock! {
    pub Downloader {}

    #[async_trait]
    impl Downloader for Downloader {
        async fn download(&self, artifact: &Artifact) -> Result<PathBuf>;
    }
}

mock! {
    pub Verifier {}

    #[async_trait]
    impl Verifier for Verifier {
        async fn verify(&self, path: &Path, artifact: &Artifact) -> Result<()>;
    }
}

mock! {
    pub Applier {}

    #[async_trait]
    impl Applier for Applier {
        async fn apply(&self, path: &Path, manifest: &ReleaseManifest) -> Result<()>;
        async fn rollback(&self) -> Result<()>;
    }
}

pub fn fixture_manifest(version: &str) -> ReleaseManifest {
    ReleaseManifest {
        version: version.to_string(),
        artifacts: vec![Artifact {
            id: "artifact-1".to_string(),
            url: "https://example.invalid/otto.bin".to_string(),
            sha256_hex: None,
            signature_b64: None,
            public_key_hex: None,
        }],
    }
}
