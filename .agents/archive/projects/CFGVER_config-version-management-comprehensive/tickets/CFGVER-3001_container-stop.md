# Ticket: CFGVER-3001: Implement safe Docker container shutdown before updates

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- code-reviewer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement safe Docker container shutdown logic that stops all Maproom containers before config updates without affecting user's other Docker containers. This prevents config file conflicts and ensures containers restart with new configurations.

## Background
Before updating configuration files, we must stop running Maproom containers to prevent conflicts when config files are modified. If containers are running when configs change, they may continue using old configs or crash when files are replaced under them. This could leave the system in an inconsistent state.

This ticket implements the critical first step of Docker integration: safely shutting down only our containers while preserving all other Docker resources the user may have running.

Reference: `architecture.md` lines 208-222 for Docker container cleanup approach.

## Acceptance Criteria
- [ ] Function `stopContainers()` stops all Maproom Docker containers using docker compose
- [ ] Waits for containers to stop completely with 30 second timeout
- [ ] Handles Docker not installed/running gracefully with warning message
- [ ] Does not affect user's other Docker containers (uses specific compose file)
- [ ] Returns success status when containers stop successfully
- [ ] Returns failure status with error details when operation fails
- [ ] Uses `execFile` (not shell exec) to prevent command injection attacks

## Technical Requirements
- **Module Location:** `packages/maproom-mcp/src/config-manager.ts`
- **Function Name:** `stopContainers()`
- **Docker Command:** `docker compose -f ~/.maproom-mcp/docker-compose.yml down`
- **Implementation:**
  - Use Node.js `child_process.execFile` (promisified) NOT `exec` or `execSync`
  - Pass arguments as array (no shell interpolation): `['compose', '-f', COMPOSE_FILE, 'down']`
  - Set working directory to cache directory: `{ cwd: CACHE_DIR }`
  - Set timeout: `{ timeout: 30000 }` (30 seconds)
  - Handle ENOENT error (Docker not found)
  - Handle connection errors (Docker daemon not running)
  - Handle timeout errors (containers stuck)

## Implementation Notes
**Security-Critical Implementation (from `security-review.md` lines 138-167):**

```javascript
const { execFile } = require('child_process');
const { promisify } = require('util');
const execFileAsync = promisify(execFile);

async function stopContainers() {
  const COMPOSE_FILE = path.join(CACHE_DIR, 'docker-compose.yml');

  // Check if compose file exists
  if (!fs.existsSync(COMPOSE_FILE)) {
    logger.warn('Docker compose file not found, skipping container shutdown');
    return { success: true, skipped: true };
  }

  try {
    await execFileAsync('docker', [
      'compose',
      '-f', COMPOSE_FILE, // Passed as argument, NOT interpolated
      'down'
    ], {
      cwd: CACHE_DIR,
      timeout: 30000 // 30 second timeout
    });

    logger.info('Successfully stopped Maproom containers');
    return { success: true };
  } catch (error) {
    if (error.code === 'ENOENT') {
      logger.warn('Docker not available, skipping container shutdown');
      return { success: true, skipped: true };
    }

    if (error.killed) { // Timeout
      logger.warn('Container shutdown timeout, continuing with update (best effort)');
      return { success: false, timeout: true };
    }

    logger.error('Failed to stop containers:', error.message);
    throw new Error(`Failed to stop containers: ${error.message}`);
  }
}
```

**Error Handling Scenarios:**
1. **Docker not found (ENOENT):** Skip with warning, continue update
2. **Timeout:** Log warning, continue with update (best effort)
3. **Permission denied:** Clear error with sudo instructions
4. **Docker daemon not running:** Return error with actionable message

**Return Value:**
```javascript
{
  success: boolean,
  skipped?: boolean,  // Docker not available
  timeout?: boolean,  // Containers didn't stop in time
  error?: string      // Error message if failed
}
```

## Dependencies
- CFGVER-2001 (called before backup in update flow)

## Risk Assessment
- **Risk**: Command injection via shell interpolation
  - **Mitigation**: Use `execFile` with array arguments, NEVER use `exec` or template strings
  - **Severity**: Critical (arbitrary code execution)
  - **Reference**: `security-review.md` lines 138-167

- **Risk**: Affecting user's other Docker containers
  - **Mitigation**: Use specific compose file path, never use bare `docker compose down`
  - **Severity**: High (data loss in user's other projects)

- **Risk**: Containers stuck, preventing update
  - **Mitigation**: 30 second timeout, log warning and continue (best effort)
  - **Severity**: Medium (update continues anyway)

- **Risk**: Docker not installed causing update failure
  - **Mitigation**: Graceful skip with warning message
  - **Severity**: Low (expected scenario in some environments)

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add `stopContainers()` function)
- **Read**: `~/.maproom-mcp/docker-compose.yml` (compose file path)
- **Execute**: `docker` command (external dependency)

**Environment Detection Note**:
Docker container paths differ between environments:
- **Devcontainer**: Containers run in Docker network, accessible via hostname
- **Standalone**: Containers run on host, may need different connection strings
The implementation should detect the environment (check for `/.dockerenv` file or `TERM_PROGRAM=vscode`) and use appropriate paths. However, for this ticket (container stop), the `docker compose down` command works the same in both environments.
