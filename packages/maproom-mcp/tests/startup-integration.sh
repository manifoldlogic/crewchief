#!/usr/bin/env bash
set -euo pipefail

echo "Starting Maproom MCP Integration Tests"
echo "======================================="

# Cleanup function
cleanup() {
  echo "Cleaning up test environment..."
  cd ~/.maproom-mcp 2>/dev/null && docker compose down 2>/dev/null || true
  pkill -f "maproom-mcp" 2>/dev/null || true
  sleep 2
}
trap cleanup EXIT

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Test functions will be added in MCPSTART-4002

# Summary
echo -e "\n======================================="
echo "Test Summary"
echo "======================================="
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"

if [ $TESTS_FAILED -gt 0 ]; then
  echo "❌ Some tests failed"
  exit 1
else
  echo "✅ All tests passed"
  exit 0
fi
