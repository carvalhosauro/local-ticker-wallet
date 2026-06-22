#!/usr/bin/env bash
# Assemble a flat APT repository directory from .deb files.
set -euo pipefail

OUT_DIR="${1:?usage: build-apt-repo.sh <output-dir>}"
DEB_DIR="${2:-dist/debs}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

mkdir -p "$OUT_DIR/pool/main/l/ltw"

shopt -s nullglob
for deb in "$DEB_DIR"/*.deb; do
  cp "$deb" "$OUT_DIR/pool/main/l/ltw/"
done

# Flat repository layout (works with trusted=yes — no GPG key required).
(
  cd "$OUT_DIR"
  dpkg-scanpackages --multiversion pool/ > Packages
  gzip -kf Packages
)

cp "$ROOT/packaging/install-apt.sh" "$OUT_DIR/install-apt.sh"
chmod 755 "$OUT_DIR/install-apt.sh"

echo "APT repo ready at $OUT_DIR"
