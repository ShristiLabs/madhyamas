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

# Docs that are design/analysis documents referencing proposed files that
# don't exist yet. Internal markdown links are still checked; only source
# path references (crates/..., web/src/...) are exempted for these docs.
DESIGN_DOCS="
ENTERPRISE_OVERVIEW.md
ENTERPRISE_AUTH_RBAC.md
ENTERPRISE_CICD.md
ENTERPRISE_LICENSING_SERVER.md
ENTERPRISE_MULTI_INSTANCE.md
ENTERPRISE_PERF_SECURITY.md
ENTERPRISE_STORAGE_TRAITS.md
ENTERPRISE_WEB_UI.md
ENTERPRISE_OSS_COMPARISON.md
ENTERPRISE_AI_AGENTS.md
ENTERPRISE_CRATE_MIGRATION.md
ENTERPRISE_IMPLEMENTATION_PLAN.md
"

is_design_doc() {
    local name="$1"
    echo "$DESIGN_DOCS" | grep -qx "$name"
}

for doc in "$DOCS_DIR"/*.md; do
    # Skip TEMPLATE.md — it contains example links by design
    [ "$(basename "$doc")" = "TEMPLATE.md" ] && continue
    doc_name="$(basename "$doc")"
    # Design docs: skip source path check (they reference proposed files),
    # but still check internal markdown links.
    skip_paths=0
    if is_design_doc "$doc_name"; then
        skip_paths=1
    fi
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
    #    Skip entirely for design docs (they reference proposed files).
    [ "$skip_paths" = "1" ] && continue
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
