#!/bin/bash

# Build ProxyForge MCP binary for local use with Windsurf
# This script builds native binaries for your platform

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${PROJECT_DIR}/bin"

echo "Building ProxyForge binaries..."

cd "$PROJECT_DIR"

# Build the binaries
cargo build --release -p proxyforge-cli -p proxyforge-mcp

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Copy binaries
cp target/release/proxyforge-mcp "$OUTPUT_DIR/proxyforge-mcp"
cp target/release/proxyforge "$OUTPUT_DIR/proxyforge"

# Make executable
chmod +x "$OUTPUT_DIR/proxyforge-mcp"
chmod +x "$OUTPUT_DIR/proxyforge"

echo ""
echo "✓ Binaries extracted to: $OUTPUT_DIR"
echo "  - proxyforge-mcp (MCP server)"
echo "  - proxyforge (CLI)"
echo ""
echo "To configure Windsurf, add the following to your mcp_config.json:"
echo ""
cat << EOF
{
  "mcpServers": {
    "proxyforge": {
      "command": "${OUTPUT_DIR}/proxyforge-mcp",
      "env": {
        "PROXYFORGE_API_URL": "http://localhost:3001"
      }
    }
  }
}
EOF
echo ""
