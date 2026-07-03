#!/usr/bin/env bash
set -euo pipefail

PLIST_LABEL="com.otto.ottoupdate"
PLIST_SRC="deploy/macos/com.otto.ottoupdate.plist"
PLIST_DST="/Library/LaunchDaemons/${PLIST_LABEL}.plist"
BIN_SRC="./target/release/ottoupdate-server"
BIN_DST="/usr/local/bin/ottoupdate-server"
CFG_DIR="/usr/local/etc/ottoupdate"

sudo install -d -m 755 /usr/local/bin
sudo install -d -m 755 /usr/local/var/ottoupdate
sudo install -d -m 755 /usr/local/var/log
sudo install -d -m 755 "${CFG_DIR}"

sudo install -m 755 "${BIN_SRC}" "${BIN_DST}"
sudo install -m 644 "${PLIST_SRC}" "${PLIST_DST}"

if [[ ! -f "${CFG_DIR}/config.toml" ]]; then
  cat <<'EOF' | sudo tee "${CFG_DIR}/config.toml" >/dev/null
[server]
bind = "127.0.0.1:7430"
EOF
fi

sudo launchctl unload "${PLIST_DST}" >/dev/null 2>&1 || true
sudo launchctl load -w "${PLIST_DST}"

echo "${PLIST_LABEL} installed and loaded"
