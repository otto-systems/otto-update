use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::extract::{ConnectInfo, State};
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateLifecycleState {
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

#[derive(Debug, Clone)]
pub struct SafetyConfig {
    pub allowed_ips: Vec<IpAddr>,
    pub mutating_limit_per_minute: usize,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            allowed_ips: vec![
                IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
            ],
            mutating_limit_per_minute: 10,
        }
    }
}

#[derive(Clone)]
pub struct SafetyMiddlewareState {
    pub config: SafetyConfig,
    pub provider: Arc<dyn SafetyStateProvider>,
    pub limiter: Arc<LeakyBucketLimiter>,
}

impl SafetyMiddlewareState {
    pub fn new(config: SafetyConfig, provider: Arc<dyn SafetyStateProvider>) -> Self {
        Self {
            limiter: Arc::new(LeakyBucketLimiter::new(
                config.mutating_limit_per_minute,
                Duration::from_secs(60),
            )),
            config,
            provider,
        }
    }
}

#[async_trait]
pub trait SafetyStateProvider: Send + Sync {
    async fn current_update_state(&self) -> UpdateLifecycleState;
    async fn is_safe_to_update(&self) -> bool;
    async fn safety_reasons(&self) -> Vec<String>;
    async fn is_check_in_progress(&self) -> bool;
}

pub async fn safety_middleware(
    State(state): State<SafetyMiddlewareState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().clone();

    if !is_mutating(&method, &path) {
        return next.run(request).await;
    }

    let ip = connect_info
        .map(|info| info.0.ip())
        .or_else(|| request.extensions().get::<ConnectInfo<SocketAddr>>().map(|x| x.0.ip()));

    let Some(ip) = ip else {
        return rejection(StatusCode::FORBIDDEN, "remote_access_denied");
    };

    if !state.config.allowed_ips.contains(&ip) {
        return rejection(StatusCode::FORBIDDEN, "remote_access_denied");
    }

    if let Err(retry_after_secs) = state.limiter.try_acquire(ip).await {
        let mut response = rejection(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
        response.headers_mut().insert(
            "Retry-After",
            retry_after_secs
                .to_string()
                .parse()
                .unwrap_or_else(|_| "1".parse().expect("static value is valid")),
        );
        return response;
    }

    if path == "/v1/check" && method == Method::POST {
        if state.provider.is_check_in_progress().await {
            return rejection(StatusCode::CONFLICT, "already_checking");
        }
    }

    if (path == "/v1/approve" || path == "/v1/defer") && method == Method::POST {
        let update_state = state.provider.current_update_state().await;
        if update_state != UpdateLifecycleState::PolicyEvaluating {
            return state_rejection(update_state);
        }
    }

    if path == "/v1/approve" && method == Method::POST {
        if !state.provider.is_safe_to_update().await {
            let reasons = state.provider.safety_reasons().await;
            let reason = reasons
                .first()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            return rejection(StatusCode::CONFLICT, &reason);
        }
    }

    next.run(request).await
}

fn is_mutating(method: &Method, path: &str) -> bool {
    matches!(
        (method, path),
        (&Method::POST, "/v1/check")
            | (&Method::POST, "/v1/approve")
            | (&Method::POST, "/v1/defer")
            | (&Method::POST, "/v1/rollback")
            | (&Method::PUT, "/v1/config")
    )
}

fn state_rejection(state: UpdateLifecycleState) -> Response {
    let body = ErrorBodyWithState {
        error: "invalid_state".to_string(),
        current_state: Some(format_state(&state).to_string()),
    };
    (StatusCode::CONFLICT, axum::Json(body)).into_response()
}

fn rejection(status: StatusCode, reason: &str) -> Response {
    let body = ErrorBodyWithState {
        error: reason.to_string(),
        current_state: None,
    };
    (status, axum::Json(body)).into_response()
}

fn format_state(state: &UpdateLifecycleState) -> &'static str {
    match state {
        UpdateLifecycleState::Idle => "idle",
        UpdateLifecycleState::Checking => "checking",
        UpdateLifecycleState::ManifestFetched => "manifest_fetched",
        UpdateLifecycleState::PolicyEvaluating => "policy_evaluating",
        UpdateLifecycleState::Approved => "approved",
        UpdateLifecycleState::Deferred => "deferred",
        UpdateLifecycleState::Blocked => "blocked",
        UpdateLifecycleState::Downloading => "downloading",
        UpdateLifecycleState::Verifying => "verifying",
        UpdateLifecycleState::ReadyToApply => "ready_to_apply",
        UpdateLifecycleState::Applying => "applying",
        UpdateLifecycleState::Applied => "applied",
        UpdateLifecycleState::RolledBack => "rolled_back",
        UpdateLifecycleState::Failed => "failed",
    }
}

#[derive(Debug, Serialize)]
struct ErrorBodyWithState {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_state: Option<String>,
}

#[derive(Debug)]
pub struct LeakyBucketLimiter {
    capacity: usize,
    window: Duration,
    buckets: Mutex<HashMap<IpAddr, VecDeque<Instant>>>,
}

impl LeakyBucketLimiter {
    pub fn new(capacity: usize, window: Duration) -> Self {
        Self {
            capacity,
            window,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    pub async fn try_acquire(&self, ip: IpAddr) -> Result<(), u64> {
        let mut guard = self.buckets.lock().await;
        let now = Instant::now();
        let queue = guard.entry(ip).or_insert_with(VecDeque::new);

        while let Some(ts) = queue.front().copied() {
            if now.duration_since(ts) >= self.window {
                queue.pop_front();
            } else {
                break;
            }
        }

        if queue.len() >= self.capacity {
            if let Some(oldest) = queue.front() {
                let retry_after = self
                    .window
                    .saturating_sub(now.duration_since(*oldest))
                    .as_secs()
                    .max(1);
                return Err(retry_after);
            }
            return Err(1);
        }

        queue.push_back(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::str::FromStr;

    use async_trait::async_trait;
    use axum::extract::State;
    use axum::http::{Method, Request, StatusCode};
    use axum::middleware::from_fn_with_state;
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;

    use super::*;

    #[derive(Clone)]
    struct TestProvider {
        state: UpdateLifecycleState,
        safe: bool,
        checking: bool,
        reasons: Vec<String>,
    }

    #[async_trait]
    impl SafetyStateProvider for TestProvider {
        async fn current_update_state(&self) -> UpdateLifecycleState {
            self.state.clone()
        }

        async fn is_safe_to_update(&self) -> bool {
            self.safe
        }

        async fn safety_reasons(&self) -> Vec<String> {
            self.reasons.clone()
        }

        async fn is_check_in_progress(&self) -> bool {
            self.checking
        }
    }

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn app_with_provider(provider: Arc<dyn SafetyStateProvider>) -> Router {
        let state = SafetyMiddlewareState::new(SafetyConfig::default(), provider);
        Router::new()
            .route("/health", get(ok_handler))
            .route("/v1/check", post(ok_handler))
            .route("/v1/approve", post(ok_handler))
            .layer(from_fn_with_state(state.clone(), safety_middleware))
            .with_state(state)
    }

    #[tokio::test]
    async fn limiter_blocks_after_capacity() {
        let limiter = LeakyBucketLimiter::new(2, Duration::from_secs(60));
        let ip = IpAddr::from_str("127.0.0.1").expect("valid ip");

        assert!(limiter.try_acquire(ip).await.is_ok());
        assert!(limiter.try_acquire(ip).await.is_ok());
        assert!(limiter.try_acquire(ip).await.is_err());
    }

    #[tokio::test]
    async fn safe_endpoint_bypasses_mutating_guards() {
        let provider: Arc<dyn SafetyStateProvider> = Arc::new(TestProvider {
            state: UpdateLifecycleState::Idle,
            safe: false,
            checking: true,
            reasons: vec!["busy".to_string()],
        });
        let app = app_with_provider(provider);

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .header("x-forwarded-for", "127.0.0.1")
                    .body(axum::body::Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("response expected");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn check_endpoint_rejects_when_check_already_in_progress() {
        let provider: Arc<dyn SafetyStateProvider> = Arc::new(TestProvider {
            state: UpdateLifecycleState::Checking,
            safe: true,
            checking: true,
            reasons: vec![],
        });
        let app = app_with_provider(provider);

        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/v1/check")
            .body(axum::body::Body::empty())
            .expect("request should build");
        request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 7000))));

        let response = app.oneshot(request).await.expect("response expected");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn approve_endpoint_rejects_when_not_policy_evaluating() {
        let provider: Arc<dyn SafetyStateProvider> = Arc::new(TestProvider {
            state: UpdateLifecycleState::Idle,
            safe: true,
            checking: false,
            reasons: vec![],
        });
        let app = app_with_provider(provider);

        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/v1/approve")
            .body(axum::body::Body::empty())
            .expect("request should build");
        request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 7001))));

        let response = app.oneshot(request).await.expect("response expected");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn approve_endpoint_rejects_when_device_not_safe() {
        let provider: Arc<dyn SafetyStateProvider> = Arc::new(TestProvider {
            state: UpdateLifecycleState::PolicyEvaluating,
            safe: false,
            checking: false,
            reasons: vec!["battery_low".to_string()],
        });
        let app = app_with_provider(provider);

        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/v1/approve")
            .body(axum::body::Body::empty())
            .expect("request should build");
        request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 7002))));

        let response = app.oneshot(request).await.expect("response expected");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }
}
