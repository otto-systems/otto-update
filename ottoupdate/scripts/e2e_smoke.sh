#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${OTTOUPDATE_BASE_URL:-http://127.0.0.1:7430}"
TOKEN="${OTTOUPDATE_BEARER_TOKEN:-}"

AUTH_ARGS=()
if [[ -n "${TOKEN}" ]]; then
  AUTH_ARGS=(-H "Authorization: Bearer ${TOKEN}")
fi

assert_status() {
  local expected="$1"
  shift
  local status
  status=$(curl -sS -o /dev/null -w "%{http_code}" "$@")
  if [[ "${status}" != "${expected}" ]]; then
    echo "expected HTTP ${expected}, got ${status} for: $*" >&2
    exit 1
  fi
}

echo "Running OttoUpdate smoke checks against ${BASE_URL}"

assert_status 200 "${BASE_URL}/health"
assert_status 200 "${AUTH_ARGS[@]}" "${BASE_URL}/v1/state"
assert_status 202 -X POST "${AUTH_ARGS[@]}" "${BASE_URL}/v1/check"
assert_status 200 "${AUTH_ARGS[@]}" "${BASE_URL}/v1/config"
assert_status 200 "${AUTH_ARGS[@]}" "${BASE_URL}/v1/backups"

echo "Smoke checks passed"
