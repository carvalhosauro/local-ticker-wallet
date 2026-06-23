#!/usr/bin/env bash
# Build .deb packages from cargo-dist Linux release tarballs.
set -euo pipefail

ARTIFACTS_DIR="${1:?usage: build-debs.sh <artifacts-dir>}"
OUT_DIR="${2:-dist/debs}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$(mkdir -p "$OUT_DIR" && cd "$OUT_DIR" && pwd)"

version="${LTW_VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)}"
if [[ -z "$version" ]]; then
  echo "Could not determine package version" >&2
  exit 1
fi

build_deb() {
  local tarball="$1"
  local arch="$2"
  local version="$3"
  local work
  work="$(mktemp -d)"

  tar -xJf "$tarball" -C "$work"
  chmod 755 "$work/ltw"

  (
    cd "$work"
    sed -e "s/\${ARCH}/$arch/g" -e "s/\${VERSION}/$version/g" \
      "$ROOT/packaging/nfpm.yaml" > nfpm.yaml
    nfpm pkg --packager deb --config nfpm.yaml --target "$OUT_DIR"
  )

  rm -rf "$work"
}

shopt -s nullglob
for pattern in ltw-x86_64-unknown-linux-gnu.tar.xz ltw-aarch64-unknown-linux-gnu.tar.xz; do
  tarball="$ARTIFACTS_DIR/$pattern"
  [[ -f "$tarball" ]] || continue
  case "$pattern" in
    *x86_64*) arch=amd64 ;;
    *aarch64*) arch=arm64 ;;
    *) continue ;;
  esac
  build_deb "$tarball" "$arch" "$version"
done

if ! compgen -G "$OUT_DIR/*.deb" > /dev/null; then
  echo "No .deb packages built — expected Linux tarballs in $ARTIFACTS_DIR" >&2
  exit 1
fi

echo "Built packages:"
ls -la "$OUT_DIR"/*.deb
