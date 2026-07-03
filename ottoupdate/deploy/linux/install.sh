#!/usr/bin/env bash
set -euo pipefail

SERVICE_NAME="ottoupdate"
BIN_SRC="./target/release/ottoupdate-server"
BIN_DST="/usr/local/bin/ottoupdate-server"
CFG_DIR="/etc/ottoupdate"
CFG_FILE="${CFG_DIR}/config.toml"
UNIT_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

if ! id -u ottoupdate >/dev/null 2>&1; then
  sudo useradd --system --home /var/lib/ottoupdate --shell /usr/sbin/nologin ottoupdate
fi

sudo install -d -m 755 /var/lib/ottoupdate
sudo install -d -m 755 "${CFG_DIR}"
sudo install -m 755 "${BIN_SRC}" "${BIN_DST}"

if [[ ! -f "${CFG_FILE}" ]]; then
  cat <<'EOF' | sudo tee "${CFG_FILE}" >/dev/null
[server]
bind = "127.0.0.1:7430"
EOF
fi

sudo install -m 644 deploy/linux/ottoupdate.service "${UNIT_FILE}"
sudo systemctl daemon-reload
sudo systemctl enable --now "${SERVICE_NAME}"

echo "${SERVICE_NAME} installed and running"
