use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateState {
    Idle,
    Checking,
    ManifestFetched,
    PolicyEvaluating,
    Approved,
    Deferred,
    Blocked,
    Downloading,
    Verifying,
    ReadyToApply,
    Applying,
    Applied,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateEvent {
    CheckTriggered,
    ManifestReceived,
    PolicyApproved,
    PolicyDeferred,
    PolicyBlocked,
    DownloadComplete,
    VerificationPassed,
    VerificationFailed,
    ApplyConfirmed,
    ApplySucceeded,
    ApplyFailed,
    RollbackTriggered,
    RollbackSucceeded,
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("invalid transition: {from:?} + {event:?}")]
    InvalidTransition {
        from: UpdateState,
        event: UpdateEvent,
    },

    #[error("guard rejected transition: {from:?} -> {to:?} ({reason})")]
    GuardRejected {
        from: UpdateState,
        to: UpdateState,
        reason: String,
    },

    #[error("state persistence failed: {0}")]
    Persistence(#[from] std::io::Error),

    #[error("state serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("entry/exit hook failed: {0}")]
    Hook(String),
}

impl UpdateState {
    pub fn transition(self, event: UpdateEvent) -> Result<UpdateState, StateError> {
        let next = match (self, event) {
            (UpdateState::Idle, UpdateEvent::CheckTriggered) => UpdateState::Checking,

            (UpdateState::Checking, UpdateEvent::ManifestReceived) => UpdateState::ManifestFetched,

            (UpdateState::ManifestFetched, UpdateEvent::CheckTriggered) => {
                UpdateState::PolicyEvaluating
            }

            (UpdateState::PolicyEvaluating, UpdateEvent::PolicyApproved) => UpdateState::Approved,
            (UpdateState::PolicyEvaluating, UpdateEvent::PolicyDeferred) => UpdateState::Deferred,
            (UpdateState::PolicyEvaluating, UpdateEvent::PolicyBlocked) => UpdateState::Blocked,

            // A new check can re-open from deferred/blocked.
            (UpdateState::Deferred, UpdateEvent::CheckTriggered) => UpdateState::Checking,
            (UpdateState::Blocked, UpdateEvent::CheckTriggered) => UpdateState::Checking,

            // Guard-restricted transition: only approved policy can enter download phase.
            (UpdateState::Approved, UpdateEvent::CheckTriggered) => UpdateState::Downloading,

            (UpdateState::Downloading, UpdateEvent::DownloadComplete) => UpdateState::Verifying,

            (UpdateState::Verifying, UpdateEvent::VerificationPassed) => UpdateState::ReadyToApply,
            (UpdateState::Verifying, UpdateEvent::VerificationFailed) => UpdateState::Failed,

            (UpdateState::ReadyToApply, UpdateEvent::ApplyConfirmed) => UpdateState::Applying,

            (UpdateState::Applying, UpdateEvent::ApplySucceeded) => UpdateState::Applied,
            (UpdateState::Applying, UpdateEvent::ApplyFailed) => UpdateState::Failed,

            (UpdateState::Failed, UpdateEvent::RollbackTriggered) => UpdateState::RolledBack,
            (UpdateState::Failed, UpdateEvent::RollbackSucceeded) => UpdateState::RolledBack,

            (from, ev) => {
                return Err(StateError::InvalidTransition {
                    from,
                    event: ev,
                });
            }
        };

        Ok(next)
    }
}

#[allow(async_fn_in_trait)]
pub trait StateTransitionHooks: Send + Sync {
    async fn on_exit(
        &self,
        _state: UpdateState,
        _event: UpdateEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn on_enter(
        &self,
        _state: UpdateState,
        _event: UpdateEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub state: UpdateState,
}

pub struct NoopHooks;

impl StateTransitionHooks for NoopHooks {}

pub struct StateMachine {
    state: UpdateState,
    storage_path: PathBuf,
    hooks: Arc<dyn StateTransitionHooks>,
}

pub type SharedStateMachine = Arc<Mutex<StateMachine>>;

impl StateMachine {
    pub fn new(storage_path: impl Into<PathBuf>) -> Self {
        Self {
            state: UpdateState::Idle,
            storage_path: storage_path.into(),
            hooks: Arc::new(NoopHooks),
        }
    }

    pub fn with_hooks(
        storage_path: impl Into<PathBuf>,
        hooks: Arc<dyn StateTransitionHooks>,
    ) -> Self {
        Self {
            state: UpdateState::Idle,
            storage_path: storage_path.into(),
            hooks,
        }
    }

    pub fn shared(storage_path: impl Into<PathBuf>) -> SharedStateMachine {
        Arc::new(Mutex::new(Self::new(storage_path)))
    }

    pub fn shared_with_hooks(
        storage_path: impl Into<PathBuf>,
        hooks: Arc<dyn StateTransitionHooks>,
    ) -> SharedStateMachine {
        Arc::new(Mutex::new(Self::with_hooks(storage_path, hooks)))
    }

    pub fn state(&self) -> UpdateState {
        self.state
    }

    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    pub fn set_hooks(&mut self, hooks: Arc<dyn StateTransitionHooks>) {
        self.hooks = hooks;
    }

    #[instrument(skip(self), fields(current = ?self.state, event = ?event))]
    pub async fn apply_event(&mut self, event: UpdateEvent) -> Result<UpdateState, StateError> {
        let from = self.state;
        let to = from.transition(event)?;

        if from == UpdateState::Blocked && to == UpdateState::Downloading {
            return Err(StateError::GuardRejected {
                from,
                to,
                reason: "cannot download while policy is blocked".to_string(),
            });
        }

        debug!(from = ?from, to = ?to, event = ?event, "executing transition");

        self.hooks
            .on_exit(from, event)
            .await
            .map_err(|err| StateError::Hook(err.to_string()))?;

        self.state = to;

        self.persist().await?;

        self.hooks
            .on_enter(to, event)
            .await
            .map_err(|err| StateError::Hook(err.to_string()))?;

        info!(from = ?from, to = ?to, event = ?event, "state transition committed");

        Ok(to)
    }

    #[instrument(skip(self))]
    pub async fn persist(&self) -> Result<(), StateError> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let payload = StateSnapshot { state: self.state };
        let json = serde_json::to_vec_pretty(&payload)?;
        fs::write(&self.storage_path, json).await?;
        debug!(path = %self.storage_path.display(), state = ?self.state, "state persisted");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn load(&mut self) -> Result<UpdateState, StateError> {
        let bytes = fs::read(&self.storage_path).await?;
        let snapshot: StateSnapshot = serde_json::from_slice(&bytes)?;
        self.state = snapshot.state;
        debug!(path = %self.storage_path.display(), state = ?self.state, "state loaded");
        Ok(self.state)
    }

    pub async fn load_or_default(&mut self) -> Result<UpdateState, StateError> {
        match self.load().await {
            Ok(state) => Ok(state),
            Err(StateError::Persistence(err)) if err.kind() == std::io::ErrorKind::NotFound => {
                self.persist().await?;
                Ok(self.state)
            }
            Err(other) => Err(other),
        }
    }
}
