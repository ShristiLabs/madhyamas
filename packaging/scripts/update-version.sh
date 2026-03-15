#!/bin/bash
# Update version across all packaging files
# Usage: ./update-version.sh <new-version>

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

VERSION="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGING_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Updating version to $VERSION in all packaging files..."

# Homebrew formulas
for file in "$PACKAGING_DIR"/homebrew/*.rb; do
    sed -i.bak "s/version \"[^\"]*\"/version \"$VERSION\"/" "$file"
    rm -f "$file.bak"
done

# Chocolatey nuspec files
find "$PACKAGING_DIR/windows/chocolatey" -name "*.nuspec" -exec sed -i.bak "s/<version>[^<]*<\/version>/<version>$VERSION<\/version>/" {} \;
find "$PACKAGING_DIR/windows/chocolatey" -name "*.bak" -delete

# Snap files
for file in "$PACKAGING_DIR"/linux/snap/*.yaml; do
    sed -i.bak "s/version: '[^']*'/version: '$VERSION'/" "$file"
    rm -f "$file.bak"
done

# RPM spec files
for file in "$PACKAGING_DIR"/linux/rpm/*.spec; do
    sed -i.bak "s/^Version:.*/Version:        $VERSION/" "$file"
    rm -f "$file.bak"
done

# AUR PKGBUILD files
for file in "$PACKAGING_DIR"/linux/aur/*PKGBUILD*; do
    sed -i.bak "s/^pkgver=.*/pkgver=$VERSION/" "$file"
    rm -f "$file.bak"
done

echo "Version updated to $VERSION in all packaging files."
