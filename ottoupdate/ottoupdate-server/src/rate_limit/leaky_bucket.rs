use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::future::BoxFuture;
use tower::{Layer, Service};

#[derive(Debug)]
struct BucketState {
    current_fill: f64,
    last_leak: Instant,
}

#[derive(Debug)]
pub struct LeakyBucket {
    capacity: f64,
    leak_rate_per_sec: f64,
    state: tokio::sync::Mutex<BucketState>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitError {
    Throttled { retry_after_secs: f64 },
}

impl LeakyBucket {
    pub fn new(capacity: f64, leak_rate_per_sec: f64) -> Self {
        Self {
            capacity,
            leak_rate_per_sec,
            state: tokio::sync::Mutex::new(BucketState {
                current_fill: 0.0,
                last_leak: Instant::now(),
            }),
        }
    }

    pub async fn try_acquire(&self) -> Result<(), RateLimitError> {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_leak).as_secs_f64();

        let leaked = elapsed * self.leak_rate_per_sec;
        state.current_fill = (state.current_fill - leaked).max(0.0);
        state.last_leak = now;

        if state.current_fill + 1.0 > self.capacity {
            let wait_secs = ((state.current_fill - self.capacity + 1.0) / self.leak_rate_per_sec).max(0.0);
            return Err(RateLimitError::Throttled {
                retry_after_secs: wait_secs,
            });
        }

        state.current_fill += 1.0;
        Ok(())
    }

    pub async fn current_fill(&self) -> f64 {
        let state = self.state.lock().await;
        state.current_fill
    }
}

#[derive(Clone)]
pub struct RateLimitLayer {
    bucket: Arc<LeakyBucket>,
}

impl RateLimitLayer {
    pub fn new(bucket: Arc<LeakyBucket>) -> Self {
        Self { bucket }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            bucket: self.bucket.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    bucket: Arc<LeakyBucket>,
}

impl<S, ReqBody> Service<axum::http::Request<ReqBody>> for RateLimitService<S>
where
    ReqBody: Send + 'static,
    S: Service<axum::http::Request<ReqBody>, Response = axum::http::Response<axum::body::Body>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = axum::http::Response<axum::body::Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<ReqBody>) -> Self::Future {
        let bucket = self.bucket.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            match bucket.try_acquire().await {
                Ok(()) => inner.call(req).await,
                Err(RateLimitError::Throttled { retry_after_secs }) => {
                    let retry_after = retry_after_secs.ceil().max(1.0) as u64;
                    let remaining = (bucket.capacity - bucket.current_fill().await).max(0.0).floor() as u64;

                    let response = axum::http::Response::builder()
                        .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
                        .header("Retry-After", retry_after.to_string())
                        .header("X-RateLimit-Limit", bucket.capacity as u64)
                        .header("X-RateLimit-Remaining", remaining)
                        .body(axum::body::Body::from("rate_limited"))
                        .expect("static 429 response should build");
                    Ok(response)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn burst_test_rejects_after_capacity() {
        let bucket = LeakyBucket::new(3.0, 0.1);

        assert!(bucket.try_acquire().await.is_ok());
        assert!(bucket.try_acquire().await.is_ok());
        assert!(bucket.try_acquire().await.is_ok());
        assert!(bucket.try_acquire().await.is_err());
    }

    #[tokio::test]
    async fn drain_test_allows_after_time_leak() {
        let bucket = LeakyBucket::new(1.0, 10.0);

        assert!(bucket.try_acquire().await.is_ok());
        assert!(bucket.try_acquire().await.is_err());

        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(bucket.try_acquire().await.is_ok());
    }

    #[tokio::test]
    async fn concurrency_test_caps_total_accepts() {
        let bucket = Arc::new(LeakyBucket::new(10.0, 0.0));
        let mut handles = Vec::new();

        for _ in 0..50 {
            let bucket = bucket.clone();
            handles.push(tokio::spawn(async move { bucket.try_acquire().await.is_ok() }));
        }

        let mut accepted = 0usize;
        for handle in handles {
            if handle.await.expect("task should join") {
                accepted += 1;
            }
        }

        assert!(accepted <= 10);
    }
}
