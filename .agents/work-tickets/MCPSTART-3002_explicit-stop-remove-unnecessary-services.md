# Ticket: MCPSTART-3002: Add explicit stop and remove for unnecessary services

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
When a service should NOT be running (e.g., Ollama with Google provider), explicitly stop AND remove it, not just skip starting it.

## Background
Current code skips starting Ollama by not including it in service args. But if Ollama is already running from a previous session, it continues running. This ticket adds explicit stop + remove for services that shouldn't run based on the current provider configuration.

This implements **Phase 3.2: Explicit Service Removal** from MCPSTART_ARCHITECTURE.md (lines 203-240).

## Acceptance Criteria
- [x] Function `removeUnnecessaryServices(services[])` stops and removes containers that should not run
- [x] Uses `docker compose stop <service>` followed by `docker compose rm -f <service>` for each unnecessary service
- [x] Determines unnecessary services by comparing all_services - required_services
- [x] Logs which services are being removed with clear messaging
- [x] Called after determining required services, before starting services
- [x] Verifies removal with `logDockerState()` call
- [x] Handles case where services don't exist (graceful no-op)

## Technical Requirements
- Determine all possible services: `['maproom-postgres', 'maproom', 'ollama']`
- Required services passed as parameter (from provider config logic)
- Unnecessary services = all_services.filter(s => !required_services.includes(s))
- For each unnecessary service:
  - `docker compose stop <service>`
  - `docker compose rm -f <service>`
- Log format: "Removing unnecessary service: <service>"
- Skip if service doesn't exist (check exit code)
- Reference: MCPSTART_ARCHITECTURE.md lines 203-240

## Implementation Notes
```javascript
function removeUnnecessaryServices(requiredServices) {
  const ALL_SERVICES = ['maproom-postgres', 'maproom', 'ollama'];
  const unnecessaryServices = ALL_SERVICES.filter(s => !requiredServices.includes(s));

  if (unnecessaryServices.length === 0) {
    console.log('No unnecessary services to remove\n');
    return;
  }

  console.log('\n=== Removing Unnecessary Services ===');
  console.log(`Required services: ${requiredServices.join(', ')}`);
  console.log(`Removing: ${unnecessaryServices.join(', ')}\n`);

  for (const service of unnecessaryServices) {
    console.log(`Stopping ${service}...`);
    const stopResult = spawnSync('docker', ['compose', 'stop', service], {
      cwd: CONFIG_DIR,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    // Stopping a non-existent service is not an error
    if (stopResult.status !== 0 && !stopResult.stderr.includes('no such service')) {
      console.error(`Warning: Failed to stop ${service}`);
    }

    console.log(`Removing ${service}...`);
    const rmResult = spawnSync('docker', ['compose', 'rm', '-f', service], {
      cwd: CONFIG_DIR,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    if (rmResult.status !== 0 && !rmResult.stderr.includes('no such service')) {
      console.error(`Warning: Failed to remove ${service}`);
    }
  }

  // Verify removal
  console.log('\nVerifying removal:');
  logDockerState();
  console.log('Service removal complete\n');
}
```

Call site in `startDockerCompose()`:
```javascript
async function startDockerCompose(options) {
  // ... existing code ...

  await ensureCleanState(); // From MCPSTART-3001

  // Determine required services based on provider
  const requiredServices = ['maproom-postgres', 'maproom'];
  if (needsOllama) {
    requiredServices.push('ollama');
  }

  // Remove services that shouldn't be running
  removeUnnecessaryServices(requiredServices);

  // ... continue with service startup ...
}
```

## Dependencies
- MCPSTART-3001 (Pre-flight container state check) - Must be complete
- Builds on Phase 2's provider detection logic

## Risk Assessment
- **Risk**: `docker compose rm -f` is destructive and removes containers permanently
  - **Mitigation**: Only called on services that shouldn't run; user can rebuild if needed
- **Risk**: Service might have data that needs preservation
  - **Mitigation**: Only applies to services like Ollama; database (postgres) won't be in unnecessary list
- **Risk**: Race condition if containers are starting while being removed
  - **Mitigation**: Called after `ensureCleanState()` stops everything first

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - Add `removeUnnecessaryServices()` function and call it from `startDockerCompose()`

## Implementation Notes (Completed)

### Changes Made

1. **Added `removeUnnecessaryServices()` function** (lines 522-605)
   - Defines `ALL_SERVICES = ['postgres', 'ollama', 'maproom-mcp']`
   - Calculates unnecessary services by filtering out required services
   - For each unnecessary service:
     - Stops with `docker compose stop <service>`
     - Removes with `docker compose rm -f <service>`
   - Logs clear messaging: "Removing unnecessary service: <service>"
   - Handles non-existent services gracefully (checks for 'no such service' error)
   - Calls `logDockerState()` for verification after removal
   - Includes diagnostic logging for debugging

2. **Integrated into `startDockerCompose()`** (lines 610-622)
   - Called after `ensureCleanState()` from MCPSTART-3001
   - Called after determining required services with `getRequiredServices()`
   - Called before starting services with `docker compose up -d`
   - Replaced old logic that only stopped services (didn't remove)

### Testing Recommendations

To verify the implementation works correctly:

1. **Test with Google provider** (Ollama should be stopped AND removed):
   ```bash
   EMBEDDING_PROVIDER=google npx @crewchief/maproom-mcp
   # Should see: "Removing: ollama"
   # Verify: docker compose ps should NOT show ollama container
   ```

2. **Test with Ollama provider** (No services should be removed):
   ```bash
   EMBEDDING_PROVIDER=ollama npx @crewchief/maproom-mcp
   # Should see: "No unnecessary services to remove"
   # Verify: All three services running (postgres, ollama, maproom-mcp)
   ```

3. **Test switching providers** (Old provider's containers should be removed):
   ```bash
   # First run with Ollama
   EMBEDDING_PROVIDER=ollama npx @crewchief/maproom-mcp
   # Stop the MCP server (Ctrl+C)
   # Run with Google - Ollama should be stopped AND removed
   EMBEDDING_PROVIDER=google npx @crewchief/maproom-mcp
   # Verify: docker compose ps should NOT show ollama container at all
   ```

### Key Differences from Previous Behavior

**Before**: Old code only stopped unnecessary services (lines 534-572 in original)
- Used `docker compose stop` but NOT `docker compose rm`
- Containers remained (stopped state) and could restart with old config

**After**: New code stops AND removes unnecessary services
- Uses both `docker compose stop` followed by `docker compose rm -f`
- Containers are completely removed, preventing stale configuration issues
- Clean slate for next run with different provider
