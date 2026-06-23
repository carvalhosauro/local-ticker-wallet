#!/usr/bin/env bash
# One-time setup: add the ltw APT repository and install the package.
# Usage: curl -fsSL https://carvalhosauro.github.io/local-ticker-wallet/install-apt.sh | sudo sh
set -euo pipefail

REPO_URL="${LTW_APT_REPO:-https://carvalhosauro.github.io/local-ticker-wallet}"
LIST_FILE="/etc/apt/sources.list.d/ltw.list"

if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
  echo "Re-run as root: curl -fsSL ${REPO_URL}/install-apt.sh | sudo sh" >&2
  exit 1
fi

cat >"$LIST_FILE" <<EOF
# local-ticker-wallet — https://github.com/carvalhosauro/local-ticker-wallet
deb [trusted=yes arch=amd64,arm64] ${REPO_URL} ./
EOF

apt-get update -qq
apt-get install -y ltw

echo "Installed $(command -v ltw)"
