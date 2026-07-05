use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use anyhow::{anyhow, Result};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tracing::{debug, instrument};

use crate::traits::Artifact;

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download(&self, artifact: &Artifact) -> Result<PathBuf>;
}

pub struct StreamingDownloader {
    client: reqwest::Client,
    output_dir: PathBuf,
}

impl StreamingDownloader {
    pub fn new(output_dir: impl Into<PathBuf>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("ottoupdate-core/0.1")
            .build()
            .map_err(|err| anyhow!("failed to build http client: {err}"))?;

        Ok(Self {
            client,
            output_dir: output_dir.into(),
        })
    }

    fn build_output_path(&self, artifact: &Artifact) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        let filename = format!("{}-{ts}.bin", artifact.id);
        Path::new(&self.output_dir).join(filename)
    }
}

#[async_trait]
impl Downloader for StreamingDownloader {
    #[instrument(skip(self), fields(artifact_id = %artifact.id, url = %artifact.url))]
    async fn download(&self, artifact: &Artifact) -> Result<PathBuf> {
        fs::create_dir_all(&self.output_dir).await?;

        let mut response = self
            .client
            .get(&artifact.url)
            .send()
            .await
            .map_err(|err| anyhow!("download request failed: {err}"))?
            .error_for_status()
            .map_err(|err| anyhow!("download request returned error status: {err}"))?;

        let output_path = self.build_output_path(artifact);
        let mut file = File::create(&output_path).await?;
        let mut total_bytes: u64 = 0;

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|err| anyhow!("download stream read failed: {err}"))?
        {
            file.write_all(&chunk).await?;
            total_bytes += chunk.len() as u64;
        }

        file.flush().await?;
        debug!(path = %output_path.display(), total_bytes, "download complete");
        Ok(output_path)
    }
}
