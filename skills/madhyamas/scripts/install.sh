#!/usr/bin/env bash
# Install Madhyamas skills to a specific AI agent harness
# Usage: ./scripts/install.sh <target> [--global]
# Targets: claude, devin, windsurf, cursor, opencode, commandcode, agents, all

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(cd "$SKILL_DIR/../.." && pwd)"
DIST_DIR="$REPO_ROOT/dist"

TARGET="${1:-}"
GLOBAL="${2:-}"

if [ -z "$TARGET" ]; then
    echo "Usage: $0 <target> [--global]"
    echo "Targets: claude, devin, windsurf, cursor, opencode, commandcode, agents, all"
    exit 1
fi

# Build if dist doesn't exist
if [ ! -d "$DIST_DIR" ]; then
    echo "dist/ not found, building first..."
    bash "$SCRIPT_DIR/build.sh"
fi

# Determine install paths
get_install_path() {
    local target="$1"
    local global="$2"

    case "$target" in
        claude)
            if [ "$global" = "--global" ]; then
                echo "$HOME/.claude/skills/madhyamas"
            else
                echo "$REPO_ROOT/.claude/skills/madhyamas"
            fi
            ;;
        devin)
            if [ "$global" = "--global" ]; then
                echo "$HOME/.config/devin/skills/madhyamas"
            else
                echo "$REPO_ROOT/.devin/skills/madhyamas"
            fi
            ;;
        windsurf)
            if [ "$global" = "--global" ]; then
                echo "$HOME/.codeium/windsurf/skills/madhyamas"
            else
                echo "$REPO_ROOT/.windsurf/skills/madhyamas"
            fi
            ;;
        cursor)
            if [ "$global" = "--global" ]; then
                echo "$HOME/.cursor/rules/madhyamas.mdc"
            else
                echo "$REPO_ROOT/.cursor/rules/madhyamas.mdc"
            fi
            ;;
        opencode)
            if [ "$global" = "--global" ]; then
                echo "$HOME/.config/opencode/skills/madhyamas"
            else
                echo "$REPO_ROOT/.opencode/skills/madhyamas"
            fi
            ;;
        commandcode)
            if [ "$global" = "--global" ]; then
                echo "$HOME/.commandcode/skills/madhyamas"
            else
                echo "$REPO_ROOT/.commandcode/skills/madhyamas"
            fi
            ;;
        agents)
            echo "$REPO_ROOT/.agents/skills/madhyamas"
            ;;
        *)
            echo ""
            ;;
    esac
}

install_target() {
    local target="$1"
    local install_path
    install_path=$(get_install_path "$target" "$GLOBAL")

    if [ -z "$install_path" ]; then
        echo "Unknown target: $target"
        return 1
    fi

    local source_dir=""
    case "$target" in
        claude) source_dir="$DIST_DIR/claude/madhyamas" ;;
        devin) source_dir="$DIST_DIR/devin/madhyamas" ;;
        windsurf) source_dir="$DIST_DIR/windsurf-skills/madhyamas" ;;
        cursor) source_dir="$DIST_DIR/cursor/madhyamas.mdc" ;;
        opencode) source_dir="$DIST_DIR/opencode/madhyamas" ;;
        commandcode) source_dir="$DIST_DIR/commandcode/madhyamas" ;;
        agents) source_dir="$DIST_DIR/agents/madhyamas" ;;
    esac

    if [ ! -e "$source_dir" ]; then
        echo "Source not found: $source_dir. Run build.sh first."
        return 1
    fi

    echo "Installing $target to: $install_path"

    # Create parent directory
    mkdir -p "$(dirname "$install_path")"

    # Remove existing installation
    rm -rf "$install_path"

    # Copy or symlink
    if [ -d "$source_dir" ]; then
        cp -r "$source_dir" "$install_path"
    else
        cp "$source_dir" "$install_path"
    fi

    echo "  Installed successfully"
}

if [ "$TARGET" = "all" ]; then
    echo "Installing to all targets..."
    for t in claude devin windsurf cursor opencode commandcode agents; do
        install_target "$t" || true
    done
else
    install_target "$TARGET"
fi

echo ""
echo "Installation complete!"
echo "Restart your AI agent to load the skill."
