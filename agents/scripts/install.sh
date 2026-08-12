#!/usr/bin/env bash
# Install Madhyamas specialized agents to all supported AI agent harnesses.
#
# Source of truth: agents/agents/*.md (canonical subagent definitions) and
# agents/references/*.md (shared reference files loaded on demand).
#
# This script is LLM-agnostic and harness-agnostic:
#   - No model names are pinned in the canonical source. Harnesses that
#     require a `model:` field get `model: inherit` injected at install time.
#   - Each harness receives the format it expects (subagent profiles AND
#     slash-command skill wrappers).
#
# Usage:
#   bash agents/scripts/install.sh                # install everything
#   bash agents/scripts/install.sh --dry-run      # preview without writing
#   bash agents/scripts/install.sh claude         # only Claude Code
#   bash agents/scripts/install.sh devin          # only Devin CLI
#   bash agents/scripts/install.sh agents         # only universal .agents/
#   bash agents/scripts/install.sh windsurf       # only Windsurf
#   bash agents/scripts/install.sh cursor         # only Cursor
#   bash agents/scripts/install.sh opencode       # only OpenCode
#   bash agents/scripts/install.sh commandcode    # only CommandCode
#   bash agents/scripts/install.sh all            # everything (default)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AGENTS_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(cd "$AGENTS_DIR/.." && pwd)"
SRC_AGENTS="$AGENTS_DIR/agents"
SRC_REFS="$AGENTS_DIR/references"

DRY_RUN=false
TARGET="${1:-all}"
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
    TARGET="all"
fi

# --- helpers -----------------------------------------------------------------

have() { [[ -d "$SRC_AGENTS" ]] && compgen -G "$SRC_AGENTS/*.md" >/dev/null; }

# Extract YAML frontmatter and body from a canonical agent file.
#   $1 = source file
# Prints frontmatter (without the --- delimiters) on stdout lines, then a
# sentinel line "<<BODY>>", then the body.
split_frontmatter() {
    local file="$1"
    awk '
        NR==1 && /^---$/ { in_fm=1; next }
        in_fm && /^---$/ { in_fm=0; print "<<BODY>>"; next }
        in_fm { print }
        !in_fm { print }
    ' "$file"
}

# Get a single frontmatter value (simple top-level key, list, or folded block).
#   $1 = source file, $2 = key name
fm_value() {
    local file="$1" key="$2"
    # Handles `key: value`, `key: >\n  folded`, and `key:\n  - item` lists.
    awk -v k="$key" '
        $0 ~ "^"k":" {
            v=$0; sub("^"k": *","",v)
            if (v==">" || v=="|") {
                getline line
                while (line ~ /^  /) { print line; sub(/^  /,"",line); getline line }
                next
            }
            if (v=="") {
                # list form
                getline line
                while (line ~ /^  - /) { gsub(/^  - /,"",line); print line; getline line }
                next
            }
            print v
            exit
        }
    ' "$file"
}

# Write a subagent profile file with harness-specific frontmatter adjustments.
#   $1 = dest file path, $2 = source agent file
emit_subagent() {
    local dest="$1" src="$2"
    local name desc color tools
    name="$(fm_value "$src" name)"
    desc="$(fm_value "$src" description)"
    color="$(fm_value "$src" color)"
    # tools list
    tools="$(awk '/^allowed-tools:/{flag=1;next} flag && /^  - /{gsub(/^  - /,"");print;next} flag && !/^  - /{exit}' "$src" | tr '\n' ',' | sed 's/,*$//')"

    if $DRY_RUN; then echo "  [DRY] subagent -> $dest ($name)"; return; fi
    mkdir -p "$(dirname "$dest")"

    # Build frontmatter per harness conventions:
    # - Claude Code uses `tools:` (array) and requires `model:`.
    # - Devin uses `allowed-tools:` (list) and optional `model:`.
    # - Universal .agents/ uses `allowed-tools:`.
    # We emit a superset that all three accept; Claude also accepts `tools`.
    {
        echo "---"
        echo "name: $name"
        # Collapse folded description into a single line for portability.
        echo "description: $(echo "$desc" | tr '\n' ' ' | sed 's/  */ /g')"
        echo "model: inherit"
        echo "color: ${color:-blue}"
        echo "allowed-tools:"
        echo "$tools" | tr ',' '\n' | sed 's/^/  - /'
        echo "---"
        echo ""
        # Body (everything after the closing --- in the source).
        awk 'NR==1 && /^---$/{in_fm=1;next} in_fm && /^---$/{in_fm=0;next} !in_fm' "$src"
    } > "$dest"
    echo "  subagent -> $dest"
}

# Write a slash-command skill wrapper (SKILL.md) for an agent.
#   $1 = dest skill dir, $2 = source agent file
emit_skill() {
    local dest_dir="$1" src="$2"
    local name desc color tools triggers
    name="$(fm_value "$src" name)"
    desc="$(fm_value "$src" description)"
    color="$(fm_value "$src" color)"
    tools="$(awk '/^allowed-tools:/{flag=1;next} flag && /^  - /{gsub(/^  - /,"");print;next} flag && !/^  - /{exit}' "$src" | tr '\n' ',' | sed 's/,*$//')"
    triggers="$(awk '/^triggers:/{flag=1;next} flag && /^  - /{gsub(/^  - /,"");print;next} flag && !/^  - /{exit}' "$src" | tr '\n' ',' | sed 's/,*$//')"

    if $DRY_RUN; then echo "  [DRY] skill    -> $dest_dir/SKILL.md ($name)"; return; fi
    mkdir -p "$dest_dir"

    {
        echo "---"
        echo "name: $name"
        echo "description: $(echo "$desc" | tr '\n' ' ' | sed 's/  */ /g')"
        echo "color: ${color:-blue}"
        echo "allowed-tools:"
        echo "$tools" | tr ',' '\n' | sed 's/^/  - /'
        echo "triggers:"
        echo "$triggers" | tr ',' '\n' | sed 's/^/  - /'
        echo "---"
        echo ""
        echo "# $(echo "$name" | tr '-' ' ' | awk '{for(i=1;i<=NF;i++)$i=toupper(substr($i,1,1))substr($i,2)}1')"
        echo ""
        echo "Invoke the **$name** agent. The body below is the agent's system prompt."
        echo ""
        awk 'NR==1 && /^---$/{in_fm=1;next} in_fm && /^---$/{in_fm=0;next} !in_fm' "$src"
    } > "$dest_dir/SKILL.md"
    echo "  skill    -> $dest_dir/SKILL.md"
}

# Copy the shared references directory next to a skill output.
#   $1 = dest references dir
copy_refs() {
    local dest="$1"
    if $DRY_RUN; then echo "  [DRY] refs     -> $dest"; return; fi
    mkdir -p "$dest"
    cp "$SRC_REFS"/*.md "$dest/"
    echo "  refs     -> $dest"
}

# --- per-harness installers --------------------------------------------------

install_agents_universal() {
    echo "Universal .agents/ standard"
    for src in "$SRC_AGENTS"/*.md; do
        [[ -f "$src" ]] || continue
        local name; name="$(fm_value "$src" name)"
        emit_subagent "$REPO_ROOT/.agents/agents/$name.md" "$src"
        emit_skill   "$REPO_ROOT/.agents/skills/$name"      "$src"
    done
    copy_refs "$REPO_ROOT/.agents/skills/_shared-references"
}

install_claude() {
    echo "Claude Code (.claude/)"
    for src in "$SRC_AGENTS"/*.md; do
        [[ -f "$src" ]] || continue
        local name; name="$(fm_value "$src" name)"
        emit_subagent "$REPO_ROOT/.claude/agents/$name.md" "$src"
        emit_skill   "$REPO_ROOT/.claude/skills/$name"      "$src"
    done
    copy_refs "$REPO_ROOT/.claude/skills/_shared-references"
}

install_devin() {
    echo "Devin CLI (.devin/)"
    for src in "$SRC_AGENTS"/*.md; do
        [[ -f "$src" ]] || continue
        local name; name="$(fm_value "$src" name)"
        emit_subagent "$REPO_ROOT/.devin/agents/$name.md" "$src"
        emit_skill   "$REPO_ROOT/.devin/skills/$name"      "$src"
    done
    copy_refs "$REPO_ROOT/.devin/skills/_shared-references"
}

install_windsurf() {
    echo "Windsurf (.windsurf/)"
    for src in "$SRC_AGENTS"/*.md; do
        [[ -f "$src" ]] || continue
        local name; name="$(fm_value "$src" name)"
        emit_subagent "$REPO_ROOT/.windsurf/agents/$name.md" "$src"
        emit_skill   "$REPO_ROOT/.windsurf/skills/$name"      "$src"
    done
    copy_refs "$REPO_ROOT/.windsurf/skills/_shared-references"
}

install_cursor() {
    # Cursor flattens rules into single .mdc files under .cursor/rules/.
    # We emit one rule file per agent (subagent-style; Cursor does not have
    # a separate skill concept).
    echo "Cursor (.cursor/rules/)"
    for src in "$SRC_AGENTS"/*.md; do
        [[ -f "$src" ]] || continue
        local name desc color tools
        name="$(fm_value "$src" name)"
        desc="$(fm_value "$src" description)"
        color="$(fm_value "$src" color)"
        tools="$(awk '/^allowed-tools:/{flag=1;next} flag && /^  - /{gsub(/^  - /,"");print;next} flag && !/^  - /{exit}' "$src" | tr '\n' ',' | sed 's/,*$//')"
        local dest="$REPO_ROOT/.cursor/rules/$name.mdc"
        if $DRY_RUN; then echo "  [DRY] rule     -> $dest ($name)"; continue; fi
        mkdir -p "$(dirname "$dest")"
        {
            echo "---"
            echo "description: $(echo "$desc" | tr '\n' ' ' | sed 's/  */ /g')"
            echo "globs: **/*"
            echo "alwaysApply: false"
            echo "---"
            echo ""
            awk 'NR==1 && /^---$/{in_fm=1;next} in_fm && /^---$/{in_fm=0;next} !in_fm' "$src"
        } > "$dest"
        echo "  rule     -> $dest"
    done
}

install_opencode() {
    echo "OpenCode (.opencode/)"
    for src in "$SRC_AGENTS"/*.md; do
        [[ -f "$src" ]] || continue
        local name; name="$(fm_value "$src" name)"
        emit_subagent "$REPO_ROOT/.opencode/agents/$name.md" "$src"
        emit_skill   "$REPO_ROOT/.opencode/skills/$name"      "$src"
    done
    copy_refs "$REPO_ROOT/.opencode/skills/_shared-references"
}

install_commandcode() {
    echo "CommandCode (.commandcode/)"
    for src in "$SRC_AGENTS"/*.md; do
        [[ -f "$src" ]] || continue
        local name; name="$(fm_value "$src" name)"
        emit_subagent "$REPO_ROOT/.commandcode/agents/$name.md" "$src"
        emit_skill   "$REPO_ROOT/.commandcode/skills/$name"      "$src"
    done
    copy_refs "$REPO_ROOT/.commandcode/skills/_shared-references"
}

# --- main --------------------------------------------------------------------

if ! have; then
    echo "No agent definitions found in $SRC_AGENTS" >&2
    exit 1
fi

echo "Installing Madhyamas agents from $SRC_AGENTS"
echo "Repo root: $REPO_ROOT"
$DRY_RUN && echo "DRY RUN — no files will be written"
echo ""

case "$TARGET" in
    all)
        install_agents_universal
        install_claude
        install_devin
        install_windsurf
        install_cursor
        install_opencode
        install_commandcode
        ;;
    agents)       install_agents_universal ;;
    claude)       install_claude ;;
    devin)        install_devin ;;
    windsurf)     install_windsurf ;;
    cursor)       install_cursor ;;
    opencode)     install_opencode ;;
    commandcode)  install_commandcode ;;
    *) echo "Unknown target: $TARGET" >&2; echo "Valid: all|agents|claude|devin|windsurf|cursor|opencode|commandcode" >&2; exit 1 ;;
esac

echo ""
echo "Done. Re-run with --dry-run to preview. Validate with:"
echo "  bash agents/scripts/validate.sh"
