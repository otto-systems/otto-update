mod api;
mod config;
mod middleware;
mod platform;
mod rate_limit;

use std::sync::Arc;

use async_trait::async_trait;
use axum::middleware::from_fn_with_state;
use middleware::safety::{
    SafetyConfig, SafetyMiddlewareState, SafetyStateProvider, UpdateLifecycleState,
};
use middleware::security::SecurityConfig;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let provider: Arc<dyn SafetyStateProvider> = Arc::new(StaticSafetyStateProvider::default());
    let safety_state = SafetyMiddlewareState::new(SafetyConfig::default(), provider);

    let config = config::ServerConfig::load_or_default(std::path::Path::new("./config/server.toml"))
        .await
        .unwrap_or_default();
    let security = SecurityConfig {
        bearer_token: config.bearer_token.clone(),
    };
    let app = api::router()
        .layer(from_fn_with_state(security, middleware::security::security_middleware))
        .layer(from_fn_with_state(
            safety_state.clone(),
            middleware::safety::safety_middleware,
        ));

    #[cfg(target_os = "linux")]
    {
        platform::linux::notify_ready().await;
    }

    #[cfg(target_os = "macos")]
    {
        platform::macos::configure_launchd_environment().await;
    }

    let addr = config
        .socket_addr()
        .unwrap_or_else(|_| "127.0.0.1:7430".parse().expect("static address parses"));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("listener should bind");

    info!(bind = %addr, "ottoupdate-server scaffold initialized");

    let server = axum::serve(listener, app).with_graceful_shutdown(graceful_shutdown_signal());
    let _ = server.await;
}

async fn graceful_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            sigterm.recv().await;
            return;
        }
    }

    let _ = tokio::signal::ctrl_c().await;
}

#[derive(Default)]
struct StaticSafetyStateProvider;

#[async_trait]
impl SafetyStateProvider for StaticSafetyStateProvider {
    async fn current_update_state(&self) -> UpdateLifecycleState {
        UpdateLifecycleState::Idle
    }

    async fn is_safe_to_update(&self) -> bool {
        true
    }

    async fn safety_reasons(&self) -> Vec<String> {
        Vec::new()
    }

    async fn is_check_in_progress(&self) -> bool {
        false
    }
}
