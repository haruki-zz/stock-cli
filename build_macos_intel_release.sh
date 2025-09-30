#!/usr/bin/env bash
set -euo pipefail

# Build a macOS Intel (x86_64) release binary and package it
# with stock_code.csv and Cargo.toml into a tar.gz.
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
for f in stock_code.csv Cargo.toml; do
  if [[ ! -f "$f" ]]; then
    echo "Missing required file: $f" >&2
    exit 1
  fi
done

echo "==> Packaging ${PKG_NAME}"
mkdir -p "$DIST_DIR"
cp "target/${TARGET_TRIPLE}/release/${APP_NAME}" "${APP_NAME}"
tar -czf "${DIST_DIR}/${PKG_NAME}.tar.gz" ${APP_NAME} stock_code.csv Cargo.toml README.md
shasum -a 256 "${DIST_DIR}/${PKG_NAME}.tar.gz" > "${DIST_DIR}/${PKG_NAME}.tar.gz.sha256"
rm -f "${APP_NAME}"

echo "==> Done. Artifacts:"
ls -lh "${DIST_DIR}" | sed 's/^/  /'
