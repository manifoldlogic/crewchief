# Ticket: DINDFX-2002: Implement getWorkspaceHostPath() with execFileSync

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (5/5 getWorkspaceHostPath tests passing)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the core discovery function `getWorkspaceHostPath()` that finds the actual host path where `/workspace` is mounted from. This function solves the Docker-in-Docker volume mount problem by inspecting the current container's mounts.

## Background
After implementing Docker detection (DINDFX-2001), we now implement the critical function that discovers where `/workspace` is actually mounted from on the host. This is essential for Docker-in-Docker scenarios where the devcontainer needs to mount volumes using the host's real paths, not the intermediate container paths.

This ticket implements Phase 2 Step 2.2 from the DINDFX project plan. We use `execFileSync()` instead of `execSync()` to prevent shell injection vulnerabilities, following the security requirements from the project's security review.

**References:**
- `.agents/projects/DINDFX_docker-workspace-path-detection/planning/plan.md` - Phase 2 Step 2.2
- `.agents/projects/DINDFX_docker-workspace-path-detection/planning/architecture.md` - Component Design section 2
- `.agents/projects/DINDFX_docker-workspace-path-detection/planning/security-review.md` - execFileSync requirement

## Acceptance Criteria
- [ ] `execFileSync` imported from `child_process` module (security-safe, no shell)
- [ ] `getWorkspaceHostPath()` function added to `packages/maproom-mcp/bin/cli.cjs`
- [ ] Function includes comprehensive JSDoc comments explaining discovery logic
- [ ] Uses array arguments for docker inspect command (not shell string)
- [ ] Timeout configured: 5 seconds for hostname command
- [ ] Timeout configured: 10 seconds for docker inspect command
- [ ] Buffer limit: 1KB (1024 bytes) for hostname output
- [ ] Buffer limit: 10KB (10240 bytes) for docker inspect output
- [ ] Returns host path string when workspace mount is found
- [ ] Returns `null` gracefully when docker inspect fails
- [ ] Returns `null` gracefully when no workspace mount exists
- [ ] Trims whitespace from command output
- [ ] All 5 unit tests for `getWorkspaceHostPath()` pass
- [ ] Verification: `pnpm test getWorkspaceHostPath` shows 5/5 passing tests

## Technical Requirements

### Import Statement
```javascript
const { execFileSync } = require('child_process');
```

### Function Implementation
```javascript
/**
 * Discover the host path for /workspace by inspecting the current container
 *
 * In Docker-in-Docker scenarios, /workspace is mounted from the host, but we need
 * to know the actual host path (not the intermediate container path) to properly
 * mount volumes from spawned containers.
 *
 * This function:
 * 1. Gets our container's hostname
 * 2. Inspects our container's mounts using docker CLI
 * 3. Finds the mount with destination /workspace
 * 4. Returns the source (host) path
 *
 * @returns {string|null} Host path or null if not found/not in container
 */
function getWorkspaceHostPath() {
  try {
    // Get our container hostname (using execFileSync for security)
    const hostname = execFileSync('hostname', [], {
      encoding: 'utf8',
      timeout: 5000,      // 5 second timeout (DoS prevention)
      maxBuffer: 1024     // 1KB max (hostname is short)
    }).trim();

    if (!hostname) {
      return null;
    }

    // Query Docker for mounts of our container (using array args, not shell)
    const hostPath = execFileSync('docker', [
      'inspect',
      hostname,
      '--format',
      '{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}'
    ], {
      encoding: 'utf8',
      timeout: 10000,     // 10 second timeout (DoS prevention)
      maxBuffer: 10240    // 10KB max (docker inspect output can be larger)
    }).trim();

    if (hostPath && hostPath.length > 0) {
      return hostPath;  // Use first mount with /workspace destination
    }

    return null;
  } catch (error) {
    // If docker inspect fails, we might not have docker access
    // or we're not in a devcontainer setup
    return null;
  }
}
```

### Security Requirements
- **CRITICAL**: Use `execFileSync()` not `execSync()` (prevents shell injection)
- Array arguments prevent shell interpolation: `['inspect', hostname, '--format', ...]`
- Timeouts prevent DoS from hanging commands
- Buffer limits prevent memory exhaustion attacks

### Command Details
- **hostname command**: Gets container hostname with 5s timeout, 1KB buffer
- **docker inspect command**: Queries mounts with 10s timeout, 10KB buffer
- **Format string**: `{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}`
  - Iterates through all mounts
  - Finds mount with destination `/workspace`
  - Returns the source (host path)

### Error Handling
- Graceful null returns allow fallback handling upstream
- No error logging (fail silently)
- Docker socket not accessible → return null
- No workspace mount found → return null
- Command timeout → catch error, return null

## Implementation Notes

### Security Considerations
The security review document explicitly requires `execFileSync()` over `execSync()` to prevent shell injection vulnerabilities. By using array arguments for the docker command, we ensure that even if the hostname contains special characters, they cannot be interpreted as shell commands.

### Performance Constraints
- Hostname lookup is fast (milliseconds) - 5s timeout is generous
- Docker inspect can be slower - 10s timeout handles slow daemon responses
- Buffer limits prevent memory exhaustion from malicious/malformed output

### TDD Approach
This ticket follows Test-Driven Development:
1. Tests were written first (DINDFX-1001) and are currently failing
2. This implementation makes those tests pass
3. No test modifications needed - implementation matches test expectations

### Docker Format String
The Go template format string finds the first mount where the destination is `/workspace` and returns its source. This handles cases where:
- Container has multiple mounts
- Only some mounts have `/workspace` as destination
- We need the source (host) path, not the destination (container) path

## Dependencies
- **DINDFX-1001** - Write failing tests (REQUIRED - tests must exist)
- **DINDFX-2001** - Implement isInsideDocker() (RECOMMENDED but not strictly required)

## Risk Assessment

### Risk: Docker socket not accessible
- **Impact**: Function cannot inspect container mounts
- **Mitigation**: Return null, upstream code handles fallback to `/workspace`
- **Likelihood**: Low (devcontainer setup includes docker socket mount)

### Risk: Hostname contains special characters
- **Impact**: Could cause command injection or parsing errors
- **Mitigation**: execFileSync with array args (no shell interpolation)
- **Likelihood**: Very low with proper security implementation

### Risk: Output exceeds buffer limits
- **Impact**: Command throws error, function fails
- **Mitigation**: maxBuffer limits + graceful error handling (return null)
- **Likelihood**: Very low (hostname < 100 bytes, inspect output < 1KB typically)

### Risk: Commands hang indefinitely
- **Impact**: CLI becomes unresponsive
- **Mitigation**: Timeout values (5s for hostname, 10s for docker inspect)
- **Likelihood**: Very low with timeout configuration

### Risk: Tests fail after implementation
- **Impact**: Ticket cannot be completed
- **Mitigation**: Implementation matches test expectations exactly (see quality-strategy.md)
- **Likelihood**: Low (tests designed to match this implementation)

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - Add import and function implementation
