use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use tracing::{instrument, warn};

use crate::traits::ReleaseManifest;

#[allow(async_fn_in_trait)]
pub trait ManifestFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<ReleaseManifest>;
}

pub struct HttpManifestFetcher {
    client: reqwest::Client,
    request_timeout: Duration,
    max_retries: u32,
}

impl HttpManifestFetcher {
    pub fn new(request_timeout: Duration, max_retries: u32) -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(request_timeout)
            .timeout(request_timeout)
            .user_agent("ottoupdate-core/0.1")
            .build()
            .map_err(|err| anyhow!("failed to build http client: {err}"))?;

        Ok(Self {
            client,
            request_timeout,
            max_retries,
        })
    }

    fn should_retry(status: Option<StatusCode>) -> bool {
        match status {
            Some(code) => code.is_server_error() || code == StatusCode::TOO_MANY_REQUESTS,
            None => true,
        }
    }
}

impl Default for HttpManifestFetcher {
    fn default() -> Self {
        Self::new(Duration::from_secs(10), 2).expect("valid default fetcher config")
    }
}

impl ManifestFetcher for HttpManifestFetcher {
    #[instrument(skip(self), fields(url = %url, timeout_ms = self.request_timeout.as_millis()))]
    async fn fetch(&self, url: &str) -> Result<ReleaseManifest> {
        let attempts = self.max_retries + 1;
        let mut last_err: Option<anyhow::Error> = None;

        for attempt in 1..=attempts {
            let response = self.client.get(url).send().await;
            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let err = anyhow!("manifest fetch failed with status {status}");
                        if attempt < attempts && Self::should_retry(Some(status)) {
                            warn!(attempt, attempts, %status, "manifest fetch failed, retrying");
                            last_err = Some(err);
                            continue;
                        }
                        return Err(err);
                    }

                    let manifest = resp
                        .json::<ReleaseManifest>()
                        .await
                        .map_err(|err| anyhow!("invalid manifest payload: {err}"))?;
                    return Ok(manifest);
                }
                Err(err) => {
                    let wrapped = anyhow!("request failed: {err}");
                    if attempt < attempts && Self::should_retry(None) {
                        warn!(attempt, attempts, error = %wrapped, "manifest request failed, retrying");
                        last_err = Some(wrapped);
                        continue;
                    }
                    return Err(wrapped);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow!("manifest fetch failed")))
    }
}
