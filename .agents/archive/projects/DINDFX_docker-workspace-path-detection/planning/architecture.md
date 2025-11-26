# Architecture: Docker-in-Docker Workspace Path Detection

## Design Philosophy

**MVP Focus:** Add minimal code to solve the specific problem. No over-engineering.

**Test-Driven:** Write the failing test first to prove we understand the problem, then implement the minimal fix to make it pass.

**Zero Configuration:** The user should not need to set environment variables or configure paths manually.

## Architecture Overview

### Current Flow (Broken)

```
User runs: npx @crewchief/maproom-mcp setup --provider=openai
    ↓
bin/cli.cjs parseArgs()
    ↓
runSetup()
    ├─ validateProviderRequirements()
    ├─ checkDockerDaemon()
    ├─ checkDockerCompose()
    ├─ setupConfigDirectory()
    └─ startDockerCompose()
        ├─ env = { ...process.env }
        │   └─ WORKSPACE_HOST_PATH not in process.env ❌
        └─ spawn('docker', ['compose', 'up', '-d'], { env })
            └─ docker-compose.yml expands ${WORKSPACE_HOST_PATH:-/workspace}
                └─ Defaults to /workspace (doesn't exist on host) ❌
```

### Fixed Flow (Solution)

```
User runs: npx @crewchief/maproom-mcp setup --provider=openai
    ↓
bin/cli.cjs parseArgs()
    ↓
runSetup()
    ├─ validateProviderRequirements()
    ├─ checkDockerDaemon()
    ├─ checkDockerCompose()
    ├─ setupConfigDirectory()
    ├─ 🆕 detectAndSetWorkspacePath()  ← NEW FUNCTION
    │   ├─ isInsideDocker() ?
    │   │   ├─ Yes → getWorkspaceHostPath()
    │   │   │   └─ docker inspect $(hostname) → /host_mnt/Users/.../project
    │   │   └─ No → process.cwd()
    │   └─ process.env.WORKSPACE_HOST_PATH = result ✅
    └─ startDockerCompose()
        ├─ env = { ...process.env }
        │   └─ WORKSPACE_HOST_PATH in process.env ✅
        └─ spawn('docker', ['compose', 'up', '-d'], { env })
            └─ docker-compose.yml expands ${WORKSPACE_HOST_PATH}
                └─ Uses actual host path ✅
```

## Component Design

### 1. Detection Function

**Purpose:** Determine if code is running inside a Docker container

**Location:** `packages/maproom-mcp/bin/cli.cjs` (new function)

**Signature:**
```javascript
/**
 * Check if currently running inside a Docker container
 * @returns {boolean} True if inside Docker, false otherwise
 */
function isInsideDocker() {
  // Check for /.dockerenv (most reliable)
  if (fs.existsSync('/.dockerenv')) {
    return true;
  }

  // Check for /run/.containerenv (Podman compatibility)
  if (fs.existsSync('/run/.containerenv')) {
    return true;
  }

  // Fallback: check cgroup
  try {
    const cgroup = fs.readFileSync('/proc/1/cgroup', 'utf8');
    if (cgroup.includes('docker') || cgroup.includes('containerd')) {
      return true;
    }
  } catch (error) {
    // If /proc/1/cgroup doesn't exist, we're probably not in Linux
    return false;
  }

  return false;
}
```

**Rationale:**
- `/.dockerenv`: Standard Docker marker file
- `/run/.containerenv`: Podman compatibility
- `/proc/1/cgroup`: Fallback for containers without marker files
- Graceful failures: Returns false if detection fails

### 2. Host Path Discovery Function

**Purpose:** Find the actual host path that `/workspace` is mounted from

**Location:** `packages/maproom-mcp/bin/cli.cjs` (new function)

**Import Required:**
```javascript
const { execFileSync } = require('child_process');
```

**Signature:**
```javascript
/**
 * Discover the host path for /workspace by inspecting the current container
 * @returns {string|null} Host path or null if not found
 */
function getWorkspaceHostPath() {
  try {
    // Get our container hostname (using execFileSync for security)
    const hostname = execFileSync('hostname', [], {
      encoding: 'utf8',
      timeout: 5000,      // 5 second timeout
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
      timeout: 10000,     // 10 second timeout
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

**Rationale:**
- Uses `execFileSync()` instead of `execSync()` to prevent shell injection
- Uses container's own hostname to inspect itself
- Filters mounts to find `/workspace` destination
- Returns source path (host path)
- Graceful error handling: Returns null on failure

### 3. Workspace Path Resolution Function

**Purpose:** Determine correct workspace path for the current environment

**Location:** `packages/maproom-mcp/bin/cli.cjs` (new function)

**Dependencies:** Uses existing `diagnosticLog()` function (lines 95-102 of bin/cli.cjs)

**Signature:**
```javascript
/**
 * Resolve the appropriate workspace path for the current environment
 * Handles devcontainer (Docker-in-Docker), host, and custom override scenarios
 * @returns {string} Workspace path to use for volume mounting
 */
function resolveWorkspacePath() {
  // Priority 1: User override (for custom setups)
  if (process.env.WORKSPACE_HOST_PATH) {
    diagnosticLog('Using user-provided WORKSPACE_HOST_PATH', {
      path: process.env.WORKSPACE_HOST_PATH
    });
    return process.env.WORKSPACE_HOST_PATH;
  }

  // Priority 2: Docker-in-Docker detection
  if (isInsideDocker()) {
    diagnosticLog('Detected running inside Docker container');

    const hostPath = getWorkspaceHostPath();

    if (hostPath) {
      diagnosticLog('Discovered host workspace path', {
        hostPath,
        source: 'docker inspect'
      });
      return hostPath;
    }

    // Inside Docker but couldn't find mount - warn and use /workspace
    console.warn('⚠️  Running inside Docker but could not discover workspace host path.');
    console.warn('    Volume mount may fail. Set WORKSPACE_HOST_PATH manually if needed.');
    return '/workspace';
  }

  // Priority 3: Running on host - use current directory
  const hostPath = process.cwd();
  diagnosticLog('Running on host, using current directory', { hostPath });
  return hostPath;
}
```

**Rationale:**
- Three-tier priority system
- User override allows manual configuration if auto-detection fails
- Auto-detection for devcontainer
- Fallback to current directory for host execution
- Uses existing `diagnosticLog()` function (inherits redaction & conditional behavior)
- Diagnostic logs only appear when DIAGNOSTIC_MODE is enabled or provider not set
- Clear diagnostics for debugging

### 4. Integration Point

**Purpose:** Call resolution before docker compose starts

**Location:** `packages/maproom-mcp/bin/cli.cjs` in `runSetup()` function

**Insertion point:** Line ~1788, **before** `await startDockerCompose();`

```javascript
async function runSetup() {
  // ... existing code ...

  // Copy configs
  setupConfigDirectory();

  // 🆕 NEW: Detect and set workspace path for Docker volume mounting
  const workspacePath = resolveWorkspacePath();
  process.env.WORKSPACE_HOST_PATH = workspacePath;

  console.error('✓ Workspace path:', workspacePath);

  // Start Docker Compose (respects WORKSPACE_HOST_PATH)
  await startDockerCompose();

  // ... rest of setup ...
}
```

**Rationale:**
- Runs after config directory setup (docker-compose.yml is in place)
- Runs before docker compose (sets environment variable)
- Sets process.env.WORKSPACE_HOST_PATH for spreading to spawn env
- User feedback via console.error (doesn't pollute stdout)

## Data Flow

### Environment Variable Propagation

```
resolveWorkspacePath()
    ├─ Returns: "/host_mnt/Users/user/project"
    └─ Sets: process.env.WORKSPACE_HOST_PATH = "/host_mnt/Users/user/project"
        ↓
runSetup() → startDockerCompose()
    ├─ env = { ...process.env }
    │   └─ Includes: WORKSPACE_HOST_PATH: "/host_mnt/Users/user/project"
    └─ spawn('docker', ['compose', 'up', '-d'], { env })
        ↓
docker compose reads docker-compose.yml
    ├─ volumes:
    │   └─ - ${WORKSPACE_HOST_PATH:-/workspace}:/workspace:ro
    └─ Expands to:
        └─ - /host_mnt/Users/user/project:/workspace:ro
            ↓
maproom-mcp container starts
    └─ /workspace → /host_mnt/Users/user/project (bind mount from host)
        └─ Files accessible ✅
```

## Testing Architecture

### Test Structure

**Files:**
- Unit tests: `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`
- Integration tests: `packages/maproom-mcp/tests/integration/workspace-path-detection.int.test.ts`

```
describe('Docker-in-Docker Workspace Path Detection')
  ├─ describe('isInsideDocker()')
  │   ├─ it('detects /.dockerenv file')
  │   ├─ it('detects /run/.containerenv file')
  │   ├─ it('detects from cgroup')
  │   └─ it('returns false when not in Docker')
  │
  ├─ describe('getWorkspaceHostPath()')
  │   ├─ it('discovers host path from docker inspect')
  │   ├─ it('returns null when docker inspect fails')
  │   └─ it('returns null when no workspace mount exists')
  │
  ├─ describe('resolveWorkspacePath()')
  │   ├─ it('uses WORKSPACE_HOST_PATH if set')
  │   ├─ it('discovers path when inside Docker')
  │   ├─ it('uses process.cwd() on host')
  │   └─ it('falls back to /workspace if detection fails')
  │
  └─ describe('Integration')
      └─ it('sets WORKSPACE_HOST_PATH before startDockerCompose()')
```

### Test-Driven Implementation Order

1. **Write failing tests** for all functions
2. **Run tests** → all fail (functions don't exist yet)
3. **Implement functions** one at a time
4. **Run tests** → verify each function works
5. **Integration test** → verify end-to-end flow
6. **Manual verification** → actually run setup command

## Error Handling

### Failure Modes and Responses

| Failure Mode | Detection | Response |
|-------------|-----------|----------|
| Docker socket not accessible | `docker inspect` throws error | Return null, fallback to `/workspace` |
| Not in devcontainer | No `/.dockerenv` file | Use `process.cwd()` |
| No workspace mount | docker inspect returns empty | Return null, fallback to `/workspace` |
| Multiple workspace mounts | docker inspect returns multiple | Use first match |
| Permission denied on hostname | `hostname` command fails | Return null, fallback gracefully |

### User-Facing Messages

**Success (devcontainer):**
```
✓ Workspace path: /host_mnt/Users/danielbushman/git/manifoldlogic/crewchief
```

**Success (host):**
```
✓ Workspace path: /Users/danielbushman/git/manifoldlogic/crewchief
```

**Warning (detection failed):**
```
⚠️  Running inside Docker but could not discover workspace host path.
    Volume mount may fail. Set WORKSPACE_HOST_PATH manually if needed.
✓ Workspace path: /workspace (fallback)
```

**User override:**
```
✓ Workspace path: /custom/path (user-provided)
```

## Performance Considerations

### Execution Time

- `fs.existsSync('/.dockerenv')`: <1ms (file stat)
- `execFileSync('hostname', [])`: ~10ms (process spawn)
- `execFileSync('docker', ['inspect', ...])`: ~50-100ms (docker query)
- **Total overhead:** ~60-110ms (one-time during setup)

### Optimization Strategies

- **Lazy evaluation**: Only call docker inspect if inside Docker
- **No caching needed**: Runs once per setup, not per request
- **Early returns**: Check cheapest conditions first

## Security Considerations

### Path Validation

Minimal validation applied to WORKSPACE_HOST_PATH to prevent path traversal:

```javascript
function validateWorkspacePath(path) {
  // Check for path traversal patterns
  if (path.includes('..')) {
    console.warn(`⚠️  Workspace path contains ".." (path traversal risk): ${path}`);
    console.warn('    Proceeding with caution. Read-only mount limits risk.');
  }

  // Warn if relative path (not absolute)
  if (!path.startsWith('/')) {
    console.warn(`⚠️  Workspace path is not absolute: ${path}`);
    console.warn('    May cause unexpected behavior.');
  }

  // Don't verify path exists - container host may not see it
  return path;
}
```

**Rationale:**
- **Minimal validation**: MVP approach - warn but don't block
- **Read-only mount**: Primary security mitigation (in docker-compose.yml)
- **Path traversal**: Detect `..` but don't reject (user override may have valid reason)
- **No existence check**: Host vs container filesystem - can't verify from inside container
- **Defense in depth**: Combine with execFileSync (no shell injection)

### Buffer Limits and Timeouts

All `execFileSync()` calls include resource limits:

```javascript
// hostname command
execFileSync('hostname', [], {
  encoding: 'utf8',
  timeout: 5000,      // 5 second timeout (DoS prevention)
  maxBuffer: 1024     // 1KB max (hostname is short)
})

// docker inspect command
execFileSync('docker', [...], {
  encoding: 'utf8',
  timeout: 10000,     // 10 second timeout (DoS prevention)
  maxBuffer: 10240    // 10KB max (docker inspect output can be larger)
})
```

**Rationale:**
- **DoS prevention**: Prevents resource exhaustion from long-running or large outputs
- **Reasonable limits**: Sized appropriately for expected output
- **Security first**: Implemented in Phase 2 (not retrofitted in Phase 3)

## Backward Compatibility

### Existing Behavior Preserved

**For users who manually set WORKSPACE_HOST_PATH:**
```bash
export WORKSPACE_HOST_PATH=/custom/path
npx @crewchief/maproom-mcp setup --provider=openai
# Uses /custom/path ✅ (unchanged)
```

**For users running on host (not in devcontainer):**
```bash
# On host machine
npx @crewchief/maproom-mcp setup --provider=openai
# Uses process.cwd() ✅ (new behavior, but correct)
```

**For existing docker-compose.yml files:**
```yaml
volumes:
  - ${WORKSPACE_HOST_PATH:-/workspace}:/workspace:ro
```
- Still works with manual WORKSPACE_HOST_PATH
- Still defaults to /workspace if variable not set
- No breaking changes

## Future Enhancements (Out of Scope)

These are **not** part of this MVP but could be added later:

1. **Windows support**: Detect WSL2 vs native Windows paths
2. **Podman support**: Handle podman-specific mount discovery
3. **Kubernetes support**: Detect K8s environment and adjust accordingly
4. **Path validation**: Verify discovered path exists and is readable
5. **Config file override**: Allow `.maproom-mcp/config.json` to override

## Implementation Checklist

- [ ] Write `isInsideDocker()` test
- [ ] Write `getWorkspaceHostPath()` test
- [ ] Write `resolveWorkspacePath()` test
- [ ] Write integration test
- [ ] Implement `isInsideDocker()`
- [ ] Implement `getWorkspaceHostPath()`
- [ ] Implement `resolveWorkspacePath()`
- [ ] Add call to `resolveWorkspacePath()` in `runSetup()`
- [ ] Run unit tests → verify all pass
- [ ] Manual test: Run setup in devcontainer
- [ ] Manual test: Run setup on host
- [ ] Verify maproom-mcp container can access files
- [ ] Update documentation

## Success Criteria

1. ✅ All unit tests pass
2. ✅ Integration test passes
3. ✅ Manual setup in devcontainer succeeds
4. ✅ Container can access workspace files
5. ✅ No breaking changes to existing workflows
6. ✅ Clear user feedback in console
