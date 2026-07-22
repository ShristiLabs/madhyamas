#!/bin/bash
# Build the unified Madhyamas binary for release
# Usage: ./build-all.sh <version>

set -e

VERSION="${1:-0.1.0}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/dist"

echo "Building Madhyamas v$VERSION (unified binary)"
echo "=============================================="

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build frontend first (embedded into binary via rust-embed)
echo "Building frontend..."
cd "$ROOT_DIR/web"
npm ci
npm run build

# Build the unified binary (proxy + web UI + MCP + CLI)
echo "Building unified binary..."
cd "$ROOT_DIR"
cargo build --release -p madhyamas
cp target/release/madhyamas "$OUTPUT_DIR/"

echo ""
echo "Build complete! Artifacts in: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR"
