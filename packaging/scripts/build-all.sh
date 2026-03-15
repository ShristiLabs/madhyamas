#!/bin/bash
# Build all Madhyamas packages for release
# Usage: ./build-all.sh <version>

set -e

VERSION="${1:-0.1.0}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/dist"

echo "Building Madhyamas v$VERSION"
echo "================================"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build frontend first
echo "Building frontend..."
cd "$ROOT_DIR/web"
npm ci
npm run build

# Build all Rust binaries
echo "Building Rust binaries..."
cd "$ROOT_DIR"

# Main binary (madhyamas)
cargo build --release -p madhyamas-cli
cp target/release/madhyamas "$OUTPUT_DIR/"

# CLI binary
cargo build --release -p madhyamas-cli
cp target/release/madhyamas-cli "$OUTPUT_DIR/"

# MCP binary
cargo build --release -p madhyamas-mcp
cp target/release/madhyamas-mcp "$OUTPUT_DIR/"

# Copy web assets
cp -r web/dist "$OUTPUT_DIR/web"

echo ""
echo "Build complete! Artifacts in: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR"
