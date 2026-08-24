#!/usr/bin/env bash
# check-versioned-build.sh — verify the versioned docs build end to end.
#
# Builds SNAPSHOT, a simulated release (v0.9.9), SNAPSHOT again, and a
# backport of v0.9.9 — all into a throwaway staging directory — and asserts
# the multi-version layout, the latest/ alias, the root redirect, and the
# cross-version switcher links. Exits 0 on pass, nonzero on fail.
#
# Used by CI (see .github/workflows/docs.yml) and runnable locally:
#   ./scripts/check-versioned-build.sh   (from docs-site/)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SCRIPT="$SCRIPT_DIR/build-versioned.sh"

TMP_STAGING="$(mktemp -d)"
cleanup() { rm -rf "$TMP_STAGING"; }
trap cleanup EXIT

pass=0
fail=0
ok()   { pass=$((pass + 1)); echo "  ok: $1"; }
bad()  { fail=$((fail + 1)); echo "  FAIL: $1" >&2; }
check() { # check <description> <command...>
  local desc="$1"; shift
  if "$@" >/dev/null 2>&1; then ok "$desc"; else bad "$desc"; fi
}

contains() { # contains <file> <needle> — grep -F
  grep -Fq "$2" "$1"
}

echo "==> [1/4] SNAPSHOT build"
"$SCRIPT" snapshot --staging "$TMP_STAGING" >/dev/null 2>&1
echo "==> [2/4] release v0.9.9 build"
"$SCRIPT" release v0.9.9 --staging "$TMP_STAGING" >/dev/null 2>&1
echo "==> [3/4] SNAPSHOT rebuild (preservation)"
"$SCRIPT" snapshot --staging "$TMP_STAGING" >/dev/null 2>&1

echo "==> Assertions"
check "staging contains SNAPSHOT/"        test -d "$TMP_STAGING/SNAPSHOT"
check "staging contains v0.9.9/"          test -d "$TMP_STAGING/v0.9.9"
check "staging contains latest/"          test -d "$TMP_STAGING/latest"
check "root index.html exists"            test -f "$TMP_STAGING/index.html"
check "latest/ mirrors v0.9.9/"           cmp -s "$TMP_STAGING/latest/index.html" "$TMP_STAGING/v0.9.9/index.html"
check "root redirects to latest/"         contains "$TMP_STAGING/index.html" 'url=https://shristilabs.github.io/madhyamas/latest/'
check "SNAPSHOT has v0.9.9 switcher link" contains "$TMP_STAGING/SNAPSHOT/index.html" 'https://shristilabs.github.io/madhyamas/v0.9.9/'
check "v0.9.9 has SNAPSHOT switcher link" contains "$TMP_STAGING/v0.9.9/index.html" 'https://shristilabs.github.io/madhyamas/SNAPSHOT/'
check "SNAPSHOT built under its base"     contains "$TMP_STAGING/SNAPSHOT/index.html" '/madhyamas/SNAPSHOT/'
check "v0.9.9 built under its base"       contains "$TMP_STAGING/v0.9.9/index.html" '/madhyamas/v0.9.9/'
check "SNAPSHOT labels current version"   contains "$TMP_STAGING/SNAPSHOT/index.html" 'SNAPSHOT (current)'

echo "==> [4/4] backport v0.9.9 rebuild"
"$SCRIPT" backport v0.9.9 --staging "$TMP_STAGING" >/dev/null 2>&1
check "backport keeps SNAPSHOT/"          test -d "$TMP_STAGING/SNAPSHOT"
check "backport keeps latest/ in sync"    cmp -s "$TMP_STAGING/latest/index.html" "$TMP_STAGING/v0.9.9/index.html"

echo
echo "Passed: $pass  Failed: $fail"
if [ "$fail" -ne 0 ]; then
  echo "check-versioned-build: FAIL" >&2
  exit 1
fi
echo "check-versioned-build: PASS"
