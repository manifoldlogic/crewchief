# Ticket: MCPSTART-4002: Implement automated test cases for provider startup

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- test-runner
- verify-ticket
- commit-ticket

## Summary
Add 5 critical test cases to integration script: Google provider (Ollama NOT running), Default (Ollama running), OpenAI (Ollama NOT running), Explicit Ollama (running), Diagnostics. These tests prove the provider-based startup fix works correctly.

## Background
These are the core acceptance tests that prove the fix from Phases 1-3 works as designed. Tests verify that:
1. Google provider doesn't start Ollama (uses Google's embeddings)
2. Default behavior starts all services including Ollama
3. OpenAI provider doesn't start Ollama (uses OpenAI's embeddings)
4. Explicit EMBEDDING_PROVIDER=ollama DOES start Ollama
5. Diagnostic logs show correct environment variables and commands

Implements test cases from **MCPSTART_QUALITY_STRATEGY.md lines 15-134**.

This is the final validation that the fix solves the original issue: unnecessary Ollama startup when using external embedding providers.

## Acceptance Criteria
- [ ] Test 1: Google provider - Ollama does NOT start, postgres+maproom-mcp DO start
- [ ] Test 2: Default (no provider) - All services start including Ollama
- [ ] Test 3: OpenAI provider - Ollama does NOT start
- [ ] Test 4: Explicit EMBEDDING_PROVIDER=ollama - Ollama DOES start
- [ ] Test 5: Diagnostic logs show correct env vars and commands
- [ ] All tests pass consistently (no flakiness)
- [ ] Each test includes container state verification
- [ ] Tests complete in under 2 minutes total

## Technical Requirements

### Test 1: Google Provider (Ollama Should NOT Start)
```bash
test_google_provider() {
  echo -e "\n[TEST 1] Google Provider - Ollama should NOT start"
  cleanup

  export GOOGLE_API_KEY="test-key-123"
  export EMBEDDING_PROVIDER="google"

  timeout 15 node packages/maproom-mcp/bin/cli.cjs &
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
```

### Test 2: Default Behavior (All Services Including Ollama)
```bash
test_default_all_services() {
  echo -e "\n[TEST 2] Default (no provider) - All services should start"
  cleanup

  # Unset provider env vars
  unset EMBEDDING_PROVIDER
  unset GOOGLE_API_KEY
  unset OPENAI_API_KEY

  timeout 15 node packages/maproom-mcp/bin/cli.cjs &
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
```

### Test 3: OpenAI Provider (Ollama Should NOT Start)
```bash
test_openai_provider() {
  echo -e "\n[TEST 3] OpenAI Provider - Ollama should NOT start"
  cleanup

  export OPENAI_API_KEY="sk-test-key-456"
  export EMBEDDING_PROVIDER="openai"

  timeout 15 node packages/maproom-mcp/bin/cli.cjs &
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
```

### Test 4: Explicit Ollama Provider
```bash
test_explicit_ollama() {
  echo -e "\n[TEST 4] Explicit EMBEDDING_PROVIDER=ollama - Ollama SHOULD start"
  cleanup

  export EMBEDDING_PROVIDER="ollama"

  timeout 15 node packages/maproom-mcp/bin/cli.cjs &
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
```

### Test 5: Diagnostic Logs
```bash
test_diagnostic_logs() {
  echo -e "\n[TEST 5] Diagnostic logs show correct env vars and commands"
  cleanup

  export GOOGLE_API_KEY="test-key-789"
  export EMBEDDING_PROVIDER="google"

  # Capture logs
  LOG_FILE=$(mktemp)
  timeout 15 node packages/maproom-mcp/bin/cli.cjs > "$LOG_FILE" 2>&1 &
  CLI_PID=$!
  sleep 8

  # Check for diagnostic output
  if ! grep -q "EMBEDDING_PROVIDER" "$LOG_FILE"; then
    echo "❌ FAIL: No EMBEDDING_PROVIDER in logs"
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
```

### Integration with Framework
Add function calls after cleanup trap in startup-integration.sh:
```bash
# Run all tests
test_google_provider || true
test_default_all_services || true
test_openai_provider || true
test_explicit_ollama || true
test_diagnostic_logs || true
```

## Implementation Notes

### Test Case Details (from MCPSTART_QUALITY_STRATEGY.md)

**Test 1: Google Provider**
- Lines 19-60 in quality strategy
- Verifies: Ollama absent, postgres + maproom-mcp present
- Key env vars: GOOGLE_API_KEY, EMBEDDING_PROVIDER=google

**Test 2: Default Behavior**
- Lines 62-97 in quality strategy
- Verifies: All 3 services present
- Key env vars: None set (defaults)

**Test 3: OpenAI Provider**
- Lines 99-134 in quality strategy
- Verifies: Ollama absent, core services present
- Key env vars: OPENAI_API_KEY, EMBEDDING_PROVIDER=openai

**Test 4: Explicit Ollama**
- Verifies explicit Ollama request works
- Key env vars: EMBEDDING_PROVIDER=ollama

**Test 5: Diagnostics**
- Verifies logging from Phase 1 tickets
- Checks for env var and docker command output

### Timing Considerations
- Sleep 8 seconds after CLI start (allows Docker Compose to start containers)
- Timeout 15 seconds on CLI (prevents hanging if startup fails)
- 2 second sleep after cleanup (allows Docker to fully stop)

### Error Handling
- Each test returns 1 on failure, increments TESTS_FAILED
- Use `|| true` when calling tests to prevent script exit
- Final summary reports total pass/fail
- Script exits 1 if any test failed

### Debugging Tests
If tests fail:
1. Check Docker is running: `docker ps`
2. Check container logs: `docker logs maproom-SERVICE`
3. Check CLI output: Remove `> /dev/null 2>&1` to see logs
4. Increase sleep times if timing issue suspected
5. Run cleanup manually: `cd ~/.maproom-mcp && docker compose down`

## Dependencies
- **Prerequisite**: MCPSTART-4001 (framework must exist)
- **Blocks**: All other tickets - these tests verify the complete fix

Without this ticket, we cannot verify that:
- MCPSTART-1001, 1002, 1003, 1004 (Phase 1 diagnostic logging) works
- MCPSTART-2001, 2002, 2003 (Phase 2 env passing) works
- MCPSTART-3001, 3002, 3003 (Phase 3 selective startup) works

## Risk Assessment
- **Risk**: Tests might be flaky due to Docker timing
  - **Mitigation**: Use adequate sleep times (8 seconds), can increase if needed
- **Risk**: Tests might pass locally but fail in CI
  - **Mitigation**: Test on clean system, document environment requirements
- **Risk**: Container names might differ across environments
  - **Mitigation**: Use standard Docker Compose naming from maproom-mcp
- **Risk**: Low - tests only verify, don't modify production code
  - **Mitigation**: Tests are isolated in tests/ directory

## Files/Packages Affected
- `packages/maproom-mcp/tests/startup-integration.sh` (modify - add test functions)
