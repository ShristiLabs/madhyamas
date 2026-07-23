#!/usr/bin/env bash
# Validate Madhyamas skills package
# Usage: ./scripts/validate.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"

ERRORS=0
WARNINGS=0

error() {
    echo "ERROR: $1"
    ERRORS=$((ERRORS + 1))
}

warning() {
    echo "WARNING: $1"
    WARNINGS=$((WARNINGS + 1))
}

ok() {
    echo "OK: $1"
}

echo "Validating Madhyamas skills at: $SKILL_DIR"
echo ""

# 1. Check required files exist
echo "=== File Structure ==="
if [ ! -f "$SKILL_DIR/SKILL.md" ]; then
    error "SKILL.md not found"
else
    ok "SKILL.md exists"
fi

if [ ! -d "$SKILL_DIR/references" ]; then
    error "references/ directory not found"
else
    ok "references/ directory exists"
fi

# 2. Check SKILL.md frontmatter
echo ""
echo "=== SKILL.md Frontmatter ==="
if ! head -1 "$SKILL_DIR/SKILL.md" | grep -q "^---$"; then
    error "SKILL.md missing YAML frontmatter opening ---"
else
    ok "Frontmatter opening --- found"
fi

if ! grep -q "^name: madhyamas$" "$SKILL_DIR/SKILL.md"; then
    error "SKILL.md missing 'name: madhyamas' in frontmatter"
else
    ok "name field present"
fi

if ! grep -q "^description:" "$SKILL_DIR/SKILL.md"; then
    error "SKILL.md missing 'description' in frontmatter"
else
    ok "description field present"
fi

if ! grep -q "^license:" "$SKILL_DIR/SKILL.md"; then
    warning "SKILL.md missing 'license' in frontmatter"
else
    ok "license field present"
fi

# 3. Check SKILL.md line count (should be under 500)
echo ""
echo "=== SKILL.md Size ==="
SKILL_LINES=$(wc -l < "$SKILL_DIR/SKILL.md")
if [ "$SKILL_LINES" -gt 500 ]; then
    error "SKILL.md is $SKILL_LINES lines (should be under 500)"
else
    ok "SKILL.md is $SKILL_LINES lines (under 500)"
fi

# 4. Check all referenced files exist
echo ""
echo "=== Reference Files ==="
EXPECTED_REFS=(
    "setup.md"
    "mcp-tools.md"
    "cli-commands.md"
    "rest-api.md"
    "traffic-inspection.md"
    "mocking.md"
    "breakpoints.md"
    "rewrites.md"
    "throttling.md"
    "replay.md"
    "sessions.md"
    "grpc.md"
    "scripting.md"
    "plugins.md"
    "websockets.md"
    "export-import.md"
    "troubleshooting.md"
    "harness-setup.md"
)

for ref in "${EXPECTED_REFS[@]}"; do
    if [ ! -f "$SKILL_DIR/references/$ref" ]; then
        error "Missing reference file: references/$ref"
    fi
done
ok "All ${#EXPECTED_REFS[@]} expected reference files checked"

# 5. Check for broken links in SKILL.md
echo ""
echo "=== Link Validation ==="
while IFS= read -r line; do
    # Extract markdown links to references/
    if [[ "$line" =~ references/([a-z-]+\.md) ]]; then
        ref_file="${BASH_REMATCH[1]}"
        if [ ! -f "$SKILL_DIR/references/$ref_file" ]; then
            error "Broken link in SKILL.md: references/$ref_file"
        fi
    fi
done < "$SKILL_DIR/SKILL.md"
ok "SKILL.md links validated"

# 6. Check no emojis in files
echo ""
echo "=== Content Checks ==="
if grep -rP '[\x{1F300}-\x{1F9FF}]' "$SKILL_DIR" --include="*.md" 2>/dev/null; then
    warning "Emojis found in markdown files"
else
    ok "No emojis in markdown files"
fi

# 7. Check MCP tool count
echo ""
echo "=== MCP Tools ==="
MCP_COUNT=$(grep -c "^### madhyamas_" "$SKILL_DIR/references/mcp-tools.md" || true)
if [ "$MCP_COUNT" -lt 60 ]; then
    warning "Only $MCP_COUNT MCP tools documented (expected ~67)"
else
    ok "$MCP_COUNT MCP tools documented"
fi

# 8. Check CLI command count
echo ""
echo "=== CLI Commands ==="
CLI_COUNT=$(grep -c "^### " "$SKILL_DIR/references/cli-commands.md" || true)
if [ "$CLI_COUNT" -lt 50 ]; then
    warning "Only $CLI_COUNT CLI subcommands documented (expected ~58)"
else
    ok "$CLI_COUNT CLI subcommands documented"
fi

# 9. Check scripts are executable
echo ""
echo "=== Scripts ==="
for script in build.sh install.sh validate.sh; do
    if [ ! -f "$SKILL_DIR/scripts/$script" ]; then
        error "Missing script: scripts/$script"
    elif [ ! -x "$SKILL_DIR/scripts/$script" ]; then
        warning "Script not executable: scripts/$script"
    else
        ok "scripts/$script exists and is executable"
    fi
done

# 10. Summary
echo ""
echo "=== Summary ==="
echo "Errors:   $ERRORS"
echo "Warnings: $WARNINGS"

if [ "$ERRORS" -gt 0 ]; then
    echo ""
    echo "VALIDATION FAILED - fix errors above"
    exit 1
else
    echo ""
    echo "VALIDATION PASSED"
    if [ "$WARNINGS" -gt 0 ]; then
        echo "($WARNINGS warnings to review)"
    fi
    exit 0
fi
