use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use ottoupdate_core::coordinator::{UpdateConfig, UpdateCoordinator, UpdateOutcome};
use ottoupdate_core::manifest_fetcher::HttpManifestFetcher;
use ottoupdate_core::traits::{
    Applier, Artifact, Downloader, PolicyDecision, PolicyEngine, ReleaseManifest, Verifier,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct TestPolicyEngine {
    decision: PolicyDecision,
}

impl PolicyEngine for TestPolicyEngine {
    async fn evaluate(&self, _manifest: &ReleaseManifest) -> Result<PolicyDecision> {
        Ok(self.decision.clone())
    }
}

struct TestDownloader {
    payload: Vec<u8>,
}

impl Downloader for TestDownloader {
    async fn download(&self, _artifact: &Artifact) -> Result<PathBuf> {
        let path = std::env::temp_dir().join(format!("full-cycle-{}.bin", uuid::Uuid::new_v4()));
        tokio::fs::write(&path, &self.payload).await?;
        Ok(path)
    }
}

struct TestVerifier;

impl Verifier for TestVerifier {
    async fn verify(&self, _path: &Path, _artifact: &Artifact) -> Result<()> {
        Ok(())
    }
}

struct TestApplier {
    apply_fails: bool,
    rollback_called: Arc<AtomicBool>,
}

impl Applier for TestApplier {
    async fn apply(&self, _path: &Path, _manifest: &ReleaseManifest) -> Result<()> {
        if self.apply_fails {
            return Err(anyhow!("forced apply failure"));
        }
        Ok(())
    }

    async fn rollback(&self) -> Result<()> {
        self.rollback_called.store(true, Ordering::SeqCst);
        Ok(())
    }
}

fn state_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{}_{}_state.json",
        name,
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ))
}

fn manifest_json(server: &MockServer) -> serde_json::Value {
    serde_json::json!({
        "version": "2.1.0",
        "artifacts": [
            {
                "id": "artifact-1",
                "url": format!("{}/artifact.bin", server.uri()),
                "sha256_hex": null,
                "signature_b64": null,
                "public_key_hex": null
            }
        ]
    })
}

#[tokio::test]
async fn full_cycle_applies_successfully() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/manifest.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_json(&server)))
        .mount(&server)
        .await;

    let rollback_called = Arc::new(AtomicBool::new(false));
    let coordinator = UpdateCoordinator::new(
        UpdateConfig {
            manifest_url: format!("{}/manifest.json", server.uri()),
            state_path: state_path("full_cycle_apply"),
            max_retries: 0,
            retry_delay: Duration::zero(),
        },
        Arc::new(HttpManifestFetcher::default()),
        Arc::new(TestPolicyEngine {
            decision: PolicyDecision::Approved,
        }),
        Arc::new(TestDownloader {
            payload: b"payload".to_vec(),
        }),
        Arc::new(TestVerifier),
        Arc::new(TestApplier {
            apply_fails: false,
            rollback_called: rollback_called.clone(),
        }),
    );

    let outcome = coordinator.run_update_cycle().await.expect("cycle should succeed");
    assert_eq!(outcome, UpdateOutcome::Applied("2.1.0".to_string()));
    assert!(!rollback_called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn full_cycle_rolls_back_when_apply_fails() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/manifest.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(manifest_json(&server)))
        .mount(&server)
        .await;

    let rollback_called = Arc::new(AtomicBool::new(false));
    let coordinator = UpdateCoordinator::new(
        UpdateConfig {
            manifest_url: format!("{}/manifest.json", server.uri()),
            state_path: state_path("full_cycle_rollback"),
            max_retries: 0,
            retry_delay: Duration::zero(),
        },
        Arc::new(HttpManifestFetcher::default()),
        Arc::new(TestPolicyEngine {
            decision: PolicyDecision::Approved,
        }),
        Arc::new(TestDownloader {
            payload: b"payload".to_vec(),
        }),
        Arc::new(TestVerifier),
        Arc::new(TestApplier {
            apply_fails: true,
            rollback_called: rollback_called.clone(),
        }),
    );

    let outcome = coordinator.run_update_cycle().await.expect("cycle should run");
    match outcome {
        UpdateOutcome::Failed { .. } => {}
        other => panic!("expected failed outcome, got {other:?}"),
    }
    assert!(rollback_called.load(Ordering::SeqCst));
}
