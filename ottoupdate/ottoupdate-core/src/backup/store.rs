use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BackupStore {
    backup_dir: PathBuf,
    install_dir: PathBuf,
    history_path: PathBuf,
    max_backups: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: Uuid,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub path: PathBuf,
    pub file_count: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BackupIndex {
    records: Vec<BackupRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    record: BackupRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RestoreEvent {
    event_id: Uuid,
    version: String,
    outcome: String,
    recorded_at: DateTime<Utc>,
    reason: Option<String>,
}

impl BackupStore {
    pub fn new(backup_dir: PathBuf, install_dir: PathBuf, history_path: PathBuf) -> Self {
        Self {
            backup_dir,
            install_dir,
            history_path,
            max_backups: 3,
        }
    }

    pub fn with_max_backups(mut self, max_backups: usize) -> Self {
        self.max_backups = max_backups.max(1);
        self
    }

    pub async fn create_backup(&self, version: &str) -> Result<BackupRecord> {
        tokio::fs::create_dir_all(&self.backup_dir)
            .await
            .with_context(|| format!("failed to create {}", self.backup_dir.display()))?;

        let backup_id = Uuid::new_v4();
        let target_dir = self.backup_dir.join(backup_id.to_string());
        tokio::fs::create_dir_all(&target_dir)
            .await
            .with_context(|| format!("failed to create {}", target_dir.display()))?;

        let files = collect_files_recursive(&self.install_dir).await?;
        copy_files_parallel(&files, &self.install_dir, &target_dir, 4).await?;

        let sha = hash_directory_contents(&target_dir).await?;

        let record = BackupRecord {
            id: backup_id,
            version: version.to_string(),
            created_at: Utc::now(),
            path: target_dir.clone(),
            file_count: files.len(),
            sha256: sha,
        };

        self.write_backup_manifest(&record).await?;
        self.update_index(record.clone()).await?;
        self.prune_old_backups().await?;

        Ok(record)
    }

    pub async fn restore(&self, backup_id: Uuid) -> Result<()> {
        let index = self.read_index().await?;
        let record = index
            .records
            .iter()
            .find(|r| r.id == backup_id)
            .cloned()
            .context("backup id not found")?;

        let actual_sha = hash_directory_contents(&record.path).await?;
        if actual_sha != record.sha256 {
            anyhow::bail!("backup checksum mismatch");
        }

        let backup_of_current = self
            .install_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("install.swap.{}", Uuid::new_v4()));

        if tokio::fs::metadata(&self.install_dir).await.is_ok() {
            tokio::fs::rename(&self.install_dir, &backup_of_current)
                .await
                .with_context(|| {
                    format!(
                        "failed to move current install {} to {}",
                        self.install_dir.display(),
                        backup_of_current.display()
                    )
                })?;
        }

        tokio::fs::create_dir_all(&self.install_dir).await.with_context(|| {
            format!("failed to re-create install dir {}", self.install_dir.display())
        })?;

        let files = collect_files_recursive(&record.path).await?;
        copy_files_parallel(&files, &record.path, &self.install_dir, 4).await?;

        let _ = tokio::fs::remove_dir_all(&backup_of_current).await;

        self.append_restore_event(&record.version).await?;
        Ok(())
    }

    pub async fn list_backups(&self) -> Result<Vec<BackupRecord>> {
        Ok(self.read_index().await?.records)
    }

    async fn update_index(&self, record: BackupRecord) -> Result<()> {
        let mut index = self.read_index().await.unwrap_or_default();
        index.records.push(record);
        index
            .records
            .sort_by(|a, b| b.created_at.cmp(&a.created_at));

        write_json_pretty(&self.index_path(), &index).await
    }

    async fn read_index(&self) -> Result<BackupIndex> {
        match tokio::fs::read_to_string(self.index_path()).await {
            Ok(raw) => Ok(serde_json::from_str::<BackupIndex>(&raw)
                .context("failed to parse backup_index.json")?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(BackupIndex::default()),
            Err(err) => Err(err).context("failed to read backup_index.json"),
        }
    }

    async fn write_backup_manifest(&self, record: &BackupRecord) -> Result<()> {
        let manifest = BackupManifest {
            record: record.clone(),
        };
        write_json_pretty(&record.path.join("backup_manifest.json"), &manifest).await
    }

    async fn prune_old_backups(&self) -> Result<()> {
        let mut index = self.read_index().await?;
        if index.records.len() <= self.max_backups {
            return Ok(());
        }

        let removed = index.records.split_off(self.max_backups);
        for record in removed {
            let _ = tokio::fs::remove_dir_all(record.path).await;
        }

        write_json_pretty(&self.index_path(), &index).await
    }

    async fn append_restore_event(&self, version: &str) -> Result<()> {
        let mut events: Vec<RestoreEvent> = match tokio::fs::read_to_string(&self.history_path).await {
            Ok(raw) => serde_json::from_str(&raw).context("failed to parse update_history.json")?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => return Err(err).context("failed to read update_history.json"),
        };

        events.push(RestoreEvent {
            event_id: Uuid::new_v4(),
            version: version.to_string(),
            outcome: "rolled_back".to_string(),
            recorded_at: Utc::now(),
            reason: Some("restore_executed".to_string()),
        });

        write_json_pretty(&self.history_path, &events).await
    }

    fn index_path(&self) -> PathBuf {
        self.backup_dir.join("backup_index.json")
    }
}

async fn collect_files_recursive(base: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![base.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err).with_context(|| format!("failed reading {}", dir.display())),
        };

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| format!("failed iterating {}", dir.display()))?
        {
            let path = entry.path();
            let meta = entry
                .metadata()
                .await
                .with_context(|| format!("failed to stat {}", path.display()))?;
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                files.push(path);
            }
        }
    }

    Ok(files)
}

async fn copy_files_parallel(
    files: &[PathBuf],
    source_root: &Path,
    target_root: &Path,
    concurrency: usize,
) -> Result<()> {
    let sem = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut handles = Vec::with_capacity(files.len());

    for src in files {
        let permit = sem.clone().acquire_owned().await.context("failed to acquire semaphore")?;
        let src = src.clone();
        let rel = src
            .strip_prefix(source_root)
            .with_context(|| {
                format!(
                    "failed stripping source root {} from {}",
                    source_root.display(),
                    src.display()
                )
            })?
            .to_path_buf();
        let dst = target_root.join(rel);

        handles.push(tokio::spawn(async move {
            let _permit = permit;
            if let Some(parent) = dst.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("failed creating {}", parent.display()))?;
            }

            tokio::fs::copy(&src, &dst)
                .await
                .with_context(|| format!("failed copying {} -> {}", src.display(), dst.display()))?;
            Ok::<(), anyhow::Error>(())
        }));
    }

    for handle in handles {
        handle.await.context("copy task join failed")??;
    }

    Ok(())
}

async fn hash_directory_contents(path: &Path) -> Result<String> {
    let mut files = collect_files_recursive(path).await?;
    files.sort();

    let mut hasher = Sha256::new();
    for file in files {
        let rel = file
            .strip_prefix(path)
            .unwrap_or(file.as_path())
            .to_string_lossy();
        hasher.update(rel.as_bytes());

        let content = tokio::fs::read(&file)
            .await
            .with_context(|| format!("failed reading {}", file.display()))?;
        hasher.update(&content);
    }

    Ok(hex::encode(hasher.finalize()))
}

async fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(value).context("failed to serialize json")?;
    tokio::fs::write(path, payload)
        .await
        .with_context(|| format!("failed writing {}", path.display()))?;
    Ok(())
}
