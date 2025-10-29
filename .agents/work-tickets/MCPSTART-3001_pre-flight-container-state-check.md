# Ticket: MCPSTART-3001: Implement pre-flight container state check

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Before starting services, check if containers are already running and optionally stop them to ensure clean state.

## Background
Stale containers from previous runs might still be running with old configurations. This ticket adds pre-flight checking that detects existing containers and stops them before starting new ones, ensuring configuration changes take effect.

This implements **Phase 3.1: Pre-Flight Container Cleanup** from MCPSTART_ARCHITECTURE.md (lines 163-201).

## Acceptance Criteria
- [ ] Function `ensureCleanState()` checks for existing containers using `docker compose ps -q`
- [ ] If containers found, stops all services with `docker compose stop`
- [ ] Logs container states before and after cleanup with descriptive messages
- [ ] Waits briefly (1 second) after stop for complete shutdown
- [ ] Called before starting services in `startDockerCompose()` function
- [ ] Does not fail if no containers are running (graceful handling)

## Technical Requirements
- Use `spawnSync('docker', ['compose', 'ps', '-q'], ...)` to detect running containers
- Parse output to check if any container IDs are returned
- Use `spawnSync('docker', ['compose', 'stop'], ...)` to stop all services
- Add `await new Promise(resolve => setTimeout(resolve, 1000))` for shutdown delay
- Log before cleanup: "Checking for existing containers..."
- Log during cleanup: "Stopping existing containers..."
- Log after cleanup: "Container cleanup complete"
- Function should be async to allow for delay
- Reference: MCPSTART_ARCHITECTURE.md lines 163-201

## Implementation Notes
```javascript
async function ensureCleanState() {
  console.log('\n=== Pre-Flight: Checking for Existing Containers ===');

  // Check if any containers exist
  const psResult = spawnSync('docker', ['compose', 'ps', '-q'], {
    cwd: CONFIG_DIR,
    encoding: 'utf-8',
    stdio: 'pipe'
  });

  const containerIds = psResult.stdout.trim();

  if (containerIds) {
    console.log(`Found existing containers, stopping all services...`);

    // Log current state before stopping
    logDockerState();

    // Stop all services
    const stopResult = spawnSync('docker', ['compose', 'stop'], {
      cwd: CONFIG_DIR,
      encoding: 'utf-8',
      stdio: 'inherit'
    });

    if (stopResult.status !== 0) {
      console.error('Failed to stop existing containers');
      throw new Error('Container cleanup failed');
    }

    // Wait for complete shutdown
    console.log('Waiting for containers to fully stop...');
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Verify cleanup
    logDockerState();
    console.log('Container cleanup complete\n');
  } else {
    console.log('No existing containers found, clean state confirmed\n');
  }
}
```

Call site in `startDockerCompose()`:
```javascript
async function startDockerCompose(options) {
  // ... existing code ...

  // Add pre-flight cleanup
  await ensureCleanState();

  // ... continue with service startup ...
}
```

## Dependencies
- MCPSTART-2001 (Explicit env parameter spawn calls) - Complete
- MCPSTART-2002 (Docker compose verification) - Complete
- MCPSTART-2003 (Provider env validation) - Complete
- Requires Phase 2 to be complete before starting Phase 3

## Risk Assessment
- **Risk**: Stopping containers may interrupt active debugging sessions or other work
  - **Mitigation**: Log clearly what is being stopped; users can skip cleanup with a flag if needed
- **Risk**: 1-second delay may not be sufficient for all containers
  - **Mitigation**: Monitor logs; increase delay if issues persist
- **Risk**: Stop command failure could leave system in inconsistent state
  - **Mitigation**: Error handling with descriptive messages; manual intervention instructions

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - Add `ensureCleanState()` function and call it from `startDockerCompose()`
