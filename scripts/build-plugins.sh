#!/usr/bin/env bash
# Build, package, and publish Madhyamas plugins.
#
# This script compiles the example plugins from madhyamas-plugin-sdk to
# wasm32-unknown-unknown, packages each as a zip (manifest + plugin.wasm),
# computes SHA-256 checksums, and optionally updates registry.json with
# the correct download URLs and checksums.
#
# Usage:
#   ./scripts/build-plugins.sh                # Build + package only
#   ./scripts/build-plugins.sh --update-registry   # Also update registry.json
#   ./scripts/build-plugins.sh --registry-url https://github.com/shristilabs/madhyamas/releases/download/plugins-latest
#
# Environment:
#   PLUGINS_DIR   Output directory for zips (default: plugins/)
#   TARGET        Rust target triple (default: wasm32-unknown-unknown)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PLUGINS_DIR="${PLUGINS_DIR:-plugins}"
TARGET="${TARGET:-wasm32-unknown-unknown}"
REGISTRY_URL=""
UPDATE_REGISTRY=false

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

# Plugin definitions: "name example manifest_dir"
# name            — short name used in zip filename (e.g. cors-helper)
# example         — cargo example target name (e.g. cors_helper)
# manifest_dir    — directory under plugins/ with madhyamas-plugin.toml
PLUGINS="cors-helper cors_helper cors-helper
request-logger request_logger request-logger
domain-blocker domain_blocker domain-blocker"

echo "=== Building plugins for ${TARGET} ==="

# Ensure target is installed
if ! rustup target list --installed 2>/dev/null | grep -q "${TARGET}"; then
  echo "Installing ${TARGET}..."
  rustup target add "${TARGET}"
fi

# Build all examples at once (faster than individual builds)
echo "Compiling plugin examples..."
cargo build --target "${TARGET}" --release --examples -p madhyamas-plugin-sdk

EXAMPLES_DIR="target/${TARGET}/release/examples"

# Package each plugin
# Write checksums to a temp file for the registry update step
CHECKSUM_FILE=$(mktemp)
trap 'rm -f "$CHECKSUM_FILE"' EXIT

echo "=== Packaging plugins ==="

while IFS=' ' read -r name example manifest_dir || [[ -n "$name" ]]; do
  [[ -z "$name" ]] && continue

  # Extract version from manifest
  version=$(grep '^version' "${PLUGINS_DIR}/${manifest_dir}/madhyamas-plugin.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/')

  wasm_file="${EXAMPLES_DIR}/${example}.wasm"
  if [[ ! -f "$wasm_file" ]]; then
    echo "ERROR: WASM file not found: ${wasm_file}" >&2
    exit 1
  fi

  zip_file="${PLUGINS_DIR}/${name}-${version}.zip"

  # Create zip in a temp dir to ensure clean structure
  tmp_dir=$(mktemp -d)
  cp "${PLUGINS_DIR}/${manifest_dir}/madhyamas-plugin.toml" "${tmp_dir}/"
  cp "$wasm_file" "${tmp_dir}/plugin.wasm"

  rm -f "$zip_file"
  abs_zip="${REPO_ROOT}/${zip_file}"
  (cd "$tmp_dir" && zip -j "$abs_zip" madhyamas-plugin.toml plugin.wasm) > /dev/null 2>&1

  rm -rf "$tmp_dir"

  # Compute checksum
  if [[ "$(uname)" == "Darwin" ]]; then
    checksum=$(shasum -a 256 "$zip_file" | awk '{print $1}')
  else
    checksum=$(sha256sum "$zip_file" | awk '{print $1}')
  fi

  echo "${name} ${checksum}" >> "$CHECKSUM_FILE"
  echo "  ${name} v${version}: ${zip_file} (sha256: ${checksum:0:16}...)"
done <<< "$PLUGINS"

echo ""
echo "=== All plugins packaged ==="

# Update registry.json if requested
if [[ "$UPDATE_REGISTRY" == "true" ]]; then
  echo ""
  echo "=== Updating registry.json ==="

  if [[ -z "$REGISTRY_URL" ]]; then
    REGISTRY_URL="https://github.com/shristilabs/madhyamas/releases/download/plugins-latest"
  fi

  # Use Python to update registry.json with correct URLs + checksums
  CHECKSUM_FILE="$CHECKSUM_FILE" python3 - "$REGISTRY_URL" "${PLUGINS_DIR}/registry.json" "$CHECKSUM_FILE" <<'PYEOF'
import json, sys

registry_url = sys.argv[1]
registry_path = sys.argv[2]
checksum_file = sys.argv[3]

# Read checksums
checksums = {}
with open(checksum_file) as f:
    for line in f:
        parts = line.strip().split()
        if len(parts) == 2:
            checksums[parts[0]] = parts[1]

with open(registry_path) as f:
    data = json.load(f)

for plugin in data["plugins"]:
    pid = plugin["manifest"]["id"]
    short_name = pid.split(".", 1)[-1] if "." in pid else pid
    version = plugin["manifest"]["version"]

    if short_name in checksums:
        plugin["download_url"] = f"{registry_url}/{short_name}-{version}.zip"
        plugin["checksum"] = f"sha256:{checksums[short_name]}"

with open(registry_path, "w") as f:
    json.dump(data, f, indent=2)
    f.write("\n")

print(f"Updated {registry_path} with {len(checksums)} plugin entries")
PYEOF

  echo "Registry updated with download URLs pointing to: ${REGISTRY_URL}"
fi

echo ""
echo "Done."
