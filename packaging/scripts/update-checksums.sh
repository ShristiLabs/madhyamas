#!/bin/bash
# Update SHA256 checksums in packaging files after release
# Usage: ./update-checksums.sh <version>

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

VERSION="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGING_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
GITHUB_REPO="madhyamas/madhyamas"

echo "Fetching checksums for v$VERSION (unified binary)..."

# Function to get checksum from GitHub release
get_checksum() {
    local filename="$1"
    local url="https://github.com/$GITHUB_REPO/releases/download/v$VERSION/$filename"
    curl -sL "$url" | sha256sum | cut -d' ' -f1
}

# Define all artifacts (single unified binary)
declare -A CHECKSUMS

# macOS
CHECKSUMS["madhyamas-x86_64-apple-darwin"]=$(get_checksum "madhyamas-v$VERSION-x86_64-apple-darwin.tar.gz")
CHECKSUMS["madhyamas-aarch64-apple-darwin"]=$(get_checksum "madhyamas-v$VERSION-aarch64-apple-darwin.tar.gz")

# Linux
CHECKSUMS["madhyamas-x86_64-unknown-linux-gnu"]=$(get_checksum "madhyamas-v$VERSION-x86_64-unknown-linux-gnu.tar.gz")
CHECKSUMS["madhyamas-aarch64-unknown-linux-gnu"]=$(get_checksum "madhyamas-v$VERSION-aarch64-unknown-linux-gnu.tar.gz")
CHECKSUMS["madhyamas-armv7-unknown-linux-gnueabihf"]=$(get_checksum "madhyamas-v$VERSION-armv7-unknown-linux-gnueabihf.tar.gz")
CHECKSUMS["madhyamas-arm-unknown-linux-gnueabihf"]=$(get_checksum "madhyamas-v$VERSION-arm-unknown-linux-gnueabihf.tar.gz")
CHECKSUMS["madhyamas-riscv64gc-unknown-linux-gnu"]=$(get_checksum "madhyamas-v$VERSION-riscv64gc-unknown-linux-gnu.tar.gz")

# Windows
CHECKSUMS["madhyamas-x86_64-pc-windows-msvc"]=$(get_checksum "madhyamas-v$VERSION-x86_64-pc-windows-msvc.zip")

echo "Checksums fetched. Update packaging files manually with these values:"
echo ""
for key in "${!CHECKSUMS[@]}"; do
    echo "$key: ${CHECKSUMS[$key]}"
done
