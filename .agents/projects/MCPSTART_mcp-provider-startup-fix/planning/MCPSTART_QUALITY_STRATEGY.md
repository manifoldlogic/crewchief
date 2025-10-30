# MCPSTART: MCP Provider Startup Fix - Quality Strategy

## Testing Philosophy

This is an **MVP fix** for a critical user-facing bug. Our testing strategy is pragmatic:

- **Focus on confidence, not coverage**: Tests that prevent backtracking and ensure the fix actually works
- **Integration over unit**: The issue is about system integration (MCP → CLI → Docker), so test the real flow
- **Manual for edge cases**: Exhaustive automated testing of every Docker Compose edge case is overkill
- **User verification**: Real users with real MCP clients are the ultimate test

## Critical Test Cases

These are the **must-pass** scenarios that prove the fix works:

### 1. Google Provider Doesn't Start Ollama
**Why critical**: This is the bug we're fixing

```bash
# Test via environment variables (simulates MCP client)
EMBEDDING_PROVIDER=google \
GOOGLE_PROJECT_ID=test-project \
node packages/maproom-mcp/bin/cli.cjs & sleep 5

# Verify Ollama is NOT running
docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"
if [ $? -eq 0 ]; then
  echo "FAIL: Ollama is running"
  exit 1
else
  echo "PASS: Ollama is not running"
fi

# Verify postgres and maproom-mcp ARE running
docker ps --filter "name=maproom-postgres" --format "{{.Names}}" | grep -q "maproom-postgres" || exit 1
docker ps --filter "name=maproom-mcp" --format "{{.Names}}" | grep -q "maproom-mcp" || exit 1
```

### 2. Default (No Provider) Starts Ollama
**Why critical**: Can't break existing users

```bash
# Test with no EMBEDDING_PROVIDER set
unset EMBEDDING_PROVIDER
node packages/maproom-mcp/bin/cli.cjs & sleep 5

# Verify all services ARE running
docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama" || exit 1
docker ps --filter "name=maproom-postgres" --format "{{.Names}}" | grep -q "maproom-postgres" || exit 1
docker ps --filter "name=maproom-mcp" --format "{{.Names}}" | grep -q "maproom-mcp" || exit 1
```

### 3. OpenAI Provider Doesn't Start Ollama
**Why critical**: Verify the fix works for all non-Ollama providers

```bash
EMBEDDING_PROVIDER=openai \
OPENAI_API_KEY=test-key \
node packages/maproom-mcp/bin/cli.cjs & sleep 5

# Verify Ollama is NOT running
docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"
if [ $? -eq 0 ]; then
  echo "FAIL: Ollama is running"
  exit 1
fi
```

### 4. Explicit Ollama Provider Starts Ollama
**Why critical**: Verify explicit provider selection works

```bash
EMBEDDING_PROVIDER=ollama \
node packages/maproom-mcp/bin/cli.cjs & sleep 5

# Verify Ollama IS running
docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama" || exit 1
```

### 5. Published Package Works
**Why critical**: Previous fixes worked locally but not when published

```bash
# Clean up any existing MCP installation
rm -rf ~/.maproom-mcp

# Install and run via npx (simulates real user experience)
EMBEDDING_PROVIDER=google \
GOOGLE_PROJECT_ID=test-project \
npx -y @crewchief/maproom-mcp@latest & sleep 10

# Verify Ollama is NOT running
docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"
if [ $? -eq 0 ]; then
  echo "FAIL: Ollama is running with published package"
  exit 1
fi
```

## Diagnostic Verification

Before declaring the fix complete, verify diagnostic logging works:

### 6. Diagnostic Logs Show Environment Variables
**Why critical**: Users need to debug their own configuration issues

```bash
# Enable diagnostic mode
MAPROOM_MCP_DEBUG=true \
EMBEDDING_PROVIDER=google \
GOOGLE_PROJECT_ID=test-project \
node packages/maproom-mcp/bin/cli.cjs 2>&1 | tee /tmp/mcp-debug.log

# Verify logs contain expected information
grep -q "EMBEDDING_PROVIDER.*google" /tmp/mcp-debug.log || exit 1
grep -q "GOOGLE_PROJECT_ID.*(set)" /tmp/mcp-debug.log || exit 1
grep -q "Docker Compose Command" /tmp/mcp-debug.log || exit 1
grep -q "Container State" /tmp/mcp-debug.log || exit 1
```

### 7. Diagnostic Logs Show Service Selection
**Why critical**: Verify the logic is working as expected

```bash
EMBEDDING_PROVIDER=google \
node packages/maproom-mcp/bin/cli.cjs 2>&1 | tee /tmp/mcp-output.log

# Should see Google-specific messaging
grep -q "Starting with Google Vertex AI" /tmp/mcp-output.log || exit 1
grep -q "Skipping Ollama" /tmp/mcp-output.log || exit 1

# Should NOT see Ollama messaging
grep -q "Starting with Ollama" /tmp/mcp-output.log && exit 1 || true
```

## Integration Test Script

Create a single bash script that runs all critical tests:

**File**: `packages/maproom-mcp/tests/startup-integration.sh`

```bash
#!/bin/bash
set -e

echo "==================================="
echo "MCP Provider Startup Integration Tests"
echo "==================================="

# Cleanup function
cleanup() {
  echo "Cleaning up containers..."
  cd ~/.maproom-mcp 2>/dev/null && docker compose down 2>/dev/null || true
  pkill -f "maproom-mcp" 2>/dev/null || true
}
trap cleanup EXIT

# Test 1: Google Provider
echo -e "\n[TEST 1] Google Provider - Ollama should NOT start"
cleanup
EMBEDDING_PROVIDER=google \
GOOGLE_PROJECT_ID=test-project \
timeout 15 node packages/maproom-mcp/bin/cli.cjs & sleep 8

if docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"; then
  echo "❌ FAIL: Ollama is running"
  docker ps
  exit 1
else
  echo "✅ PASS: Ollama not running"
fi

# Test 2: Default (no provider)
echo -e "\n[TEST 2] Default Config - Ollama SHOULD start"
cleanup
unset EMBEDDING_PROVIDER
timeout 15 node packages/maproom-mcp/bin/cli.cjs & sleep 8

if docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"; then
  echo "✅ PASS: Ollama is running"
else
  echo "❌ FAIL: Ollama not running"
  docker ps
  exit 1
fi

# Test 3: OpenAI Provider
echo -e "\n[TEST 3] OpenAI Provider - Ollama should NOT start"
cleanup
EMBEDDING_PROVIDER=openai \
OPENAI_API_KEY=test-key \
timeout 15 node packages/maproom-mcp/bin/cli.cjs & sleep 8

if docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"; then
  echo "❌ FAIL: Ollama is running"
  docker ps
  exit 1
else
  echo "✅ PASS: Ollama not running"
fi

# Test 4: Explicit Ollama
echo -e "\n[TEST 4] Explicit Ollama - Ollama SHOULD start"
cleanup
EMBEDDING_PROVIDER=ollama \
timeout 15 node packages/maproom-mcp/bin/cli.cjs & sleep 8

if docker ps --filter "name=maproom-ollama" --format "{{.Names}}" | grep -q "maproom-ollama"; then
  echo "✅ PASS: Ollama is running"
else
  echo "❌ FAIL: Ollama not running"
  docker ps
  exit 1
fi

# Test 5: Diagnostics
echo -e "\n[TEST 5] Diagnostic Logging"
cleanup
MAPROOM_MCP_DEBUG=true \
EMBEDDING_PROVIDER=google \
GOOGLE_PROJECT_ID=test-project \
timeout 15 node packages/maproom-mcp/bin/cli.cjs 2>&1 | tee /tmp/mcp-debug.log & sleep 8

if grep -q "EMBEDDING_PROVIDER.*google" /tmp/mcp-debug.log && \
   grep -q "Docker Compose Command" /tmp/mcp-debug.log; then
  echo "✅ PASS: Diagnostic logs present"
else
  echo "❌ FAIL: Diagnostic logs missing"
  cat /tmp/mcp-debug.log
  exit 1
fi

echo -e "\n==================================="
echo "✅ ALL TESTS PASSED"
echo "==================================="
```

**Usage**:
```bash
# Run all integration tests
cd /workspace
bash packages/maproom-mcp/tests/startup-integration.sh
```

## Manual Testing Checklist

These scenarios are tested manually because automating them requires complex MCP client setup:

### With Real MCP Clients

- [ ] **Claude Desktop** with `.mcp.json` config for Google
  - Verify Ollama doesn't start
  - Verify search works with Google embeddings

- [ ] **Cursor** with `.cursorrules` MCP config for OpenAI
  - Verify Ollama doesn't start
  - Verify search works with OpenAI embeddings

- [ ] **Claude Desktop** with no explicit config (default)
  - Verify Ollama starts
  - Verify search works with Ollama embeddings

### Edge Cases

- [ ] **Config file auto-update**
  - Old docker-compose.yml with hardcoded provider
  - Run CLI, verify config is updated
  - Verify backup file created

- [ ] **Stale containers**
  - Start with Ollama, stop CLI
  - Restart with Google provider
  - Verify Ollama is stopped and removed

- [ ] **Network issues**
  - Kill Docker daemon mid-startup
  - Verify clear error messages

- [ ] **Missing credentials**
  - Set EMBEDDING_PROVIDER=google without GOOGLE_PROJECT_ID
  - Verify clear error message (not silent failure)

## Test Execution Schedule

**Before Each Code Change**:
- Run integration test script
- Must pass all 5 automated tests

**Before Publishing**:
- Run integration test script
- Test published package (Test 5)
- Manual test with at least one real MCP client

**After Publishing**:
- User acceptance testing
- Monitor for issue reports

## Success Criteria

This fix is complete and correct when:

1. ✅ **Automated tests pass**: All 5 integration tests succeed
2. ✅ **Manual MCP client test passes**: At least one real client (Claude Desktop) works correctly
3. ✅ **Published package works**: `npx` installation behaves correctly
4. ✅ **No regressions**: Default Ollama behavior still works
5. ✅ **Diagnostics verify root cause**: Logs show env vars are present and correct commands are executed

## What We're NOT Testing

Being pragmatic, we're skipping these because they're low value for this MVP:

- ❌ **Every Docker Compose version**: We'll test with current stable, document requirements
- ❌ **Every MCP client**: Test with one (Claude Desktop), document for others
- ❌ **Network failure recovery**: Docker Compose handles this, not our concern
- ❌ **Concurrent CLI invocations**: Single-user tool, edge case
- ❌ **Custom docker-compose.yml modifications**: User responsibility, we provide backup
- ❌ **Performance testing**: Not relevant to this bug fix
- ❌ **Unit tests for every function**: Integration tests cover the real issue

## Quality Gates

Before marking any ticket complete:

1. **Agent runs integration test script** - Must pass all 5 tests
2. **Agent verifies diagnostic logs** - Must show expected behavior
3. **Agent confirms published package** - Must work via npx
4. **Human manual verification** - At least one real MCP client test

Before publishing to npm:

1. **All integration tests pass** on clean install
2. **Manual test with real MCP client** succeeds
3. **Version number bumped** appropriately
4. **Changelog updated** with fix details

## Debugging Failed Tests

If tests fail, check in this order:

1. **Are env vars being received?**
   - Check diagnostic logs for `EMBEDDING_PROVIDER`
   - If not present: MCP client issue or test setup issue

2. **Are correct services being selected?**
   - Check diagnostic logs for "Required services"
   - If wrong services: Logic bug in `getRequiredServices()`

3. **Is Docker Compose executing correctly?**
   - Check diagnostic logs for "Docker Compose Command"
   - Run the exact command manually to verify behavior

4. **Are containers in expected state?**
   - Check diagnostic logs for "Container State"
   - Run `docker ps` manually to verify

5. **Is this a published vs local issue?**
   - Test with local `node bin/cli.cjs`
   - Test with `npx @crewchief/maproom-mcp@latest`
   - Compare behavior and logs

## Continuous Validation

Add a GitHub Action that runs the integration test on every commit:

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      docker:
        image: docker:dind

    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: |
          cd packages/maproom-mcp
          npm install

      - name: Run integration tests
        run: |
          cd packages/maproom-mcp
          bash tests/startup-integration.sh
```

## Risk Assessment

**Low Risk Changes**:
- Adding diagnostic logging (no behavior change)
- Explicit env passing (defensive, preserves existing behavior)

**Medium Risk Changes**:
- Docker Compose stop/remove logic (could affect running containers)
- Auto-update of config files (could overwrite user customizations)

**High Risk Changes**:
- Changing default behavior (must preserve zero-config Ollama)
- Service profile migration (Docker Compose version compatibility)

## Rollback Plan

If the fix causes issues:

1. **Immediate**: Revert published package version
2. **Diagnose**: Check diagnostic logs from user reports
3. **Fix forward**: Apply targeted fix based on diagnostics
4. **Re-test**: Run full test suite before republishing

## Documentation

Update package README with troubleshooting section:

```markdown
## Troubleshooting

### Ollama starts when using Google/OpenAI

Enable diagnostic mode to see what's happening:

```bash
MAPROOM_MCP_DEBUG=true npx @crewchief/maproom-mcp
```

Check the output for:
- `EMBEDDING_PROVIDER` value (should be 'google' or 'openai')
- "Required services" (should NOT include 'ollama')
- "Container State" (ollama should not appear)

If `EMBEDDING_PROVIDER` is not set or is 'ollama', your MCP client
is not passing environment variables correctly. Check your `.mcp.json`.
```

This makes users self-sufficient and reduces support burden.
