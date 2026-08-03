#!/usr/bin/env bash
# Build all Madhyamas plugins.
#
# Discovers plugins under plugins/ (directories with Cargo.toml),
# builds each to wasm32-unknown-unknown, and packages as zips.
#
# Usage:
#   ./scripts/build-all-plugins.sh [--update-registry] [--registry-url <url>]
#
# Environment:
#   TARGET     Rust target triple (default: wasm32-unknown-unknown)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

TARGET="${TARGET:-wasm32-unknown-unknown}"
UPDATE_REGISTRY=false
REGISTRY_URL=""

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --update-registry)
      UPDATE_REGISTRY=true
      shift
      ;;
    --registry-url)
      REGISTRY_URL="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

# Discover plugins (directories with Cargo.toml under plugins/)
plugins=()
for dir in plugins/*/; do
  [[ -f "${dir}Cargo.toml" ]] && plugins+=("$(basename "$dir")")
done

if [[ ${#plugins[@]} -eq 0 ]]; then
  echo "No plugins found in plugins/" >&2
  exit 1
fi

echo "Found ${#plugins[@]} plugin(s): ${plugins[*]}"
echo ""

# Build each plugin
for plugin in "${plugins[@]}"; do
  args=("$plugin")
  if [[ "$UPDATE_REGISTRY" == "true" ]]; then
    args+=("--update-registry")
    if [[ -n "$REGISTRY_URL" ]]; then
      args+=("--registry-url" "$REGISTRY_URL")
    fi
  fi
  ./scripts/build-plugin.sh "${args[@]}"
  echo ""
done

echo "All plugins built."
