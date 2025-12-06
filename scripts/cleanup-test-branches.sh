#!/bin/bash
# Cleanup orphaned test branches created by integration tests
#
# Test suites may create temporary git branches for worktree-based testing.
# When tests fail or are interrupted, these branches may be left behind.
#
# This script removes common test branch patterns:
# - variant-test-* (from variant-injection tests)
# - cc-*-test-* (from orchestrator/agent spawn tests)
# - cc-*-agent-* (from agent execution tests)

set -e

echo "🧹 Cleaning up orphaned test branches..."

# Count branches before cleanup
VARIANT_COUNT=$(git branch | grep -c "variant-test-" || true)
CC_COUNT=$(git branch | grep -c "^  cc-" || true)
TOTAL=$((VARIANT_COUNT + CC_COUNT))

if [ "$TOTAL" -eq 0 ]; then
  echo "✅ No orphaned test branches found"
  exit 0
fi

echo "Found $TOTAL orphaned test branches:"
echo "  - variant-test-*: $VARIANT_COUNT"
echo "  - cc-*: $CC_COUNT"

# Delete variant-test branches
if [ "$VARIANT_COUNT" -gt 0 ]; then
  echo ""
  echo "Deleting variant-test-* branches..."
  git branch | grep "variant-test-" | xargs -n 1 git branch -D 2>/dev/null || true
fi

# Delete cc- test branches
if [ "$CC_COUNT" -gt 0 ]; then
  echo ""
  echo "Deleting cc-* branches..."
  git branch | grep "^  cc-" | xargs -n 1 git branch -D 2>/dev/null || true
fi

echo ""
echo "✅ Cleanup complete!"
