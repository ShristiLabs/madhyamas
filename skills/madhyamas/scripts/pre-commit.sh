#!/usr/bin/env bash
# Pre-commit hook: validate Madhyamas skills before commit
# Install: cp skills/madhyamas/scripts/pre-commit.sh .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit

set -euo pipefail

# Only run if skills directory exists
if [ ! -d "skills/madhyamas" ]; then
    exit 0
fi

# Check if any skills files are staged
STAGED_SKILLS=$(git diff --cached --name-only --diff-filter=ACM -- skills/madhyamas/ 2>/dev/null || true)

if [ -z "$STAGED_SKILLS" ]; then
    exit 0
fi

echo "Validating Madhyamas skills (staged files detected)..."
echo ""

# Run validation
if bash skills/madhyamas/scripts/validate.sh; then
    echo ""
    echo "Skills validation passed."
    exit 0
else
    echo ""
    echo "Skills validation FAILED. Fix errors above before committing."
    echo "To bypass: git commit --no-verify (not recommended)"
    exit 1
fi
