#!/bin/bash

# Stock CLI Deployment Script
# This script helps deploy the stock-cli binary with its dependencies

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_NAME="stock-cli"
TARGET_DIR="${1:-$HOME/bin}"

echo "🚀 Deploying Stock CLI..."

# Create target directory if it doesn't exist
mkdir -p "$TARGET_DIR"

# Copy binary
echo "📦 Copying binary to $TARGET_DIR/$BINARY_NAME"
cp "$SCRIPT_DIR/target/release/$BINARY_NAME" "$TARGET_DIR/"

# Copy config file to same directory as binary
echo "⚙️  Copying config.json"
cp "$SCRIPT_DIR/config.json" "$TARGET_DIR/"

# Copy stock codes file (optional)
if [ -f "$SCRIPT_DIR/stock_code.csv" ]; then
    echo "📋 Copying stock_code.csv"
    cp "$SCRIPT_DIR/stock_code.csv" "$TARGET_DIR/"
fi

# Make binary executable
chmod +x "$TARGET_DIR/$BINARY_NAME"

echo "✅ Deployment complete!"
echo ""
echo "Usage:"
echo "  $TARGET_DIR/$BINARY_NAME interactive"
echo "  $TARGET_DIR/$BINARY_NAME --help"
echo ""
echo "Files deployed:"
echo "  - $TARGET_DIR/$BINARY_NAME"
echo "  - $TARGET_DIR/config.json"
if [ -f "$TARGET_DIR/stock_code.csv" ]; then
    echo "  - $TARGET_DIR/stock_code.csv"
fi
echo ""
echo "💡 Tip: Add $TARGET_DIR to your PATH to run 'stock-cli' from anywhere"