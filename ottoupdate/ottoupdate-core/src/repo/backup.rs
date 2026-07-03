use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{error, warn};

use crate::traits::ReleaseManifest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttemptOutcome {
    Success,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAttempt {
    pub source_url: String,
    pub attempted_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub outcome: AttemptOutcome,
}

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("all sources failed")]
    AllSourcesFailed { attempts: Vec<SourceAttempt> },
    #[error("cache read/write failed: {0}")]
    CacheIo(String),
    #[error("cache parse failed: {0}")]
    CacheParse(String),
}

#[derive(Debug, Clone)]
pub struct BackupManifestRepo {
    primary: String,
    secondaries: Vec<String>,
    cache_ttl_secs: i64,
    cache_path: PathBuf,
    client: Arc<dyn ManifestSourceClient>,
    cache: Arc<Mutex<Option<CachedManifest>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedManifest {
    fetched_at: DateTime<Utc>,
    manifest: ReleaseManifest,
}

#[derive(Debug, Clone)]
pub struct RepoConfig {
    pub primary: String,
    pub secondaries: Vec<String>,
    pub cache_ttl_secs: i64,
    pub cache_path: PathBuf,
}

impl RepoConfig {
    pub fn new(primary: impl Into<String>, secondaries: Vec<String>, cache_path: PathBuf) -> Self {
        Self {
            primary: primary.into(),
            secondaries,
            cache_ttl_secs: 3600,
            cache_path,
        }
    }
}

#[async_trait]
pub trait ManifestSourceClient: Send + Sync {
    async fn fetch_with_timeout(&self, url: &str, timeout_secs: u64) -> Result<ReleaseManifest>;
}

#[derive(Debug, Clone, Default)]
pub struct HttpManifestSourceClient {
    http: reqwest::Client,
}

#[async_trait]
impl ManifestSourceClient for HttpManifestSourceClient {
    async fn fetch_with_timeout(&self, url: &str, timeout_secs: u64) -> Result<ReleaseManifest> {
        let response = self
            .http
            .get(url)
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json::<ReleaseManifest>().await?)
    }
}

impl BackupManifestRepo {
    pub async fn new(config: RepoConfig) -> Result<Self, RepoError> {
        Self::with_client(config, Arc::new(HttpManifestSourceClient::default())).await
    }

    pub async fn with_client(
        config: RepoConfig,
        client: Arc<dyn ManifestSourceClient>,
    ) -> Result<Self, RepoError> {
        let repo = Self {
            primary: config.primary,
            secondaries: config.secondaries,
            cache_ttl_secs: config.cache_ttl_secs,
            cache_path: config.cache_path,
            client,
            cache: Arc::new(Mutex::new(None)),
        };

        repo.load_cache_from_disk().await?;
        Ok(repo)
    }

    pub async fn fetch_manifest(&self) -> Result<ReleaseManifest, RepoError> {
        let mut attempts = Vec::new();

        let mut sources = Vec::with_capacity(1 + self.secondaries.len());
        sources.push(self.primary.clone());
        sources.extend(self.secondaries.clone());

        for (idx, source) in sources.iter().enumerate() {
            let started = Instant::now();
            let attempted_at = Utc::now();

            match self.client.fetch_with_timeout(source, 10).await {
                Ok(manifest) => {
                    let duration_ms = started.elapsed().as_millis() as u64;
                    attempts.push(SourceAttempt {
                        source_url: source.clone(),
                        attempted_at,
                        duration_ms,
                        outcome: AttemptOutcome::Success,
                    });

                    if idx > 0 {
                        warn!(source = %source, "primary failed; fallback source succeeded");
                    }

                    self.update_cache(manifest.clone()).await?;
                    return Ok(manifest);
                }
                Err(err) => {
                    let duration_ms = started.elapsed().as_millis() as u64;
                    attempts.push(SourceAttempt {
                        source_url: source.clone(),
                        attempted_at,
                        duration_ms,
                        outcome: AttemptOutcome::Error(err.to_string()),
                    });
                }
            }
        }

        if let Some(manifest) = self.try_fresh_cache().await {
            warn!("all network sources failed; serving fresh cached manifest");
            return Ok(manifest);
        }

        error!(attempts = attempts.len(), "all sources failed and cache unavailable/stale");
        Err(RepoError::AllSourcesFailed { attempts })
    }

    async fn try_fresh_cache(&self) -> Option<ReleaseManifest> {
        let cache = self.cache.lock().await;
        let entry = cache.as_ref()?;

        let age_secs = (Utc::now() - entry.fetched_at).num_seconds();
        if age_secs < self.cache_ttl_secs {
            return Some(entry.manifest.clone());
        }

        None
    }

    async fn update_cache(&self, manifest: ReleaseManifest) -> Result<(), RepoError> {
        let cached = CachedManifest {
            fetched_at: Utc::now(),
            manifest,
        };

        {
            let mut guard = self.cache.lock().await;
            *guard = Some(cached.clone());
        }

        if let Some(parent) = self.cache_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| RepoError::CacheIo(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(&cached)
            .map_err(|e| RepoError::CacheParse(e.to_string()))?;
        tokio::fs::write(&self.cache_path, json)
            .await
            .map_err(|e| RepoError::CacheIo(e.to_string()))?;

        Ok(())
    }

    async fn load_cache_from_disk(&self) -> Result<(), RepoError> {
        let raw = match tokio::fs::read_to_string(&self.cache_path).await {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(RepoError::CacheIo(err.to_string())),
        };

        let parsed = serde_json::from_str::<CachedManifest>(&raw)
            .map_err(|e| RepoError::CacheParse(e.to_string()))?;

        let mut guard = self.cache.lock().await;
        *guard = Some(parsed);
        Ok(())
    }

    pub async fn cache_age(&self) -> Option<Duration> {
        let guard = self.cache.lock().await;
        guard
            .as_ref()
            .map(|entry| Utc::now().signed_duration_since(entry.fetched_at))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Default)]
    struct FakeClient {
        responses: HashMap<String, Result<ReleaseManifest>>,
    }

    #[async_trait]
    impl ManifestSourceClient for FakeClient {
        async fn fetch_with_timeout(&self, url: &str, _timeout_secs: u64) -> Result<ReleaseManifest> {
            self.responses
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err(anyhow::anyhow!("missing url")))
        }
    }

    fn sample_manifest(version: &str) -> ReleaseManifest {
        ReleaseManifest {
            version: version.to_string(),
            artifacts: vec![],
        }
    }

    #[tokio::test]
    async fn falls_back_to_secondary_when_primary_fails() {
        let primary = "https://primary.example/manifest.json".to_string();
        let secondary = "https://secondary.example/manifest.json".to_string();

        let mut fake = FakeClient::default();
        fake.responses.insert(
            primary.clone(),
            Err(anyhow::anyhow!("primary down")),
        );
        fake.responses
            .insert(secondary.clone(), Ok(sample_manifest("1.2.3")));

        let temp = tempfile::tempdir().expect("tempdir should be created");
        let repo = BackupManifestRepo::with_client(
            RepoConfig {
                primary,
                secondaries: vec![secondary],
                cache_ttl_secs: 3600,
                cache_path: temp.path().join("manifest_cache.json"),
            },
            Arc::new(fake),
        )
        .await
        .expect("repo should initialize");

        let manifest = repo.fetch_manifest().await.expect("fetch should succeed");
        assert_eq!(manifest.version, "1.2.3");
    }

    #[tokio::test]
    async fn serves_fresh_cache_when_network_sources_fail() {
        let primary = "https://primary.example/manifest.json".to_string();

        let mut fake = FakeClient::default();
        fake.responses.insert(
            primary.clone(),
            Err(anyhow::anyhow!("primary down")),
        );

        let temp = tempfile::tempdir().expect("tempdir should be created");
        let cache_path = temp.path().join("manifest_cache.json");

        let cached = CachedManifest {
            fetched_at: Utc::now(),
            manifest: sample_manifest("2.0.0"),
        };
        let payload = serde_json::to_string_pretty(&cached).expect("cache should serialize");
        tokio::fs::write(&cache_path, payload)
            .await
            .expect("cache should write");

        let repo = BackupManifestRepo::with_client(
            RepoConfig {
                primary,
                secondaries: vec![],
                cache_ttl_secs: 3600,
                cache_path,
            },
            Arc::new(fake),
        )
        .await
        .expect("repo should initialize");

        let manifest = repo.fetch_manifest().await.expect("fetch should use cache");
        assert_eq!(manifest.version, "2.0.0");
    }
}
