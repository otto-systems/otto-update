use std::collections::HashSet;
use std::path::PathBuf;

use serde_yaml::Value;

fn openapi_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../api/ottoupdate.openapi.yaml")
}

fn load_openapi() -> Value {
    let path = openapi_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_yaml::from_str::<Value>(&raw)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

#[test]
fn openapi_contains_required_paths() {
    let spec = load_openapi();
    let paths = spec
        .get("paths")
        .and_then(Value::as_mapping)
        .expect("paths mapping should exist");

    let required = [
        "/health",
        "/v1/state",
        "/v1/check",
        "/v1/manifest",
        "/v1/policy",
        "/v1/approve",
        "/v1/defer",
        "/v1/progress",
        "/v1/history",
        "/v1/config",
        "/v1/rollback",
        "/v1/backups",
    ];

    for route in required {
        assert!(
            paths.contains_key(Value::from(route)),
            "missing OpenAPI path: {route}"
        );
    }
}

#[test]
fn mutating_endpoints_are_explicitly_tagged() {
    let spec = load_openapi();
    let paths = spec
        .get("paths")
        .and_then(Value::as_mapping)
        .expect("paths mapping should exist");

    let expected: HashSet<(&str, &str)> = HashSet::from([
        ("/v1/check", "post"),
        ("/v1/approve", "post"),
        ("/v1/defer", "post"),
        ("/v1/config", "put"),
        ("/v1/rollback", "post"),
    ]);

    for (route, method) in expected {
        let path_item = paths
            .get(Value::from(route))
            .and_then(Value::as_mapping)
            .unwrap_or_else(|| panic!("path should exist: {route}"));
        let operation = path_item
            .get(Value::from(method))
            .and_then(Value::as_mapping)
            .unwrap_or_else(|| panic!("operation should exist: {method} {route}"));

        let flag = operation
            .get(Value::from("x-otto-mutating"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        assert!(flag, "expected x-otto-mutating=true for {method} {route}");
    }
}
