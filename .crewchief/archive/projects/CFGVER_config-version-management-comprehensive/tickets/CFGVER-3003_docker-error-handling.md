# Ticket: CFGVER-3003: Implement comprehensive Docker error handling and recovery

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
Implement comprehensive Docker error detection and handling that provides clear, actionable error messages for common failure scenarios. This ensures users understand what went wrong and how to fix it, enabling graceful degradation when Docker is unavailable.

## Background
Docker operations can fail for many reasons: Docker not installed, daemon not running, permission denied, Docker Compose V2 not available. Each scenario requires different handling and user guidance. Poor error messages lead to user confusion and support burden.

This ticket implements a diagnostic function that detects Docker availability and provides actionable error messages for each failure scenario. This enables the update process to either continue safely (when Docker optional) or fail fast with clear recovery instructions.

## Acceptance Criteria
- [ ] Function `checkDockerAvailable()` detects Docker not installed (ENOENT error)
- [ ] Detects Docker daemon not running (connection refused error)
- [ ] Detects permission errors (need sudo or docker group)
- [ ] Detects Docker Compose V2 not available
- [ ] Provides actionable error messages for each scenario
- [ ] Returns structured result: `{ available: boolean, reason?: string, suggestion?: string }`
- [ ] Result is cached (don't check repeatedly in same update)
- [ ] Update process continues or fails gracefully based on error type

## Technical Requirements
- **Module Location:** `packages/maproom-mcp/src/config-manager.ts`
- **Function Name:** `checkDockerAvailable()`
- **Test Command:** `docker version` (quick, non-destructive)
- **Implementation:**
  - Parse error codes: ENOENT, EACCES, connection errors
  - Parse stderr for error messages
  - Cache result for performance
  - Return structured error information
  - Support optional environment variable: `MAPROOM_SKIP_DOCKER=1` for testing

## Implementation Notes
**Docker Error Detection:**

```javascript
const { execFile } = require('child_process');
const { promisify } = require('util');
const execFileAsync = promisify(execFile);

let dockerAvailabilityCache = null;

async function checkDockerAvailable() {
  // Return cached result if already checked
  if (dockerAvailabilityCache !== null) {
    return dockerAvailabilityCache;
  }

  // Allow skipping Docker checks for testing
  if (process.env.MAPROOM_SKIP_DOCKER === '1') {
    dockerAvailabilityCache = {
      available: false,
      skipped: true,
      reason: 'Docker checks disabled via MAPROOM_SKIP_DOCKER'
    };
    return dockerAvailabilityCache;
  }

  try {
    await execFileAsync('docker', ['version'], {
      timeout: 5000 // 5 second timeout
    });

    dockerAvailabilityCache = { available: true };
    return dockerAvailabilityCache;
  } catch (error) {
    dockerAvailabilityCache = parseDockerError(error);
    return dockerAvailabilityCache;
  }
}

function parseDockerError(error) {
  // Docker not installed
  if (error.code === 'ENOENT') {
    return {
      available: false,
      reason: 'Docker not installed',
      suggestion: 'Install Docker from https://docker.com'
    };
  }

  // Permission denied
  if (error.code === 'EACCES' || error.stderr?.includes('permission denied')) {
    return {
      available: false,
      reason: 'Docker permission denied',
      suggestion: 'Add user to docker group: sudo usermod -aG docker $USER\n' +
                  'Then log out and back in, or run: newgrp docker'
    };
  }

  // Docker daemon not running
  if (error.stderr?.includes('Cannot connect to the Docker daemon')) {
    return {
      available: false,
      reason: 'Docker daemon not running',
      suggestion: 'Start Docker Desktop or run: sudo systemctl start docker'
    };
  }

  // Generic error
  return {
    available: false,
    reason: `Docker error: ${error.message}`,
    suggestion: 'Check Docker installation and try again'
  };
}

async function ensureDockerAvailable() {
  const result = await checkDockerAvailable();

  if (!result.available) {
    const message = `${result.reason}\n${result.suggestion}`;
    throw new Error(message);
  }

  return result;
}
```

**Error Scenarios and Handling:**

1. **Docker Not Installed (ENOENT)**
   - **Detection:** `error.code === 'ENOENT'`
   - **Message:** "Docker not installed. Install from https://docker.com"
   - **Action:** Skip Docker operations with warning (update continues)
   - **Use Case:** CI/CD environments, development without Docker

2. **Docker Daemon Not Running**
   - **Detection:** `stderr.includes('Cannot connect to the Docker daemon')`
   - **Message:** "Docker daemon not running. Start Docker Desktop or run: sudo systemctl start docker"
   - **Action:** Fail update with recovery command
   - **Use Case:** Docker installed but not started

3. **Permission Denied**
   - **Detection:** `error.code === 'EACCES' || stderr.includes('permission denied')`
   - **Message:** "Docker permission denied. Add user to docker group: sudo usermod -aG docker $USER"
   - **Action:** Fail update with fix command
   - **Use Case:** Linux systems where user not in docker group

4. **Docker Compose V2 Not Available**
   - **Detection:** Test `docker compose version` command
   - **Message:** "Docker Compose V2 not available. Update Docker or install separately"
   - **Action:** Fail update with installation instructions
   - **Use Case:** Older Docker versions

**Usage in Update Flow:**

```javascript
async function updateConfigs() {
  const dockerResult = await checkDockerAvailable();

  if (!dockerResult.available) {
    if (dockerResult.skipped) {
      logger.warn('Docker not available, skipping container operations');
      // Continue with config update only
    } else {
      logger.error(dockerResult.reason);
      logger.info('Recovery: ' + dockerResult.suggestion);
      throw new Error('Docker required for update');
    }
  }

  // Docker available, proceed with container operations
  await stopContainers();
  await cleanupOldVolumes();
  // ... rest of update
}
```

**Environment Variable for Testing:**
- `MAPROOM_SKIP_DOCKER=1` - Skip Docker checks, allow testing without Docker
- Use in CI environments where Docker not available
- Still validate config files and update logic

## Dependencies
- CFGVER-3001 (called before container stop)
- CFGVER-3002 (called before volume cleanup)

## Risk Assessment
- **Risk**: Missing error scenarios causing confusing error messages
  - **Mitigation**: Comprehensive error parsing, generic fallback message
  - **Severity**: Low (user sees error but message may be unclear)

- **Risk**: False positives (thinking Docker unavailable when it is)
  - **Mitigation**: Test with `docker version` (reliable check)
  - **Severity**: Medium (unnecessary warning messages)

- **Risk**: Cache prevents recovery after user fixes Docker
  - **Mitigation**: Cache cleared on process restart, acceptable for single update run
  - **Severity**: Low (user can retry update)

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add `checkDockerAvailable()`, `parseDockerError()`, `ensureDockerAvailable()` functions)
- **Execute**: `docker` command (external dependency)

**Environment Detection Note**:
Docker availability checking works the same in both devcontainer and standalone environments. The `docker info` command reliably detects Docker availability in all cases.
