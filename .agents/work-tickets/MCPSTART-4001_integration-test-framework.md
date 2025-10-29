# Ticket: MCPSTART-4001: Create integration test script framework

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
Create bash script framework with cleanup, test runner, and reporting for automated integration tests. This framework will run 5+ critical test cases to verify provider-based startup behavior automatically.

## Background
All previous implementation tickets (Phases 1-3) need automated verification to ensure the fix works correctly. This ticket creates the test framework infrastructure that will run multiple test cases automatically, with proper cleanup, timeout management, and clear pass/fail reporting.

Implements testing framework from **MCPSTART_QUALITY_STRATEGY.md lines 136-236**.

This framework is critical because:
- Manual testing is error-prone and time-consuming
- Need consistent verification across different provider configurations
- Must prevent hanging tests and ensure cleanup
- Provides confidence before merging changes

## Acceptance Criteria
- [ ] Create `packages/maproom-mcp/tests/startup-integration.sh` bash script
- [ ] Includes cleanup() function with trap EXIT for guaranteed cleanup
- [ ] Each test has clear name and pass/fail output (✅/❌)
- [ ] Script exits with code 0 on all pass, 1 on any failure
- [ ] Cleanup stops all maproom containers and kills CLI processes
- [ ] Uses timeout command to prevent hanging tests
- [ ] Total runtime under 2 minutes for all tests
- [ ] Script is executable (chmod +x)

## Technical Requirements

### Script Structure
- Bash script with `set -e` (exit on error)
- Shebang: `#!/usr/bin/env bash`
- Set options: `set -euo pipefail` for strict error handling

### Cleanup Function
```bash
cleanup() {
  echo "Cleaning up..."
  cd ~/.maproom-mcp 2>/dev/null && docker compose down 2>/dev/null || true
  pkill -f "maproom-mcp" 2>/dev/null || true
  # Wait for containers to stop
  sleep 2
}
trap cleanup EXIT
```

### Test Template Pattern
Each test should follow this structure:
```bash
echo -e "\n[TEST X] Description"
cleanup
# Setup env vars
export PROVIDER_VAR=value
# Run CLI with timeout
timeout 15 node packages/maproom-mcp/bin/cli.cjs &
CLI_PID=$!
sleep 8  # Allow startup time
# Verify container state
if docker ps --filter "name=maproom-SERVICE" --format "{{.Names}}" | grep -q "maproom-SERVICE"; then
  echo "✅ PASS: Service is running"
else
  echo "❌ FAIL: Service not running"
  exit 1
fi
# Cleanup for next test
cleanup
```

### Container Verification
- Use `docker ps --filter "name=maproom-SERVICE" --format "{{.Names}}"` to check containers
- Services to check: `maproom-postgres`, `maproom-mcp`, `maproom-ollama`
- Use `grep -q` for presence checks, check `$?` for validation

### Output Format
- Clear section headers with test names
- Consistent emoji indicators: ✅ PASS, ❌ FAIL
- Summary at end with total pass/fail count
- Exit code reflects overall result (0=success, 1=failure)

## Implementation Notes

### Full Script Template (from MCPSTART_QUALITY_STRATEGY.md):

```bash
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
```

### Key Considerations
- Script must be idempotent (can run multiple times safely)
- Cleanup must be guaranteed even on script failure
- Timeouts prevent hanging on startup issues
- Sleep times (8 seconds) allow for Docker startup
- Must handle case where ~/.maproom-mcp doesn't exist yet

### Testing the Framework
After creating the script:
1. Make executable: `chmod +x packages/maproom-mcp/tests/startup-integration.sh`
2. Run with no tests: Should show 0 passed, 0 failed, exit 0
3. Test cleanup: Kill script mid-run, verify containers stop
4. Test trap: Send INT signal, verify cleanup runs

## Dependencies
None - this ticket can be completed in parallel with implementation tickets. The framework creates the structure that MCPSTART-4002 will fill with actual test cases.

## Risk Assessment
- **Risk**: Low - tests don't modify production code, only verify behavior
  - **Mitigation**: Framework is isolated in tests/ directory
- **Risk**: Cleanup might not catch all containers if naming differs
  - **Mitigation**: Use broad filters and || true for graceful failure
- **Risk**: Tests might be flaky due to timing
  - **Mitigation**: Use adequate sleep times (8 seconds) and retry logic if needed

## Files/Packages Affected
- `packages/maproom-mcp/tests/startup-integration.sh` (new file - create directory if needed)
