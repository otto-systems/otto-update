use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use tracing::{error, info, instrument, warn};

use crate::state_machine::{StateMachine, UpdateEvent};
use crate::traits::{Applier, Downloader, ManifestFetcher, PolicyDecision, PolicyEngine, Verifier};

#[derive(Debug, Clone)]
pub struct UpdateConfig {
    pub manifest_url: String,
    pub state_path: PathBuf,
    pub max_retries: u32,
    pub retry_delay: Duration,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            manifest_url: "http://localhost:7430/v1/manifest".to_string(),
            state_path: PathBuf::from("./data/ottoupdate_state.json"),
            max_retries: 0,
            retry_delay: Duration::seconds(0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    Applied(String),
    Deferred {
        reason: String,
        retry_after: DateTime<Utc>,
    },
    Blocked {
        reason: String,
    },
    Failed {
        error: String,
    },
}

pub struct UpdateCoordinator {
    config: UpdateConfig,
    manifest_fetcher: Arc<dyn ManifestFetcher>,
    policy_engine: Arc<dyn PolicyEngine>,
    downloader: Arc<dyn Downloader>,
    verifier: Arc<dyn Verifier>,
    applier: Arc<dyn Applier>,
}

impl UpdateCoordinator {
    pub fn new(
        config: UpdateConfig,
        manifest_fetcher: Arc<dyn ManifestFetcher>,
        policy_engine: Arc<dyn PolicyEngine>,
        downloader: Arc<dyn Downloader>,
        verifier: Arc<dyn Verifier>,
        applier: Arc<dyn Applier>,
    ) -> Self {
        Self {
            config,
            manifest_fetcher,
            policy_engine,
            downloader,
            verifier,
            applier,
        }
    }

    #[instrument(skip(self))]
    pub async fn run_update_cycle(&self) -> Result<UpdateOutcome> {
        info!(phase = "check", "starting update cycle");
        let mut machine = StateMachine::new(self.config.state_path.clone());
        machine.load_or_default().await?;

        machine.apply_event(UpdateEvent::CheckTriggered).await?;

        info!(phase = "fetch_manifest", url = %self.config.manifest_url, "fetching manifest");
        let manifest = self
            .manifest_fetcher
            .fetch(&self.config.manifest_url)
            .await
            .map_err(|err| anyhow!("manifest fetch failed: {err}"))?;

        machine.apply_event(UpdateEvent::ManifestReceived).await?;
        machine.apply_event(UpdateEvent::CheckTriggered).await?;

        info!(phase = "policy_evaluate", version = %manifest.version, "evaluating policy");
        let decision = self
            .policy_engine
            .evaluate(&manifest)
            .await
            .map_err(|err| anyhow!("policy evaluation failed: {err}"))?;

        match decision {
            PolicyDecision::Deferred {
                reason,
                retry_after,
            } => {
                machine.apply_event(UpdateEvent::PolicyDeferred).await?;
                info!(phase = "policy_evaluate", reason = %reason, retry_after = %retry_after, "update deferred by policy");
                return Ok(UpdateOutcome::Deferred {
                    reason,
                    retry_after,
                });
            }
            PolicyDecision::Blocked { reason } => {
                machine.apply_event(UpdateEvent::PolicyBlocked).await?;
                warn!(phase = "policy_evaluate", reason = %reason, "update blocked by policy");
                return Ok(UpdateOutcome::Blocked { reason });
            }
            PolicyDecision::Approved => {
                machine.apply_event(UpdateEvent::PolicyApproved).await?;
            }
        }

        let artifact = manifest
            .primary_artifact()
            .ok_or_else(|| anyhow!("manifest has no artifacts"))?;

        info!(phase = "download", artifact = %artifact.id, "downloading artifact");
        machine.apply_event(UpdateEvent::CheckTriggered).await?;
        let download_path = self
            .downloader
            .download(artifact)
            .await
            .map_err(|err| anyhow!("download failed: {err}"))?;
        machine.apply_event(UpdateEvent::DownloadComplete).await?;

        info!(phase = "verify", path = %download_path.display(), "verifying artifact");
        if let Err(err) = self.verifier.verify(Path::new(&download_path), artifact).await {
            machine.apply_event(UpdateEvent::VerificationFailed).await?;
            error!(phase = "verify", error = %err, "verification failed");
            return Ok(UpdateOutcome::Failed {
                error: format!("verification failed: {err}"),
            });
        }
        machine.apply_event(UpdateEvent::VerificationPassed).await?;

        info!(phase = "apply", version = %manifest.version, "applying update");
        machine.apply_event(UpdateEvent::ApplyConfirmed).await?;
        if let Err(apply_err) = self
            .applier
            .apply(Path::new(&download_path), &manifest)
            .await
        {
            machine.apply_event(UpdateEvent::ApplyFailed).await?;
            error!(phase = "apply", error = %apply_err, "apply failed, attempting rollback");

            let mut msg = format!("apply failed: {apply_err}");
            if let Err(rollback_err) = self.applier.rollback().await {
                error!(phase = "rollback", error = %rollback_err, "rollback failed");
                msg = format!("{msg}; rollback failed: {rollback_err}");
            } else {
                machine.apply_event(UpdateEvent::RollbackTriggered).await?;
                info!(phase = "rollback", "rollback succeeded");
            }

            return Ok(UpdateOutcome::Failed { error: msg });
        }

        machine.apply_event(UpdateEvent::ApplySucceeded).await?;
        info!(phase = "complete", version = %manifest.version, "update applied");
        Ok(UpdateOutcome::Applied(manifest.version))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use anyhow::{anyhow, Result};
    use chrono::{Duration, Utc};

    use super::{UpdateConfig, UpdateCoordinator, UpdateOutcome};
    use crate::traits::{
        Applier, Artifact, Downloader, ManifestFetcher, PolicyDecision, PolicyEngine,
        ReleaseManifest, Verifier,
    };

    struct StubManifestFetcher {
        manifest: ReleaseManifest,
    }

    #[async_trait]
    impl ManifestFetcher for StubManifestFetcher {
        async fn fetch(&self, _url: &str) -> Result<ReleaseManifest> {
            Ok(self.manifest.clone())
        }
    }

    struct StubPolicyEngine {
        decision: PolicyDecision,
    }

    #[async_trait]
    impl PolicyEngine for StubPolicyEngine {
        async fn evaluate(&self, _manifest: &ReleaseManifest) -> Result<PolicyDecision> {
            Ok(self.decision.clone())
        }
    }

    struct StubDownloader;

    #[async_trait]
    impl Downloader for StubDownloader {
        async fn download(&self, _artifact: &Artifact) -> Result<PathBuf> {
            Ok(PathBuf::from("/tmp/otto-artifact.bin"))
        }
    }

    struct StubVerifier {
        should_fail: bool,
    }

    #[async_trait]
    impl Verifier for StubVerifier {
        async fn verify(&self, _path: &Path, _artifact: &Artifact) -> Result<()> {
            if self.should_fail {
                return Err(anyhow!("bad signature"));
            }
            Ok(())
        }
    }

    struct StubApplier {
        should_fail_apply: bool,
        rollback_called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Applier for StubApplier {
        async fn apply(&self, _path: &Path, _manifest: &ReleaseManifest) -> Result<()> {
            if self.should_fail_apply {
                return Err(anyhow!("apply I/O error"));
            }
            Ok(())
        }

        async fn rollback(&self) -> Result<()> {
            self.rollback_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    fn state_file(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{}_{}_state.json",
            name,
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        path
    }

    fn sample_manifest() -> ReleaseManifest {
        ReleaseManifest {
            version: "1.2.3".to_string(),
            artifacts: vec![Artifact {
                id: "artifact-1".to_string(),
                url: "https://example.com/artifact".to_string(),
                sha256_hex: None,
                signature_b64: None,
                public_key_hex: None,
            }],
        }
    }

    #[tokio::test]
    async fn run_update_cycle_returns_applied_when_everything_succeeds() {
        let rollback_called = Arc::new(AtomicBool::new(false));
        let coordinator = UpdateCoordinator::new(
            UpdateConfig {
                state_path: state_file("applied"),
                ..UpdateConfig::default()
            },
            Arc::new(StubManifestFetcher {
                manifest: sample_manifest(),
            }),
            Arc::new(StubPolicyEngine {
                decision: PolicyDecision::Approved,
            }),
            Arc::new(StubDownloader),
            Arc::new(StubVerifier { should_fail: false }),
            Arc::new(StubApplier {
                should_fail_apply: false,
                rollback_called: rollback_called.clone(),
            }),
        );

        let outcome = coordinator.run_update_cycle().await.expect("cycle should run");
        assert_eq!(outcome, UpdateOutcome::Applied("1.2.3".to_string()));
        assert!(!rollback_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn run_update_cycle_returns_deferred_when_policy_defers() {
        let retry_after = Utc::now() + Duration::hours(4);
        let coordinator = UpdateCoordinator::new(
            UpdateConfig {
                state_path: state_file("deferred"),
                ..UpdateConfig::default()
            },
            Arc::new(StubManifestFetcher {
                manifest: sample_manifest(),
            }),
            Arc::new(StubPolicyEngine {
                decision: PolicyDecision::Deferred {
                    reason: "maintenance window".to_string(),
                    retry_after,
                },
            }),
            Arc::new(StubDownloader),
            Arc::new(StubVerifier { should_fail: false }),
            Arc::new(StubApplier {
                should_fail_apply: false,
                rollback_called: Arc::new(AtomicBool::new(false)),
            }),
        );

        let outcome = coordinator.run_update_cycle().await.expect("cycle should run");
        assert_eq!(
            outcome,
            UpdateOutcome::Deferred {
                reason: "maintenance window".to_string(),
                retry_after,
            }
        );
    }

    #[tokio::test]
    async fn run_update_cycle_returns_blocked_when_policy_blocks() {
        let coordinator = UpdateCoordinator::new(
            UpdateConfig {
                state_path: state_file("blocked"),
                ..UpdateConfig::default()
            },
            Arc::new(StubManifestFetcher {
                manifest: sample_manifest(),
            }),
            Arc::new(StubPolicyEngine {
                decision: PolicyDecision::Blocked {
                    reason: "fleet freeze".to_string(),
                },
            }),
            Arc::new(StubDownloader),
            Arc::new(StubVerifier { should_fail: false }),
            Arc::new(StubApplier {
                should_fail_apply: false,
                rollback_called: Arc::new(AtomicBool::new(false)),
            }),
        );

        let outcome = coordinator.run_update_cycle().await.expect("cycle should run");
        assert_eq!(
            outcome,
            UpdateOutcome::Blocked {
                reason: "fleet freeze".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn run_update_cycle_rolls_back_and_returns_failed_when_apply_fails() {
        let rollback_called = Arc::new(AtomicBool::new(false));
        let coordinator = UpdateCoordinator::new(
            UpdateConfig {
                state_path: state_file("apply_failed"),
                ..UpdateConfig::default()
            },
            Arc::new(StubManifestFetcher {
                manifest: sample_manifest(),
            }),
            Arc::new(StubPolicyEngine {
                decision: PolicyDecision::Approved,
            }),
            Arc::new(StubDownloader),
            Arc::new(StubVerifier { should_fail: false }),
            Arc::new(StubApplier {
                should_fail_apply: true,
                rollback_called: rollback_called.clone(),
            }),
        );

        let outcome = coordinator.run_update_cycle().await.expect("cycle should run");
        match outcome {
            UpdateOutcome::Failed { error } => {
                assert!(error.contains("apply failed"));
            }
            other => panic!("expected failed outcome, got {other:?}"),
        }
        assert!(rollback_called.load(Ordering::SeqCst));
    }
}
