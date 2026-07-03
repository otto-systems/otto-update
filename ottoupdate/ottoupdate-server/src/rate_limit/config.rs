use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::rate_limit::leaky_bucket::LeakyBucket;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    pub capacity: f64,
    pub leak_rate_per_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub global: BucketConfig,
    pub per_endpoint: HashMap<String, BucketConfig>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut per_endpoint = HashMap::new();
        per_endpoint.insert(
            "POST /v1/check".to_string(),
            BucketConfig {
                capacity: 5.0,
                leak_rate_per_sec: 0.083,
            },
        );
        per_endpoint.insert(
            "POST /v1/approve".to_string(),
            BucketConfig {
                capacity: 10.0,
                leak_rate_per_sec: 0.167,
            },
        );
        per_endpoint.insert(
            "POST /v1/rollback".to_string(),
            BucketConfig {
                capacity: 3.0,
                leak_rate_per_sec: 0.05,
            },
        );
        per_endpoint.insert(
            "POST /v1/defer".to_string(),
            BucketConfig {
                capacity: 10.0,
                leak_rate_per_sec: 0.167,
            },
        );

        Self {
            global: BucketConfig {
                capacity: 60.0,
                leak_rate_per_sec: 1.0,
            },
            per_endpoint,
        }
    }
}

impl RateLimitConfig {
    pub async fn load_or_default(config_dir: &Path) -> Self {
        let path = config_dir.join("rate_limit.toml");
        match tokio::fs::read_to_string(path).await {
            Ok(raw) => toml::from_str::<RateLimitConfig>(&raw).unwrap_or_else(|_| Self::default()),
            Err(_) => Self::default(),
        }
    }

    pub fn for_endpoint(&self, endpoint_key: &str) -> BucketConfig {
        self.per_endpoint
            .get(endpoint_key)
            .cloned()
            .unwrap_or_else(|| self.global.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RegistryKey {
    endpoint: String,
    client_ip: IpAddr,
}

#[derive(Clone)]
struct BucketEntry {
    bucket: Arc<LeakyBucket>,
    last_seen: Instant,
}

#[derive(Clone)]
pub struct RateLimitRegistry {
    config: RateLimitConfig,
    entries: Arc<DashMap<RegistryKey, BucketEntry>>,
    idle_ttl: Duration,
}

impl RateLimitRegistry {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: Arc::new(DashMap::new()),
            idle_ttl: Duration::from_secs(600),
        }
    }

    pub fn bucket_for(&self, endpoint: &str, client_ip: IpAddr) -> Arc<LeakyBucket> {
        let key = RegistryKey {
            endpoint: endpoint.to_string(),
            client_ip,
        };

        if let Some(mut entry) = self.entries.get_mut(&key) {
            entry.last_seen = Instant::now();
            return entry.bucket.clone();
        }

        let config = self.config.for_endpoint(endpoint);
        let bucket = Arc::new(LeakyBucket::new(config.capacity, config.leak_rate_per_sec));
        self.entries.insert(
            key,
            BucketEntry {
                bucket: bucket.clone(),
                last_seen: Instant::now(),
            },
        );

        bucket
    }

    pub fn evict_idle_entries(&self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| now.duration_since(entry.last_seen) <= self.idle_ttl);
    }

    pub async fn snapshot(&self) -> Vec<BucketStatus> {
        let mut out = Vec::new();

        for entry in self.entries.iter() {
            let fill = entry.value().bucket.current_fill().await;
            out.push(BucketStatus {
                endpoint: entry.key().endpoint.clone(),
                client_ip: entry.key().client_ip.to_string(),
                current_fill: fill,
            });
        }

        out
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BucketStatus {
    pub endpoint: String,
    pub client_ip: String,
    pub current_fill: f64,
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub endpoint: Option<String>,
}

pub async fn get_rate_limit_status(
    State(registry): State<Arc<RateLimitRegistry>>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Query(query): Query<StatusQuery>,
) -> Response {
    let ip = connect_info
        .map(|info| info.0.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

    if ip != IpAddr::V4(Ipv4Addr::LOCALHOST) && ip != IpAddr::V6(Ipv6Addr::LOCALHOST) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "remote_access_denied"})),
        )
            .into_response();
    }

    registry.evict_idle_entries();
    let mut statuses = registry.snapshot().await;

    if let Some(endpoint) = query.endpoint {
        statuses.retain(|row| row.endpoint == endpoint);
    }

    Json(statuses).into_response()
}
