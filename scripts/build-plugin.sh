#!/usr/bin/env bash
# Build a single Madhyamas plugin.
#
# Reads the version from the plugin's Cargo.toml, compiles to
# wasm32-unknown-unknown, packages as a zip (manifest + plugin.wasm),
# and prints the SHA-256 checksum.
#
# Usage:
#   ./scripts/build-plugin.sh <plugin-name> [--update-registry] [--registry-url <url>]
#
# Example:
#   ./scripts/build-plugin.sh cors-helper
#   ./scripts/build-plugin.sh cors-helper --update-registry
#
# Environment:
#   TARGET     Rust target triple (default: wasm32-unknown-unknown)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

TARGET="${TARGET:-wasm32-unknown-unknown}"
PLUGIN_NAME=""
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
      if [[ -z "$PLUGIN_NAME" ]]; then
        PLUGIN_NAME="$1"
      else
        echo "Unknown argument: $1" >&2
        exit 1
      fi
      shift
      ;;
  esac
done

if [[ -z "$PLUGIN_NAME" ]]; then
  echo "Usage: $0 <plugin-name> [--update-registry] [--registry-url <url>]" >&2
  echo "Available plugins:" >&2
  ls -d plugins/*/ 2>/dev/null | sed 's|plugins/||;s|/||' | sed 's/^/  /' >&2
  exit 1
fi

PLUGIN_DIR="plugins/${PLUGIN_NAME}"

if [[ ! -d "$PLUGIN_DIR" ]]; then
  echo "ERROR: Plugin directory not found: ${PLUGIN_DIR}" >&2
  exit 1
fi

if [[ ! -f "${PLUGIN_DIR}/Cargo.toml" ]]; then
  echo "ERROR: Plugin Cargo.toml not found: ${PLUGIN_DIR}/Cargo.toml" >&2
  exit 1
fi

if [[ ! -f "${PLUGIN_DIR}/madhyamas-plugin.toml" ]]; then
  echo "ERROR: Plugin manifest not found: ${PLUGIN_DIR}/madhyamas-plugin.toml" >&2
  exit 1
fi

# Ensure target is installed
if ! rustup target list --installed 2>/dev/null | grep -q "${TARGET}"; then
  echo "Installing ${TARGET}..."
  rustup target add "${TARGET}"
fi

# Extract version from Cargo.toml
version=$(grep '^version' "${PLUGIN_DIR}/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/')
echo "Building ${PLUGIN_NAME} v${version} for ${TARGET}..."

# Build the plugin (not a workspace member, so build from its directory)
(cd "$PLUGIN_DIR" && cargo build --target "${TARGET}" --release)

# Find the WASM output (cdylib naming: underscores instead of hyphens)
wasm_file="${REPO_ROOT}/target/${TARGET}/release/madhyamas_plugin_$(echo "${PLUGIN_NAME}" | tr '-' '_').wasm"
if [[ ! -f "$wasm_file" ]]; then
  # Fallback: check in plugin's own target dir
  wasm_file="${PLUGIN_DIR}/target/${TARGET}/release/madhyamas_plugin_$(echo "${PLUGIN_NAME}" | tr '-' '_').wasm"
fi
if [[ ! -f "$wasm_file" ]]; then
  echo "ERROR: WASM file not found" >&2
  echo "  Looked in: ${REPO_ROOT}/target/${TARGET}/release/" >&2
  echo "  And: ${PLUGIN_DIR}/target/${TARGET}/release/" >&2
  exit 1
fi

# Package as zip
zip_file="plugins/${PLUGIN_NAME}-${version}.zip"
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

cp "${PLUGIN_DIR}/madhyamas-plugin.toml" "${tmp_dir}/"
cp "$wasm_file" "${tmp_dir}/plugin.wasm"

rm -f "$zip_file"
abs_zip="${REPO_ROOT}/${zip_file}"
(cd "$tmp_dir" && zip -j "$abs_zip" madhyamas-plugin.toml plugin.wasm) > /dev/null 2>&1

# Compute checksum
if [[ "$(uname)" == "Darwin" ]]; then
  checksum=$(shasum -a 256 "$zip_file" | awk '{print $1}')
else
  checksum=$(sha256sum "$zip_file" | awk '{print $1}')
fi

echo "  Package: ${zip_file}"
echo "  SHA-256: ${checksum}"
echo "  Version: ${version}"

# Update registry.json if requested
if [[ "$UPDATE_REGISTRY" == "true" ]]; then
  if [[ -z "$REGISTRY_URL" ]]; then
    REGISTRY_URL="https://github.com/shristilabs/madhyamas/releases/download/${PLUGIN_NAME}-latest"
  fi

  python3 - "$PLUGIN_NAME" "$version" "$checksum" "$REGISTRY_URL" "plugins/registry.json" <<'PYEOF'
import json, sys

plugin_name = sys.argv[1]
version = sys.argv[2]
checksum = sys.argv[3]
url_base = sys.argv[4]
registry_path = sys.argv[5]

plugin_id = f"madhyamas.{plugin_name}"
download_url = f"{url_base}/{plugin_name}-{version}.zip"

with open(registry_path) as f:
    data = json.load(f)

found = False
for entry in data["plugins"]:
    if entry["manifest"]["id"] == plugin_id:
        entry["manifest"]["version"] = version
        entry["download_url"] = download_url
        entry["checksum"] = f"sha256:{checksum}"
        entry["updated_at"] = __import__("datetime").datetime.utcnow().isoformat() + "Z"
        found = True
        break

if not found:
    print(f"WARNING: Plugin {plugin_id} not found in registry.json", file=sys.stderr)

with open(registry_path, "w") as f:
    json.dump(data, f, indent=2)
    f.write("\n")

print(f"Updated {registry_path}: {plugin_id} v{version}")
PYEOF
fi

echo "Done."
