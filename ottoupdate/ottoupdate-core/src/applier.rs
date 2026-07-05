use std::path::{Path, PathBuf};

use async_trait::async_trait;
use anyhow::{anyhow, Result};
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::traits::ReleaseManifest;

#[async_trait]
pub trait Applier: Send + Sync {
    async fn apply(&self, path: &Path, manifest: &ReleaseManifest) -> Result<()>;
    async fn rollback(&self) -> Result<()>;
}

pub struct AtomicApplier {
    install_dir: PathBuf,
    staging_dir: PathBuf,
    backup_dir: PathBuf,
    last_backup_path: Mutex<Option<PathBuf>>,
}

impl AtomicApplier {
    pub fn new(
        install_dir: impl Into<PathBuf>,
        staging_dir: impl Into<PathBuf>,
        backup_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            install_dir: install_dir.into(),
            staging_dir: staging_dir.into(),
            backup_dir: backup_dir.into(),
            last_backup_path: Mutex::new(None),
        }
    }

    fn active_path(&self) -> PathBuf {
        self.install_dir.join("active.bin")
    }

    fn staged_path(&self, version: &str) -> PathBuf {
        self.staging_dir.join(format!("{version}.bin"))
    }

    fn backup_path(&self, version: &str) -> PathBuf {
        self.backup_dir.join(format!("active-{version}.bak"))
    }
}

#[async_trait]
impl Applier for AtomicApplier {
    #[instrument(skip(self), fields(path = %path.display(), version = %manifest.version))]
    async fn apply(&self, path: &Path, manifest: &ReleaseManifest) -> Result<()> {
        fs::create_dir_all(&self.install_dir).await?;
        fs::create_dir_all(&self.staging_dir).await?;
        fs::create_dir_all(&self.backup_dir).await?;

        let staged = self.staged_path(&manifest.version);
        fs::copy(path, &staged)
            .await
            .map_err(|err| anyhow!("failed to stage artifact: {err}"))?;

        let active = self.active_path();
        let backup = self.backup_path(&manifest.version);
        if fs::try_exists(&active).await.unwrap_or(false) {
            fs::rename(&active, &backup)
                .await
                .map_err(|err| anyhow!("failed to backup active install: {err}"))?;
            let mut guard = self.last_backup_path.lock().await;
            *guard = Some(backup.clone());
        }

        fs::rename(&staged, &active)
            .await
            .map_err(|err| anyhow!("failed to atomically swap staged artifact into active: {err}"))?;

        debug!(active = %active.display(), "atomic apply completed");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn rollback(&self) -> Result<()> {
        let backup = {
            let guard = self.last_backup_path.lock().await;
            guard.clone()
        };

        let Some(backup) = backup else {
            return Err(anyhow!("no backup available for rollback"));
        };

        let active = self.active_path();
        if fs::try_exists(&active).await.unwrap_or(false) {
            fs::remove_file(&active)
                .await
                .map_err(|err| anyhow!("failed to remove active artifact before rollback: {err}"))?;
        }

        fs::rename(&backup, &active)
            .await
            .map_err(|err| anyhow!("failed to restore backup: {err}"))?;

        debug!(active = %active.display(), "rollback completed");
        Ok(())
    }
}
