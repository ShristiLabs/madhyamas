#!/bin/bash

# Build Madhyamas MCP binary for local use with Windsurf
# This script builds native binaries for your platform

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${PROJECT_DIR}/bin"

echo "Building Madhyamas binaries..."

cd "$PROJECT_DIR"

# Build the binaries
cargo build --release -p madhyamas-cli -p madhyamas-mcp

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Copy binaries
cp target/release/madhyamas-mcp "$OUTPUT_DIR/madhyamas-mcp"
cp target/release/madhyamas "$OUTPUT_DIR/madhyamas"

# Make executable
chmod +x "$OUTPUT_DIR/madhyamas-mcp"
chmod +x "$OUTPUT_DIR/madhyamas"

echo ""
echo "✓ Binaries extracted to: $OUTPUT_DIR"
echo "  - madhyamas-mcp (MCP server)"
echo "  - madhyamas (CLI)"
echo ""
echo "To configure Windsurf, add the following to your mcp_config.json:"
echo ""
cat << EOF
{
  "mcpServers": {
    "madhyamas": {
      "command": "${OUTPUT_DIR}/madhyamas-mcp",
      "env": {
        "MADHYAMAS_API_URL": "http://localhost:3001"
      }
    }
  }
}
EOF
echo ""
