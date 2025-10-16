#!/usr/bin/env bash
set -euo pipefail

# Build a macOS Intel (x86_64) release binary and package it
# together with README, docs, and assets into a tar.gz bundle ready
# for distribution.
# Usage: ./build_macos_intel_release.sh [TAG]
# Example: ./build_macos_intel_release.sh v1.2.3

APP_NAME=stock-cli
TARGET_TRIPLE=x86_64-apple-darwin
DIST_DIR=dist

TAG_NAME=${1:-local}
PKG_NAME="${APP_NAME}-macos-x86_64-${TAG_NAME}"

echo "==> Ensuring Rust target: ${TARGET_TRIPLE}"
if ! rustup target list --installed | grep -qx "${TARGET_TRIPLE}"; then
  rustup target add ${TARGET_TRIPLE}
fi

echo "==> Building release for ${TARGET_TRIPLE}"
cargo build --release --target ${TARGET_TRIPLE}

echo "==> Verifying required files"
for f in README.md; do
  if [[ ! -f "$f" ]]; then
    echo "Missing required file: $f" >&2
    exit 1
  fi
done

for d in docs assets; do
  if [[ ! -d "$d" ]]; then
    echo "Missing required directory: $d" >&2
    exit 1
  fi
done

echo "==> Packaging ${PKG_NAME}"
mkdir -p "$DIST_DIR"
STAGING_DIR="${DIST_DIR}/${PKG_NAME}"
rm -rf "${STAGING_DIR}"
mkdir -p "${STAGING_DIR}"
trap 'rm -rf "${STAGING_DIR}"' EXIT
cp "target/${TARGET_TRIPLE}/release/${APP_NAME}" "${STAGING_DIR}/${APP_NAME}"
cp README.md "${STAGING_DIR}/"
cp -R docs "${STAGING_DIR}/"
cp -R assets "${STAGING_DIR}/"

tar -C "$DIST_DIR" -czf "${DIST_DIR}/${PKG_NAME}.tar.gz" "${PKG_NAME}"
shasum -a 256 "${DIST_DIR}/${PKG_NAME}.tar.gz" >"${DIST_DIR}/${PKG_NAME}.tar.gz.sha256"

rm -rf "${STAGING_DIR}"
trap - EXIT

echo "==> Done. Artifacts:"
ls -lh "${DIST_DIR}" | sed 's/^/  /'
