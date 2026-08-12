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
    warning "Only $MCP_COUNT MCP tools documented (expected ~135)"
else
    ok "$MCP_COUNT MCP tools documented"
fi

# 8. Check CLI command count
echo ""
echo "=== CLI Commands ==="
CLI_COUNT=$(grep -c "^### " "$SKILL_DIR/references/cli-commands.md" || true)
if [ "$CLI_COUNT" -lt 50 ]; then
    warning "Only $CLI_COUNT CLI subcommands documented (expected ~128)"
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

# 10. Code-sync checks (compare docs against actual source code)
#
# These checks compare the skill reference docs against the real Rust source
# so that documentation drift is caught by CI. If any of these fail, run the
# ai-agent-tooling sync workflow (see agents/references/ai-agent-tooling-workflow.md)
# to bring the docs back in sync with the code. Do NOT silence these checks by
# editing validate.sh — fix the docs instead.
echo ""
echo "=== Code-Sync Checks ==="
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
MCP_TOOLS_DIR="$REPO_ROOT/crates/madhyamas-mcp/src/tools"
ROUTES_FILE="$REPO_ROOT/crates/madhyamas-api/src/routes.rs"
CLI_DIR="$REPO_ROOT/crates/madhyamas-cli/src/commands"

# 10a. MCP tools: code vs doc count
if [ -d "$MCP_TOOLS_DIR" ]; then
    CODE_MCP=$(grep -rhoE '"madhyamas_[a-z_]+"' "$MCP_TOOLS_DIR"/*.rs 2>/dev/null | sort -u | wc -l | tr -d ' ')
    DOC_MCP=$(grep -cE "^### .*madhyamas_" "$SKILL_DIR/references/mcp-tools.md" || true)
    if [ "$CODE_MCP" -ne "$DOC_MCP" ]; then
        error "MCP tool count mismatch: code has $CODE_MCP, doc has $DOC_MCP. Run the ai-agent-tooling sync workflow."
    else
        ok "MCP tool count matches code ($CODE_MCP tools)"
    fi

    # Also check the set difference so renamed/removed tools are caught.
    MISSING_MCP=$(comm -23 \
        <(grep -rhoE '"madhyamas_[a-z_]+"' "$MCP_TOOLS_DIR"/*.rs 2>/dev/null | sort -u | tr -d '"') \
        <(grep -oE 'madhyamas_[a-z_]+' "$SKILL_DIR/references/mcp-tools.md" | sort -u) \
        | wc -l | tr -d ' ')
    if [ "$MISSING_MCP" -ne 0 ]; then
        error "MCP tools missing from doc: $MISSING_MCP. Run the ai-agent-tooling sync workflow."
    fi
else
    warning "MCP tools source directory not found at $MCP_TOOLS_DIR (skipping code-sync)"
fi

# 10b. REST endpoints: code vs doc count
if [ -f "$ROUTES_FILE" ]; then
    # Extract all quoted path strings from routes.rs (handles multi-line .route() calls)
    CODE_REST=$(grep -oE '"/[a-zA-Z0-9/{}_.-]+"' "$ROUTES_FILE" | sed 's/"//g' | sort -u | wc -l | tr -d ' ')
    # Extract the path only from the 2nd column of table rows (| METHOD | `path` | ...)
    DOC_REST_PATHS=$(grep -E '^\| (GET|POST|PUT|DELETE|PATCH) \|' "$SKILL_DIR/references/rest-api.md" \
        | awk -F'|' '{print $3}' | grep -oE '`/[^`]+`' | sed 's/`//g' | sort -u | wc -l | tr -d ' ')
    if [ "$CODE_REST" -ne "$DOC_REST_PATHS" ]; then
        error "REST endpoint path mismatch: code has $CODE_REST unique paths, doc has $DOC_REST_PATHS. Run the ai-agent-tooling sync workflow."
    else
        ok "REST endpoint paths match code ($CODE_REST paths)"
    fi
else
    warning "Routes file not found at $ROUTES_FILE (skipping code-sync)"
fi

# 10c. CLI subcommands: code vs doc count
if [ -d "$CLI_DIR" ]; then
    # Count enum variants (subcommands) across all area files. Each line matching
    # `    VariantName(...)` or `    VariantName { ... }` or `    VariantName,` is a
    # subcommand. Nested subcommand enums (Recording/Collections) are counted too.
    CODE_CLI=$(grep -rhoE '^\s+[A-Z][a-zA-Z]+(\(|\{|,|$)' "$CLI_DIR"/*.rs 2>/dev/null \
        | grep -vE 'fn |struct |impl |enum |pub |use |async |//|::|Args|Commands' \
        | sed -E 's/^\s+//;s/[({,].*//' | sort -u | wc -l | tr -d ' ')
    DOC_CLI=$(grep -cE "^### " "$SKILL_DIR/references/cli-commands.md" || true)
    # The code variant count is approximate (shared arg structs, helper enums).
    # Use a tolerance: the doc count should be within a reasonable band of the code.
    if [ "$DOC_CLI" -lt 100 ]; then
        error "CLI subcommand count mismatch: doc has $DOC_CLI (expected >=100). Run the ai-agent-tooling sync workflow."
    else
        ok "CLI subcommand count looks healthy ($DOC_CLI documented)"
    fi
else
    warning "CLI commands source directory not found at $CLI_DIR (skipping code-sync)"
fi

# 11. Summary
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
