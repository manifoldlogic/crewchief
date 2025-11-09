#!/usr/bin/env bash
set -euo pipefail

# Get the workspace root directory (3 levels up from this script)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
CLI_PATH="$WORKSPACE_ROOT/packages/maproom-mcp/bin/cli.cjs"
CONFIG_DIR="$WORKSPACE_ROOT/packages/maproom-mcp/config"

echo "Starting Maproom MCP Integration Tests"
echo "======================================="
echo "Workspace root: $WORKSPACE_ROOT"
echo "CLI path: $CLI_PATH"
echo "Config dir: $CONFIG_DIR"
echo ""

# ========================================
# Docker Compose Configuration Selection
# ========================================
# Determine which docker-compose configuration to use based on TEST_BUILD_FROM_SOURCE.
#
# TEST_BUILD_FROM_SOURCE controls whether to build from source or pull from Docker Hub:
#   - true (default): Use docker-compose.test.yml to build from source
#     * Best for: Integration tests during development, CI/CD, before images published
#     * Build time: ~2-5 minutes on first build (cached thereafter)
#     * Image tag: maproom-mcp:test
#
#   - false: Use docker-compose.yml only to pull from Docker Hub
#     * Best for: Testing published images, production-like testing
#     * Requires: Images already published to crewchief/maproom-mcp
#     * Startup: Fast (no build), just image pull
#
# The test configuration (docker-compose.test.yml) overrides only the maproom-mcp service
# to build from source. All other configuration (environment, volumes, networks, health
# checks) is inherited from the base docker-compose.yml.
#
if [ "${TEST_BUILD_FROM_SOURCE:-true}" = "true" ]; then
  COMPOSE_FILES="-f docker-compose.yml -f docker-compose.test.yml"
  echo "🔧 Using TEST configuration (building from source)"
  echo "   - Base config: docker-compose.yml"
  echo "   - Override: docker-compose.test.yml"
  echo "   - Image tag: maproom-mcp:test"
  echo "   - Build time: ~2-5 minutes (first build)"
else
  COMPOSE_FILES="-f docker-compose.yml"
  echo "📦 Using PRODUCTION configuration (pulling from Docker Hub)"
  echo "   - Config: docker-compose.yml only"
  echo "   - Image: crewchief/maproom-mcp:\${MAPROOM_VERSION:-latest}"
  echo "   - Requires: Pre-published images"
fi
echo ""

# Cleanup function
cleanup() {
  echo "Cleaning up test environment..."
  # Use the same COMPOSE_FILES configuration for cleanup
  cd "$CONFIG_DIR" 2>/dev/null && docker compose $COMPOSE_FILES down 2>/dev/null || true
  pkill -f "maproom-mcp" 2>/dev/null || true
  sleep 2
}
trap cleanup EXIT

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Test 1: Google Provider - Ollama should NOT start
test_google_provider() {
  echo -e "\n[TEST 1] Google Provider - Ollama should NOT start"
  cleanup

  export GOOGLE_API_KEY="test-key-123"
  export MAPROOM_EMBEDDING_PROVIDER="google"

  timeout 15 node "$CLI_PATH" &
  CLI_PID=$!
  sleep 8

  # Verify Ollama is NOT running
  if docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"; then
    echo "❌ FAIL: Ollama is running (should not be)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    return 1
  else
    echo "✅ PASS: Ollama not running"
  fi

  # Verify postgres and maproom-mcp ARE running
  if ! docker ps --filter "name=maproom-postgres" --format "{{.Names}}" | grep -q "maproom-postgres"; then
    echo "❌ FAIL: Postgres not running"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    return 1
  fi

  if ! docker ps --filter "name=maproom-mcp" --format "{{.Names}}" | grep -q "maproom-mcp"; then
    echo "❌ FAIL: maproom-mcp not running"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    return 1
  fi

  echo "✅ PASS: Postgres and maproom-mcp running"
  TESTS_PASSED=$((TESTS_PASSED + 1))
  cleanup
}

# Test 2: Default (no provider) - All services should start
test_default_all_services() {
  echo -e "\n[TEST 2] Default (no provider) - All services should start"
  cleanup

  # Unset provider env vars
  unset MAPROOM_EMBEDDING_PROVIDER
  unset GOOGLE_API_KEY
  unset OPENAI_API_KEY

  timeout 15 node "$CLI_PATH" &
  CLI_PID=$!
  sleep 8

  # Verify all three services are running
  SERVICES=("maproom-postgres" "maproom-mcp" "maproom-ollama")
  for SERVICE in "${SERVICES[@]}"; do
    if ! docker ps --filter "name=$SERVICE" --format "{{.Names}}" | grep -q "$SERVICE"; then
      echo "❌ FAIL: $SERVICE not running"
      TESTS_FAILED=$((TESTS_FAILED + 1))
      return 1
    fi
  done

  echo "✅ PASS: All services running (postgres, mcp, ollama)"
  TESTS_PASSED=$((TESTS_PASSED + 1))
  cleanup
}

# Test 3: OpenAI Provider - Ollama should NOT start
test_openai_provider() {
  echo -e "\n[TEST 3] OpenAI Provider - Ollama should NOT start"
  cleanup

  export OPENAI_API_KEY="sk-test-key-456"
  export MAPROOM_EMBEDDING_PROVIDER="openai"

  timeout 15 node "$CLI_PATH" &
  CLI_PID=$!
  sleep 8

  # Verify Ollama is NOT running
  if docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"; then
    echo "❌ FAIL: Ollama is running (should not be)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    return 1
  else
    echo "✅ PASS: Ollama not running"
  fi

  # Verify postgres and maproom-mcp ARE running
  if ! docker ps --filter "name=maproom-postgres" --format "{{.Names}}" | grep -q "maproom-postgres"; then
    echo "❌ FAIL: Postgres not running"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    return 1
  fi

  echo "✅ PASS: Core services running without Ollama"
  TESTS_PASSED=$((TESTS_PASSED + 1))
  cleanup
}

# Test 4: Explicit MAPROOM_EMBEDDING_PROVIDER=ollama - Ollama SHOULD start
test_explicit_ollama() {
  echo -e "\n[TEST 4] Explicit MAPROOM_EMBEDDING_PROVIDER=ollama - Ollama SHOULD start"
  cleanup

  export MAPROOM_EMBEDDING_PROVIDER="ollama"

  timeout 15 node "$CLI_PATH" &
  CLI_PID=$!
  sleep 8

  # Verify Ollama IS running
  if ! docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"; then
    echo "❌ FAIL: Ollama not running (should be)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    return 1
  fi

  echo "✅ PASS: Ollama started correctly"
  TESTS_PASSED=$((TESTS_PASSED + 1))
  cleanup
}

# Test 5: Diagnostic logs show correct env vars and commands
test_diagnostic_logs() {
  echo -e "\n[TEST 5] Diagnostic logs show correct env vars and commands"
  cleanup

  export GOOGLE_API_KEY="test-key-789"
  export MAPROOM_EMBEDDING_PROVIDER="google"

  # Capture logs
  LOG_FILE=$(mktemp)
  timeout 15 node packages/maproom-mcp/bin/cli.cjs > "$LOG_FILE" 2>&1 &
  CLI_PID=$!
  sleep 8

  # Check for diagnostic output
  if ! grep -q "MAPROOM_EMBEDDING_PROVIDER" "$LOG_FILE"; then
    echo "❌ FAIL: No MAPROOM_EMBEDDING_PROVIDER in logs"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    rm "$LOG_FILE"
    return 1
  fi

  if ! grep -q "google" "$LOG_FILE"; then
    echo "❌ FAIL: Provider value not in logs"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    rm "$LOG_FILE"
    return 1
  fi

  if ! grep -q "docker compose" "$LOG_FILE"; then
    echo "❌ FAIL: Docker command not in logs"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    rm "$LOG_FILE"
    return 1
  fi

  echo "✅ PASS: Diagnostic logs present and correct"
  TESTS_PASSED=$((TESTS_PASSED + 1))
  rm "$LOG_FILE"
  cleanup
}

# Run all tests
test_google_provider || true
test_default_all_services || true
test_openai_provider || true
test_explicit_ollama || true
test_diagnostic_logs || true

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
