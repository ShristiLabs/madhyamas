#!/usr/bin/env bash
# Check that every public module in crates/madhyamas-core/src/ has a
# corresponding docs/ reference page (or is explicitly exempted).
#
# Usage: bash scripts/check-docs-coverage.sh
#
# Exits non-zero if any module lacks documentation.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE_SRC="$REPO_ROOT/crates/madhyamas-core/src"
DOCS_DIR="$REPO_ROOT/docs"
errors=0

# Modules documented indirectly (part of another doc) or intentionally
# undocumented. Format: "module_name:reason"
EXEMPT="
config:covered by ARCHITECTURE.md and GETTING_STARTED.md
error:covered by ARCHITECTURE.md (error types)
replay:covered by API_INTERCEPT.md, EDIT_THEN_REPEAT.md, REPEAT_ADVANCED.md
session:covered by PERSISTENCE.md and API_TRAFFIC.md
websocket:covered by API_WEBSOCKET_GRPC.md
"

is_exempt() {
    local mod="$1"
    while IFS=: read -r name reason; do
        [ -z "$name" ] && continue
        if [ "$name" = "$mod" ]; then
            return 0
        fi
    done <<< "$EXEMPT"
    return 1
}

echo "Checking docs coverage for madhyamas-core modules..."

for item in "$CORE_SRC"/*; do
    name="$(basename "$item")"
    module_name="${name%.rs}"
    [ "$module_name" = "mod" ] && continue
    [ "$module_name" = "lib" ] && continue

    # Check for an exact-match doc (case-insensitive)
    doc_name="$(echo "$module_name" | tr '[:lower:]' '[:upper:]')"
    found=""
    for doc in "$DOCS_DIR"/*.md; do
        base="$(basename "$doc" .md)"
        base_upper="$(echo "$base" | tr '[:lower:]' '[:upper:]')"
        if [ "$base_upper" = "$doc_name" ]; then
            found="$doc"
            break
        fi
    done

    if [ -n "$found" ]; then
        continue
    fi

    # Check exemption list
    if is_exempt "$module_name"; then
        continue
    fi

    # Check if any doc mentions the module name
    if grep -rql "$module_name" "$DOCS_DIR"/*.md 2>/dev/null; then
        continue
    fi

    echo "  UNDOCUMENTED: $module_name ($item)"
    errors=$((errors + 1))
done

if [ "$errors" -gt 0 ]; then
    echo ""
    echo "FAIL: $errors module(s) lack documentation."
    echo "Create a docs/ page or add to the EXEMPT list in scripts/check-docs-coverage.sh"
    exit 1
else
    echo "OK: all madhyamas-core modules have documentation coverage."
    exit 0
fi
