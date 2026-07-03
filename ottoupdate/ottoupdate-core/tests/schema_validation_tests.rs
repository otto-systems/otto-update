use std::path::PathBuf;

use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

fn schema_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../otto-protocol/schemas")
        .join(file_name)
}

fn load_json(path: PathBuf) -> Value {
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

fn compile_draft7(schema: &Value) -> JSONSchema {
    JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
        .expect("schema should compile")
}

#[test]
fn global_release_policy_schema_accepts_valid_document() {
    let schema = load_json(schema_path("global_release_policy.schema.json"));
    let compiled = compile_draft7(&schema);

    let valid_policy = serde_json::json!({
      "policy_id": "stable-global-policy",
      "name": "Stable rollout",
      "description": "Default staged rollout policy",
      "priority": 100,
      "enabled": true,
      "target": {
        "channels": ["stable"],
        "platforms": ["macos", "windows"],
        "device_tags_any": ["managed"],
        "versions_below": "2.0.0"
      },
      "rules": [
        {
          "kind": "allow",
          "reason": "standard rollout"
        }
      ],
      "maintenance_window": null,
      "max_deferrals": {
        "count": 3,
        "window_days": 14
      }
    });

    let result = compiled.validate(&valid_policy);
    assert!(result.is_ok(), "valid policy should pass: {result:?}");
}

#[test]
fn global_release_policy_schema_rejects_missing_required_fields() {
    let schema = load_json(schema_path("global_release_policy.schema.json"));
    let compiled = compile_draft7(&schema);

    let invalid_policy = serde_json::json!({
      "name": "Missing required keys",
      "enabled": true
    });

    let result = compiled.validate(&invalid_policy);
    assert!(result.is_err(), "invalid policy should fail validation");
}

#[test]
fn device_state_schema_accepts_valid_snapshot() {
    let schema = load_json(schema_path("device_state.schema.json"));
    let compiled = compile_draft7(&schema);

    let valid_state = serde_json::json!({
      "device_id": "dev-001",
      "hostname": "otto-laptop",
      "recorded_at": "2026-01-01T10:00:00Z",
      "platform": {
        "os": "macos",
        "os_version": "15.4",
        "kernel_version": "24.4.0",
        "arch": "arm64"
      },
      "hardware": {
        "cpu_model": "Apple M2",
        "cpu_cores": 8,
        "ram_bytes": 17179869184,
        "disk_total_bytes": 512000000000,
        "disk_free_bytes": 256000000000,
        "battery_percent": 92,
        "on_ac_power": true
      },
      "network": {
        "connected": true,
        "connection_type": "wifi",
        "metered": false
      },
      "installed_product": {
        "name": "otto",
        "version": "1.2.0",
        "channel": "stable",
        "install_path": "/Applications/Otto.app"
      },
      "update_history": [
        {
          "event_id": "evt-001",
          "version": "1.1.0",
          "outcome": "applied",
          "recorded_at": "2025-12-15T08:00:00Z",
          "reason": null
        }
      ],
      "tags": ["managed"],
      "deferred_count": 0,
      "last_deferred_at": null,
      "managed": true,
      "management_group": "engineering"
    });

    let result = compiled.validate(&valid_state);
    assert!(result.is_ok(), "valid device state should pass: {result:?}");
}

#[test]
fn device_state_schema_rejects_wrong_types() {
    let schema = load_json(schema_path("device_state.schema.json"));
    let compiled = compile_draft7(&schema);

    let invalid_state = serde_json::json!({
      "device_id": "dev-001",
      "hostname": "otto-laptop",
      "recorded_at": "2026-01-01T10:00:00Z",
      "platform": {
        "os": "macos",
        "os_version": "15.4",
        "kernel_version": "24.4.0",
        "arch": "arm64"
      },
      "hardware": {
        "cpu_model": "Apple M2",
        "cpu_cores": "eight",
        "ram_bytes": 17179869184,
        "disk_total_bytes": 512000000000,
        "disk_free_bytes": 256000000000,
        "battery_percent": 92,
        "on_ac_power": true
      },
      "network": {
        "connected": true,
        "connection_type": "wifi",
        "metered": false
      },
      "installed_product": {
        "name": "otto",
        "version": "1.2.0",
        "channel": "stable",
        "install_path": "/Applications/Otto.app"
      },
      "update_history": [],
      "tags": [],
      "deferred_count": 0,
      "last_deferred_at": null,
      "managed": true,
      "management_group": null
    });

    let result = compiled.validate(&invalid_state);
    assert!(result.is_err(), "invalid device state should fail validation");
}
