#!/usr/bin/env bash
# Install git hooks for madhyamas development.
# Copies hooks from the hooks/ directory into .git/hooks/ and makes them executable.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOKS_SRC="$REPO_ROOT/hooks"
HOOKS_DST="$REPO_ROOT/.git/hooks"

if [ ! -d "$HOOKS_SRC" ]; then
  echo "install-hooks: no hooks/ directory found at $HOOKS_SRC"
  exit 1
fi

mkdir -p "$HOOKS_DST"

for hook in "$HOOKS_SRC"/*; do
  name=$(basename "$hook")
  if [ "$name" = "install.sh" ]; then
    continue
  fi
  cp "$hook" "$HOOKS_DST/$name"
  chmod +x "$HOOKS_DST/$name"
  echo "install-hooks: installed $name"
done

echo ""
echo "install-hooks: done. Hooks will run automatically on git operations."
echo "  To bypass temporarily:  git commit --no-verify"
