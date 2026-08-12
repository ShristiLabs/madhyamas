#!/usr/bin/env bash
# Validate the Madhyamas specialized agents package.
#
# Checks:
#   1. Required directories exist (agents/, references/, scripts/)
#   2. Every agent file has valid YAML frontmatter with required fields
#   3. No model names are pinned in canonical source (LLM-agnostic)
#   4. Every referenced file in agent bodies exists
#   5. Agent system-prompt bodies are under 500 lines
#   6. No emojis in agent or reference markdown
#   7. Scripts are executable
#
# Usage: bash agents/scripts/validate.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AGENTS_DIR="$(dirname "$SCRIPT_DIR")"
SRC_AGENTS="$AGENTS_DIR/agents"
SRC_REFS="$AGENTS_DIR/references"
SRC_SCRIPTS="$AGENTS_DIR/scripts"

errors=0
warnings=0
pass() { printf '  \033[32mok\033[0m   %s\n' "$1"; }
fail() { printf '  \033[31mFAIL\033[0m %s\n' "$1"; errors=$((errors+1)); }
warn() { printf '  \033[33mwarn\033[0m %s\n' "$1"; warnings=$((warnings+1)); }

# Model names that would violate LLM-agnosticism. Match specific model
# identifiers, not bare product prefixes (so "claude-desktop-config" or
# "gpt-partition" do not false-positive).
MODEL_PATTERN='(^|[^a-zA-Z])(gpt-[0-9]|claude-(sonnet|opus|haiku|[0-9])|gemini|llama|mistral|glm-[0-9]|grok|swe-[0-9])'

echo "Validating Madhyamas agents package at $AGENTS_DIR"
echo ""

# 1. Required directories
echo "1. Directory structure"
for d in "$SRC_AGENTS" "$SRC_REFS" "$SRC_SCRIPTS"; do
    [[ -d "$d" ]] && pass "directory $d" || fail "missing directory $d"
done
echo ""

# 2. Agent files present
echo "2. Agent definitions"
shopt -s nullglob
agent_files=( "$SRC_AGENTS"/*.md )
if [[ ${#agent_files[@]} -eq 0 ]]; then
    fail "no agent .md files in $SRC_AGENTS"
else
    pass "found ${#agent_files[@]} agent definition(s)"
fi
echo ""

# 3. Frontmatter + LLM-agnosticism + line count + emoji check
echo "3. Per-agent checks"
REQUIRED_FIELDS=(name description color allowed-tools triggers)
EMOJI_PATTERN=$'[\xF0\x9F\x80-\xBF]'

# Detect emoji using perl (portable across macOS/Linux); fall back to a raw
# byte grep if perl is unavailable. We only flag actual emoji (U+1F000 and
# above), not typographic arrows (U+2190+) or dingbats used in prose.
has_emoji() {
    # Exit 0 (true) if an emoji is found, non-zero otherwise.
    if command -v perl >/dev/null 2>&1; then
        perl -CSD -ne '$f=1 if /[\x{1F000}-\x{1FAFF}\x{1F300}-\x{1F9FF}]/; END{exit($f?0:1)}' "$1"
        return $?
    fi
    # Fallback: raw byte grep for the UTF-8 emoji lead bytes (F0 9F ...).
    LC_ALL=C grep -q $'\xF0\x9F' "$1"
}

for src in "$SRC_AGENTS"/*.md; do
    [[ -f "$src" ]] || continue
    name="$(basename "$src" .md)"

    # Frontmatter must start with --- on line 1 and close with ---.
    if ! head -1 "$src" | grep -q '^---$'; then
        fail "$name: missing opening --- frontmatter delimiter"
        continue
    fi
    # Find closing delimiter.
    close_line=$(awk 'NR>1 && /^---$/{print NR; exit}' "$src")
    if [[ -z "$close_line" ]]; then
        fail "$name: missing closing --- frontmatter delimiter"
        continue
    fi

    # Required fields.
    fm_body=$(sed -n "2,$((close_line-1))p" "$src")
    for field in "${REQUIRED_FIELDS[@]}"; do
        if ! echo "$fm_body" | grep -q "^${field}:"; then
            fail "$name: missing required frontmatter field '$field'"
        fi
    done

    # name field must match filename.
    fm_name=$(echo "$fm_body" | awk -F': *' '/^name:/{print $2; exit}')
    if [[ "$fm_name" != "$name" ]]; then
        fail "$name: frontmatter name '$fm_name' does not match filename"
    fi

    # LLM-agnostic: no model names anywhere in the file.
    if grep -Eq "$MODEL_PATTERN" "$src"; then
        match=$(grep -Eo "$MODEL_PATTERN" "$src" | head -1)
        fail "$name: contains a model name ($match) — must be LLM-agnostic"
    fi

    # Body line count (after frontmatter).
    body_lines=$(awk -v c="$close_line" 'NR>c' "$src" | wc -l | tr -d ' ')
    if [[ "$body_lines" -gt 500 ]]; then
        fail "$name: body is $body_lines lines (max 500)"
    else
        pass "$name: body $body_lines lines"
    fi

    # Emoji check.
    if has_emoji "$src"; then
        fail "$name: contains emoji (not allowed)"
    fi
done
echo ""

# 4. Referenced files exist (agents/references/*.md and relative ./ paths)
echo "4. Reference file existence"
ref_files=( "$SRC_REFS"/*.md )
if [[ ${#ref_files[@]} -eq 0 ]]; then
    fail "no reference files in $SRC_REFS"
else
    pass "found ${#ref_files[@]} reference file(s)"
fi

for src in "$SRC_AGENTS"/*.md; do
    [[ -f "$src" ]] || continue
    name="$(basename "$src" .md)"
    # Extract `agents/references/foo.md` style mentions. We only match the
    # `agents/references/` prefix (not bare `references/`) so paths under
    # `skills/madhyamas/references/` are not falsely flagged.
    for ref in $(grep -oE 'agents/references/[A-Za-z0-9_-]+\.md' "$src" | sort -u); do
        rel="${ref#agents/}"
        target="$AGENTS_DIR/$rel"
        if [[ -f "$target" ]]; then
            pass "$name -> $ref exists"
        else
            fail "$name references $ref but file not found at $target"
        fi
    done
    # Check sibling agent references (agents/agents/foo.md or ./foo.md).
    for ref in $(grep -oE 'agents/agents/[A-Za-z0-9_-]+\.md' "$src" | sort -u); do
        target="$AGENTS_DIR/${ref#agents/}"
        if [[ -f "$target" ]]; then
            pass "$name -> $ref exists"
        else
            fail "$name references $ref but file not found at $target"
        fi
    done
done
echo ""

# 5. Reference files: emoji + line count
echo "5. Reference file checks"
for ref in "$SRC_REFS"/*.md; do
    [[ -f "$ref" ]] || continue
    rname="$(basename "$ref")"
    if has_emoji "$ref"; then
        fail "$rname: contains emoji (not allowed)"
    fi
    lines=$(wc -l < "$ref" | tr -d ' ')
    if [[ "$lines" -gt 1000 ]]; then
        warn "$rname: $lines lines (consider splitting if >1000)"
    else
        pass "$rname: $lines lines"
    fi
done
echo ""

# 6. Scripts executable
echo "6. Script checks"
for s in "$SRC_SCRIPTS"/*.sh; do
    [[ -f "$s" ]] || continue
    if [[ -x "$s" ]]; then
        pass "$(basename "$s") is executable"
    else
        fail "$(basename "$s") is not executable (chmod +x)"
    fi
    if bash -n "$s" 2>/dev/null; then
        pass "$(basename "$s") syntax ok"
    else
        fail "$(basename "$s") has a syntax error"
    fi
done
echo ""

# Summary
echo "------------------------------------------------"
if [[ "$errors" -eq 0 ]]; then
    printf '\033[32mVALID\033[0m — %d error(s), %d warning(s)\n' "$errors" "$warnings"
else
    printf '\033[31mINVALID\033[0m — %d error(s), %d warning(s)\n' "$errors" "$warnings"
    exit 1
fi
