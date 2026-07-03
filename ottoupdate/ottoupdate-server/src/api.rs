use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct HistoryQuery {
    limit: Option<u32>,
    offset: Option<u32>,
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/state", get(state))
        .route("/v1/check", post(check))
        .route("/v1/manifest", get(manifest))
        .route("/v1/policy", get(policy))
        .route("/v1/approve", post(approve))
        .route("/v1/defer", post(defer))
        .route("/v1/progress", get(progress))
        .route("/v1/history", get(history))
        .route("/v1/config", get(config_get).put(config_set))
        .route("/v1/rollback", post(rollback))
        .route("/v1/backups", get(backups))
}

async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": 0
    }))
}

async fn state() -> Json<Value> {
    Json(json!({
        "device_state": {},
        "update_state": {},
        "active_manifest": null
    }))
}

async fn check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "check_id": Uuid::new_v4(),
            "triggered_at": chrono::Utc::now()
        }))
    )
}

async fn manifest() -> Json<Value> {
    Json(json!({
        "version": "0.0.0",
        "artifacts": []
    }))
}

async fn policy() -> Json<Value> {
    Json(json!({
        "decision": "require_approval",
        "reason": "stub_policy_engine",
        "until": null,
        "group": null
    }))
}

async fn approve() -> Json<Value> {
    Json(json!({ "status": "approved" }))
}

async fn defer() -> Json<Value> {
    Json(json!({ "until": chrono::Utc::now() + chrono::Duration::hours(6) }))
}

async fn progress() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

async fn history(Query(query): Query<HistoryQuery>) -> Json<Value> {
    Json(json!({
        "items": [],
        "limit": query.limit.unwrap_or(50),
        "offset": query.offset.unwrap_or(0),
        "total": 0
    }))
}

async fn config_get() -> Json<Value> {
    Json(json!({
        "channel": "stable",
        "auto_check_enabled": true
    }))
}

async fn config_set(Json(body): Json<Value>) -> Json<Value> {
    Json(body)
}

async fn rollback() -> (StatusCode, Json<Value>) {
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "rollback_id": Uuid::new_v4(),
            "triggered_at": chrono::Utc::now()
        }))
    )
}

async fn backups() -> Json<Value> {
    Json(json!({ "items": [] }))
}
