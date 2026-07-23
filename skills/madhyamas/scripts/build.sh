#!/usr/bin/env bash
# Build Madhyamas skills for all target AI agent harnesses
# Usage: ./scripts/build.sh [--dry-run]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(cd "$SKILL_DIR/../.." && pwd)"
DIST_DIR="$REPO_ROOT/dist"

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
fi

echo "Building Madhyamas skills for all targets..."
echo "Source: $SKILL_DIR"
echo "Output: $DIST_DIR"
echo ""

if [ "$DRY_RUN" = true ]; then
    echo "DRY RUN - no files will be written"
fi

# Function to copy skill directory
copy_skill() {
    local target="$1"
    local dest="$DIST_DIR/$target/madhyamas"

    if [ "$DRY_RUN" = true ]; then
        echo "  [DRY] Would copy to $dest"
        return
    fi

    mkdir -p "$(dirname "$dest")"
    rm -rf "$dest"
    cp -r "$SKILL_DIR" "$dest"
    echo "  Copied to $dest"
}

# Function to flatten skill into single file (for Cursor/Windsurf Rules)
flatten_skill() {
    local target="$1"
    local output_file="$2"
    local frontmatter="$3"

    if [ "$DRY_RUN" = true ]; then
        echo "  [DRY] Would flatten to $output_file"
        return
    fi

    mkdir -p "$(dirname "$DIST_DIR/$output_file")"

    {
        echo "$frontmatter"
        echo ""
        # Extract body (after frontmatter) from SKILL.md
        sed -n '/^---$/,/^---$/!p' "$SKILL_DIR/SKILL.md" | tail -n +1

        # Append all reference files
        for ref in "$SKILL_DIR/references/"*.md; do
            if [ -f "$ref" ]; then
                echo ""
                echo "---"
                echo ""
                echo "# $(basename "$ref" .md | tr '-' ' ' | awk '{for(i=1;i<=NF;i++) $i=toupper(substr($i,1,1)) substr($i,2)}1')"
                echo ""
                # Skip frontmatter if present in reference files
                sed -n '/^---$/,/^---$/!p' "$ref" 2>/dev/null || cat "$ref"
            fi
        done
    } > "$DIST_DIR/$output_file"

    echo "  Flattened to $DIST_DIR/$output_file"
}

# 1. Agent Skills Standard (universal) - no changes
echo "1. Agent Skills Standard (universal)"
copy_skill "agents"

# 2. Claude Code - add allowed-tools to frontmatter
echo "2. Claude Code"
if [ "$DRY_RUN" = false ]; then
    copy_skill "claude"
    # Add Claude-specific frontmatter fields
    SKILL_FILE="$DIST_DIR/claude/madhyamas/SKILL.md"
    # Insert allowed-tools after the metadata block
    if grep -q "^metadata:" "$SKILL_FILE"; then
        sed -i.bak '/^  project-url:/a\allowed-tools: Bash(madhyamas:*) Read Write Edit Grep' "$SKILL_FILE"
        rm -f "$SKILL_FILE.bak"
    fi
    echo "  Added Claude-specific frontmatter"
else
    echo "  [DRY] Would copy and add allowed-tools"
fi

# 3. Devin - add triggers and permissions
echo "3. Devin CLI"
if [ "$DRY_RUN" = false ]; then
    copy_skill "devin"
    SKILL_FILE="$DIST_DIR/devin/madhyamas/SKILL.md"
    if grep -q "^metadata:" "$SKILL_FILE"; then
        sed -i.bak '/^  project-url:/a\triggers: ["user", "model"]' "$SKILL_FILE"
        rm -f "$SKILL_FILE.bak"
    fi
    echo "  Added Devin-specific frontmatter"
else
    echo "  [DRY] Would copy and add triggers"
fi

# 4. Windsurf Skills - minimal frontmatter
echo "4. Windsurf Skills"
copy_skill "windsurf-skills"

# 5. Windsurf Rules - flattened with trigger frontmatter
echo "5. Windsurf Rules (flattened)"
WINDSURF_FM="---
trigger: model_decision
description: >
  Procedural knowledge for using Madhyamas HTTP/HTTPS debugging proxy.
  Use when debugging API traffic, mocking responses, setting breakpoints,
  rewriting traffic, throttling networks, replaying requests, inspecting
  WebSocket/gRPC, exporting HAR, managing sessions, or troubleshooting
  proxy/TLS issues.
---"
flatten_skill "windsurf-rules" "windsurf-rules/madhyamas.md" "$WINDSURF_FM"

# 6. Cursor - flattened .mdc file
echo "6. Cursor (flattened .mdc)"
CURSOR_FM="---
description: >
  Procedural knowledge for using Madhyamas HTTP/HTTPS debugging proxy.
  Use when debugging API traffic, mocking responses, setting breakpoints,
  rewriting traffic, throttling networks, replaying requests, inspecting
  WebSocket/gRPC, exporting HAR, managing sessions, or troubleshooting
  proxy/TLS issues.
alwaysApply: false
---"
flatten_skill "cursor" "cursor/madhyamas.mdc" "$CURSOR_FM"

# 7. OpenCode - minimal frontmatter
echo "7. OpenCode"
copy_skill "opencode"

# 8. CommandCode - minimal frontmatter
echo "8. CommandCode"
copy_skill "commandcode"

# 9. Package as .skill (zip)
echo "9. Package .skill file"
if [ "$DRY_RUN" = false ]; then
    mkdir -p "$DIST_DIR"
    cd "$SKILL_DIR/.."
    zip -r -q "$DIST_DIR/madhyamas.skill" madhyamas/ -x "madhyamas/scripts/.gitkeep" "madhyamas/assets/.gitkeep"
    echo "  Packaged to $DIST_DIR/madhyamas.skill"

    # Also create a universal zip
    cd "$SKILL_DIR/.."
    zip -r -q "$DIST_DIR/madhyamas-skill.zip" madhyamas/
    echo "  Packaged to $DIST_DIR/madhyamas-skill.zip"
else
    echo "  [DRY] Would package .skill and .zip"
fi

echo ""
echo "Build complete!"
echo "Outputs in: $DIST_DIR"

if [ "$DRY_RUN" = false ]; then
    echo ""
    echo "Generated files:"
    find "$DIST_DIR" -type f | sort
fi
