#!/usr/bin/env bash
# Check docs/ for broken internal links and references to non-existent source paths.
#
# Usage: bash scripts/check-docs.sh
#
# Checks:
# 1. Internal markdown links [text](file.md) — target must exist relative to docs/
# 2. Source path references like `crates/madhyamas-core/src/...` — path must exist
#
# Exits non-zero on any broken reference.

set -euo pipefail

DOCS_DIR="$(cd "$(dirname "$0")/.." && pwd)/docs"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
errors=0

echo "Checking internal links and source path references in docs/..."

for doc in "$DOCS_DIR"/*.md; do
    # Skip TEMPLATE.md — it contains example links by design
    [ "$(basename "$doc")" = "TEMPLATE.md" ] && continue
    # 1. Check internal markdown links: [text](something.md)
    #    Extract .md targets (ignore http/https URLs and anchors)
    while IFS= read -r target; do
        # Skip external URLs and anchors
        case "$target" in
            http://*|https://*|\#*) continue ;;
        esac
        # Strip anchor
        file_part="${target%%#*}"
        [ -z "$file_part" ] && continue
        full="$DOCS_DIR/$file_part"
        if [ ! -f "$full" ]; then
            echo "  BROKEN LINK: $(basename "$doc") -> $target"
            errors=$((errors + 1))
        fi
    done < <(grep -oE '\]\([^)]+\)' "$doc" | sed 's/^](//;s/)$//' | grep '\.md' || true)

    # 2. Check source path references: `crates/...` or `web/src/...`
    #    Skip paths inside fenced code blocks (``` ... ```) since those are
    #    often illustrative examples, not real file references.
    while IFS= read -r ref_path; do
        full="$REPO_ROOT/${ref_path%/}"
        if [ ! -e "$full" ]; then
            echo "  STALE PATH:  $(basename "$doc") -> $ref_path"
            errors=$((errors + 1))
        fi
    done < <(awk '/^```/{in_block=!in_block; next} !in_block' "$doc" | grep -oE '(crates|web/src)/[a-zA-Z0-9_/.-]+' | sort -u || true)
done

if [ "$errors" -gt 0 ]; then
    echo ""
    echo "FAIL: $errors broken reference(s) found in docs/"
    exit 1
else
    echo "OK: all internal links and source path references resolve."
    exit 0
fi
