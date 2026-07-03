use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::applier::Applier;
pub use crate::downloader::Downloader;
pub use crate::manifest_fetcher::ManifestFetcher;
pub use crate::verifier::Verifier;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub url: String,
    pub sha256_hex: Option<String>,
    pub signature_b64: Option<String>,
    pub public_key_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    pub artifacts: Vec<Artifact>,
}

impl ReleaseManifest {
    pub fn primary_artifact(&self) -> Option<&Artifact> {
        self.artifacts.first()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PolicyDecision {
    Approved,
    Deferred {
        reason: String,
        retry_after: DateTime<Utc>,
    },
    Blocked {
        reason: String,
    },
}

#[allow(async_fn_in_trait)]
pub trait PolicyEngine: Send + Sync {
    async fn evaluate(&self, manifest: &ReleaseManifest) -> Result<PolicyDecision>;
}
