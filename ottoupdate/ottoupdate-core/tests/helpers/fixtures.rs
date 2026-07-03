use chrono::{Duration, Utc};
use uuid::Uuid;

use ottoupdate_core::decision::matrix::{DecisionInput, PolicyDecision};
use ottoupdate_core::device::collector::{
    DeviceState, HardwareState, InstalledProductState, NetworkState, PlatformState,
    UpdateHistoryItem,
};
use ottoupdate_core::traits::{Artifact, ReleaseManifest};

pub fn artifact_fixture(id: &str, url: &str) -> Artifact {
    Artifact {
        id: id.to_string(),
        url: url.to_string(),
        sha256_hex: None,
        signature_b64: None,
        public_key_hex: None,
    }
}

pub fn release_manifest_fixture(version: &str) -> ReleaseManifest {
    ReleaseManifest {
        version: version.to_string(),
        artifacts: vec![artifact_fixture("artifact-1", "https://example.invalid/otto.bin")],
    }
}

pub fn device_state_fixture() -> DeviceState {
    DeviceState {
        device_id: Uuid::new_v4().to_string(),
        hostname: "otto-device".to_string(),
        recorded_at: Utc::now(),
        platform: PlatformState {
            os: "macos".to_string(),
            os_version: "15.5".to_string(),
            kernel_version: "24.5.0".to_string(),
            arch: "arm64".to_string(),
        },
        hardware: HardwareState {
            cpu_model: "Apple M2".to_string(),
            cpu_cores: 8,
            ram_bytes: 16 * 1024 * 1024 * 1024,
            disk_total_bytes: 512 * 1024 * 1024 * 1024,
            disk_free_bytes: 256 * 1024 * 1024 * 1024,
            battery_percent: Some(100),
            on_ac_power: true,
        },
        network: NetworkState {
            connected: true,
            connection_type: "wifi".to_string(),
            metered: false,
        },
        installed_product: InstalledProductState {
            name: "otto".to_string(),
            version: "1.0.0".to_string(),
            channel: "stable".to_string(),
            install_path: "/Applications/Otto.app".to_string(),
        },
        update_history: vec![UpdateHistoryItem {
            event_id: Uuid::new_v4().to_string(),
            version: "0.9.0".to_string(),
            outcome: "applied".to_string(),
            recorded_at: Utc::now() - Duration::days(7),
            reason: None,
        }],
        tags: vec!["managed".to_string(), "pilot".to_string()],
        deferred_count: 0,
        last_deferred_at: None,
        managed: true,
        management_group: Some("engineering".to_string()),
    }
}

pub fn decision_input_fixture() -> DecisionInput {
    DecisionInput {
        policy_decision: PolicyDecision::Approve,
        device_safe: true,
        battery_percent: Some(100),
        disk_free_mb: Some(10_000),
        update_size_mb: Some(10),
        in_maintenance_window: true,
        deferred_count: 0,
        max_deferred_days: 7,
        breaking_changes: false,
        requires_reboot: false,
        network_metered: false,
        staged_percentage: None,
        device_in_canary: true,
        manifest_revoked: false,
        approval_granted: true,
        now: Utc::now(),
        next_maintenance_window_start: Some(Utc::now() + Duration::hours(2)),
    }
}
