use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::middleware;
use axum::routing::{get, post, put};
use axum::Router;
use tower::ServiceExt;

#[path = "../src/middleware/safety.rs"]
mod safety;

use safety::{SafetyConfig, SafetyMiddlewareState, SafetyStateProvider, UpdateLifecycleState};

#[derive(Clone)]
struct FakeProvider {
    state: UpdateLifecycleState,
    safe: bool,
    reasons: Vec<String>,
    checking: bool,
}

#[async_trait]
impl SafetyStateProvider for FakeProvider {
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

fn app_for(provider: FakeProvider) -> Router {
    let state = SafetyMiddlewareState::new(
        SafetyConfig::default(),
        Arc::new(provider) as Arc<dyn SafetyStateProvider>,
    );

    Router::new()
        .route("/v1/state", get(|| async { "ok" }))
        .route("/v1/check", post(|| async { "ok" }))
        .route("/v1/approve", post(|| async { "ok" }))
        .route("/v1/defer", post(|| async { "ok" }))
        .route("/v1/config", put(|| async { "ok" }))
        .route("/v1/rollback", post(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(
            state,
            safety::safety_middleware,
        ))
}

fn request(method: Method, path: &str, ip: IpAddr) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .expect("request should build");
    request
        .extensions_mut()
        .insert(axum::extract::ConnectInfo(SocketAddr::new(ip, 5000)));
    request
}

#[tokio::test]
async fn origin_restriction_rejects_non_localhost() {
    let app = app_for(FakeProvider {
        state: UpdateLifecycleState::PolicyEvaluating,
        safe: true,
        reasons: vec![],
        checking: false,
    });

    let response = app
        .oneshot(request(
            Method::POST,
            "/v1/check",
            IpAddr::V4(Ipv4Addr::new(10, 1, 1, 7)),
        ))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn state_guard_rejects_approve_when_not_policy_evaluating() {
    let app = app_for(FakeProvider {
        state: UpdateLifecycleState::Idle,
        safe: true,
        reasons: vec![],
        checking: false,
    });

    let response = app
        .oneshot(request(Method::POST, "/v1/approve", IpAddr::V4(Ipv4Addr::LOCALHOST)))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn safety_check_rejects_approve_when_unsafe() {
    let app = app_for(FakeProvider {
        state: UpdateLifecycleState::PolicyEvaluating,
        safe: false,
        reasons: vec!["low_battery".to_string()],
        checking: false,
    });

    let response = app
        .oneshot(request(Method::POST, "/v1/approve", IpAddr::V4(Ipv4Addr::LOCALHOST)))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn idempotency_rejects_when_check_already_running() {
    let app = app_for(FakeProvider {
        state: UpdateLifecycleState::PolicyEvaluating,
        safe: true,
        reasons: vec![],
        checking: true,
    });

    let response = app
        .oneshot(request(Method::POST, "/v1/check", IpAddr::V4(Ipv4Addr::LOCALHOST)))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn rate_limit_rejects_after_burst() {
    let app = app_for(FakeProvider {
        state: UpdateLifecycleState::PolicyEvaluating,
        safe: true,
        reasons: vec![],
        checking: false,
    });

    for _ in 0..10 {
        let response = app
            .clone()
            .oneshot(request(Method::POST, "/v1/check", IpAddr::V4(Ipv4Addr::LOCALHOST)))
            .await
            .expect("router should respond");
        assert!(response.status().is_success() || response.status() == StatusCode::CONFLICT);
    }

    let response = app
        .oneshot(request(Method::POST, "/v1/check", IpAddr::V4(Ipv4Addr::LOCALHOST)))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}
