#!/usr/bin/env bash
set -euo pipefail

# Local helper to build a macOS Intel (x86_64) release package.
# Requires Rust toolchain installed. Run from repo root.

APP_NAME=stock-cli
TARGET_TRIPLE=x86_64-apple-darwin
DIST_DIR=dist

echo "==> Ensuring Rust target: ${TARGET_TRIPLE}"
rustup target add ${TARGET_TRIPLE} || true

echo "==> Building release binary for ${TARGET_TRIPLE}"
cargo build --release --target ${TARGET_TRIPLE}

TAG_NAME=${1:-local}
PKG_NAME="${APP_NAME}-macos-x86_64-${TAG_NAME}"

echo "==> Packaging ${PKG_NAME}"
mkdir -p "${DIST_DIR}"
cp "target/${TARGET_TRIPLE}/release/${APP_NAME}" "${APP_NAME}"
tar -czf "${DIST_DIR}/${PKG_NAME}.tar.gz" ${APP_NAME} config.json stock_code.csv README.md
shasum -a 256 "${DIST_DIR}/${PKG_NAME}.tar.gz" > "${DIST_DIR}/${PKG_NAME}.tar.gz.sha256"

echo "==> Done"
echo "Artifacts in ${DIST_DIR}/:"
ls -lh "${DIST_DIR}" | sed 's/^/  /'

