# Ticket: MCPSTART-3003: Add verification of final container state

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- integration-tester
- verify-ticket
- commit-ticket

## Summary
After all startup/cleanup operations complete, verify and log the final container state to confirm expected services are running.

## Background
Even after explicit stop/remove and selective startup, we need to verify the final state matches expectations. This ticket adds final verification that confirms exactly which services are running and alerts if there are discrepancies.

This completes **Phase 3: Clean State Management** from MCPSTART_ARCHITECTURE.md.

## Acceptance Criteria
- [x] After `startDockerCompose()` completes, call `verifyFinalState(expectedServices)`
- [x] Compare running services against expected services (requiredServices)
- [x] Log warning if unexpected services are running
- [x] Log error if expected services are NOT running
- [x] Provides clear diagnostic output for troubleshooting
- [x] Uses JSON format parsing for reliable service detection
- [x] Returns success/failure status for programmatic use

## Technical Requirements
- Use `docker compose ps --format json` for structured output
- Parse JSON to extract service names where `State === 'running'`
- Compare running vs expected sets:
  - Unexpected = running - expected (warning)
  - Missing = expected - running (error)
- Log format:
  - Success: "✅ All expected services running: [list]"
  - Warning: "⚠️  WARNING: Unexpected services running: [list]"
  - Error: "❌ ERROR: Expected services not running: [list]"
- Return boolean: true if state matches expectations, false otherwise

## Implementation Notes
```javascript
function verifyFinalState(expectedServices) {
  console.log('\n=== Verifying Final Container State ===');
  console.log(`Expected services: ${expectedServices.join(', ')}`);

  // Log current state first
  logDockerState();

  // Get running services
  const result = spawnSync('docker', ['compose', 'ps', '--format', 'json'], {
    cwd: CONFIG_DIR,
    encoding: 'utf-8',
    stdio: 'pipe'
  });

  if (result.status !== 0) {
    console.error('❌ ERROR: Failed to verify container state');
    console.error(result.stderr);
    return false;
  }

  // Parse JSON output
  const containers = result.stdout.trim().split('\n')
    .filter(line => line.trim())
    .map(line => {
      try {
        return JSON.parse(line);
      } catch (e) {
        console.error(`Warning: Failed to parse container JSON: ${line}`);
        return null;
      }
    })
    .filter(c => c !== null && c.State === 'running');

  const runningServices = containers.map(c => c.Service);

  console.log(`Running services: ${runningServices.join(', ') || '(none)'}`);

  // Check for unexpected services
  const unexpected = runningServices.filter(s => !expectedServices.includes(s));
  if (unexpected.length > 0) {
    console.error(`⚠️  WARNING: Unexpected services running: ${unexpected.join(', ')}`);
    console.error('These services should have been removed. Manual cleanup may be needed.');
  }

  // Check for missing services
  const missing = expectedServices.filter(s => !runningServices.includes(s));
  if (missing.length > 0) {
    console.error(`❌ ERROR: Expected services not running: ${missing.join(', ')}`);
    console.error('Startup may have failed. Check logs above for errors.');
    return false;
  }

  // Success
  if (unexpected.length === 0) {
    console.log(`✅ All expected services running: ${runningServices.join(', ')}`);
    console.log('Final state verification: PASS\n');
    return true;
  } else {
    console.log('Final state verification: PASS (with warnings)\n');
    return true;
  }
}
```

Call site in `startDockerCompose()`:
```javascript
async function startDockerCompose(options) {
  // ... existing code ...

  await ensureCleanState(); // MCPSTART-3001

  const requiredServices = ['maproom-postgres', 'maproom'];
  if (needsOllama) {
    requiredServices.push('ollama');
  }

  removeUnnecessaryServices(requiredServices); // MCPSTART-3002

  // Start services
  // ... docker compose up code ...

  // Verify final state
  const stateOk = verifyFinalState(requiredServices);
  if (!stateOk) {
    console.error('\n⚠️  Container state verification failed. Check errors above.');
    console.error('You may need to manually stop containers: docker compose stop');
    process.exit(1);
  }

  console.log('\n✅ Docker Compose services started successfully');
}
```

## Dependencies
- MCPSTART-3001 (Pre-flight container state check) - Must be complete
- MCPSTART-3002 (Explicit stop and remove) - Must be complete
- MCPSTART-1003 (Container state verification logging) - Used by this ticket

## Risk Assessment
- **Risk**: Verification may give false positives if container states are in transition
  - **Mitigation**: Run verification after `docker compose up` completes; add small delay if needed
- **Risk**: Unexpected services might be from unrelated docker compose projects
  - **Mitigation**: Uses compose project-specific commands; unlikely to interfere
- **Risk**: Error exit may be too aggressive if warnings are acceptable
  - **Mitigation**: Only exit(1) on missing services; unexpected services are warnings only

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - Add `verifyFinalState()` function and call it from `startDockerCompose()` after service startup

## Implementation Notes

### Changes Made

1. **Added `verifyFinalState()` function** (lines 607-685):
   - Uses `docker compose ps --format json` for structured output
   - Parses JSON to extract service names where `State === 'running'`
   - Compares running vs expected services:
     - Unexpected services: logs warning (services running that shouldn't be)
     - Missing services: logs error and returns false (expected services not running)
   - Returns boolean: `true` if state matches expectations, `false` otherwise
   - Follows exact log format from ticket specification:
     - Success: "✅ All expected services running: [list]"
     - Warning: "⚠️  WARNING: Unexpected services running: [list]"
     - Error: "❌ ERROR: Expected services not running: [list]"

2. **Integrated into `startDockerCompose()`** (lines 785-793):
   - Called after services start successfully (inside compose 'exit' event handler)
   - Passes `requiredServices` as the expected services parameter
   - If verification fails (returns false):
     - Logs error message with troubleshooting guidance
     - Rejects the promise with descriptive error
     - This prevents the startup process from continuing with incomplete services
   - If verification succeeds (returns true):
     - Resolves the promise normally
     - Allows startup to proceed to health checks

### Verification Points

All acceptance criteria have been met:
- ✅ After `startDockerCompose()` completes, calls `verifyFinalState(expectedServices)`
- ✅ Compares running services against expected services (requiredServices)
- ✅ Logs warning if unexpected services are running
- ✅ Logs error if expected services are NOT running
- ✅ Provides clear diagnostic output for troubleshooting
- ✅ Uses JSON format parsing for reliable service detection
- ✅ Returns success/failure status for programmatic use

### Testing Recommendations

The integration test at `packages/maproom-mcp/tests/docker-compose-verification.test.ts` should verify:
1. Successful verification when all expected services are running
2. Warning logged when unexpected services are present (but still returns true)
3. Error logged and false returned when expected services are missing
4. Error rejection in `startDockerCompose()` when verification fails
