#!/usr/bin/env bash
set -euo pipefail

# Stock CLI macOS quarantine removal helper.
# Run this script from the unpacked release directory (or pass the binary path)
# to clear Gatekeeper quarantine flags so the app can be launched normally.

BIN_PATH="${1:-./stock-cli}"

if [[ ! -e "$BIN_PATH" ]]; then
  echo "Error: stock-cli binary not found at '${BIN_PATH}'." >&2
  echo "Place deploy.sh in the same directory as the binary or pass an explicit path." >&2
  exit 1
fi

echo "Clearing macOS quarantine attribute on '${BIN_PATH}'..."
set +e
xattr -d -r com.apple.quarantine "${BIN_PATH}" >/dev/null 2>&1
STATUS=$?
set -e

if [[ ${STATUS} -eq 0 ]]; then
  echo "Quarantine attribute removed. You can now launch the app directly."
else
  echo "No quarantine attribute found or it was already cleared."
fi
