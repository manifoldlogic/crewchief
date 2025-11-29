# Quality Strategy: Docker-in-Docker Workspace Path Detection

## Testing Philosophy

**Test-Driven Development:** Write failing tests first to prove we understand the problem, then implement code to make tests pass.

**Confidence over Coverage:** Focus on critical paths and integration points, not achieving 100% coverage.

**MVP Mindset:** Tests prevent rework and catch regressions, not ceremonial checkboxes.

## Test Strategy

### Test Pyramid

```
                    /\
                   /  \
                  / E2E \          1 test
                 /------\
                /        \
               / Integration\     3 tests
              /------------\
             /              \
            /  Unit Tests    \   12 tests
           /------------------\
```

**Rationale:**
- **Unit tests** validate individual functions work correctly
- **Integration tests** verify functions work together
- **E2E test** confirms actual user workflow succeeds

### Unit Tests

**File:** `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`

#### Test Suite 1: isInsideDocker()

**Purpose:** Verify Docker environment detection works across different container types

```javascript
describe('isInsideDocker()', () => {
  it('should detect /.dockerenv file', () => {
    // GIVEN: /.dockerenv file exists
    // WHEN: isInsideDocker() is called
    // THEN: returns true
  });

  it('should detect /run/.containerenv file (Podman)', () => {
    // GIVEN: /run/.containerenv file exists (Podman)
    // WHEN: isInsideDocker() is called
    // THEN: returns true
  });

  it('should detect from /proc/1/cgroup', () => {
    // GIVEN: /proc/1/cgroup contains "docker" or "containerd"
    // WHEN: isInsideDocker() is called
    // THEN: returns true
  });

  it('should return false when not in Docker', () => {
    // GIVEN: No Docker markers exist
    // WHEN: isInsideDocker() is called
    // THEN: returns false
  });

  it('should return false when cgroup read fails', () => {
    // GIVEN: /proc/1/cgroup is not readable
    // WHEN: isInsideDocker() is called
    // THEN: returns false (graceful failure)
  });
});
```

**Mocking strategy:**
- Mock `fs.existsSync()` to simulate file presence
- Mock `fs.readFileSync()` to simulate cgroup content
- Test graceful error handling

#### Test Suite 2: getWorkspaceHostPath()

**Purpose:** Verify host path discovery from docker inspect

```javascript
describe('getWorkspaceHostPath()', () => {
  it('should discover host path from docker inspect', () => {
    // GIVEN: hostname returns "container-abc123"
    // AND: docker inspect returns "/host_mnt/Users/user/project"
    // WHEN: getWorkspaceHostPath() is called
    // THEN: returns "/host_mnt/Users/user/project"
  });

  it('should return null when docker inspect fails', () => {
    // GIVEN: docker inspect command throws error
    // WHEN: getWorkspaceHostPath() is called
    // THEN: returns null (graceful failure)
  });

  it('should return null when no workspace mount exists', () => {
    // GIVEN: docker inspect returns empty string
    // WHEN: getWorkspaceHostPath() is called
    // THEN: returns null
  });

  it('should handle hostname command failure', () => {
    // GIVEN: hostname command throws error
    // WHEN: getWorkspaceHostPath() is called
    // THEN: returns null (graceful failure)
  });

  it('should trim whitespace from paths', () => {
    // GIVEN: docker inspect returns "  /host_mnt/path  \n"
    // WHEN: getWorkspaceHostPath() is called
    // THEN: returns "/host_mnt/path" (trimmed)
  });
});
```

**Mocking strategy:**
- Mock `execSync()` for hostname and docker inspect commands
- Simulate various output formats
- Test error handling paths

#### Test Suite 3: resolveWorkspacePath()

**Purpose:** Verify workspace path resolution logic prioritizes correctly

```javascript
describe('resolveWorkspacePath()', () => {
  it('should use WORKSPACE_HOST_PATH if set', () => {
    // GIVEN: process.env.WORKSPACE_HOST_PATH = "/custom/path"
    // WHEN: resolveWorkspacePath() is called
    // THEN: returns "/custom/path" (user override)
  });

  it('should discover path when inside Docker', () => {
    // GIVEN: isInsideDocker() returns true
    // AND: getWorkspaceHostPath() returns "/host_mnt/path"
    // WHEN: resolveWorkspacePath() is called
    // THEN: returns "/host_mnt/path"
  });

  it('should use process.cwd() on host', () => {
    // GIVEN: isInsideDocker() returns false
    // AND: process.cwd() returns "/Users/user/project"
    // WHEN: resolveWorkspacePath() is called
    // THEN: returns "/Users/user/project"
  });

  it('should fall back to /workspace if detection fails', () => {
    // GIVEN: isInsideDocker() returns true
    // AND: getWorkspaceHostPath() returns null
    // WHEN: resolveWorkspacePath() is called
    // THEN: returns "/workspace" (fallback)
  });

  it('should log diagnostic messages', () => {
    // GIVEN: Various scenarios
    // WHEN: resolveWorkspacePath() is called
    // THEN: diagnosticLog() is called with appropriate messages
  });
});
```

**Mocking strategy:**
- Mock `isInsideDocker()` to simulate environments
- Mock `getWorkspaceHostPath()` to simulate discovery
- Mock `process.cwd()` for host execution
- Spy on `diagnosticLog()` to verify logging

### Integration Tests

**File:** `packages/maproom-mcp/tests/integration/workspace-path-detection.int.test.ts`

#### Integration Test 1: runSetup() Flow

```javascript
describe('runSetup() integration', () => {
  it('should set WORKSPACE_HOST_PATH before startDockerCompose()', async () => {
    // GIVEN: Running in devcontainer environment
    // AND: Workspace mounted at /host_mnt/Users/user/project
    // WHEN: runSetup() is called
    // THEN: process.env.WORKSPACE_HOST_PATH is set to host path
    // AND: startDockerCompose() receives correct environment
  });

  it('should pass WORKSPACE_HOST_PATH to docker compose spawn', async () => {
    // GIVEN: WORKSPACE_HOST_PATH is set to "/test/path"
    // WHEN: startDockerCompose() is called
    // THEN: spawn() receives env with WORKSPACE_HOST_PATH="/test/path"
  });

  it('should handle detection failure gracefully in setup', async () => {
    // GIVEN: Detection functions return null
    // WHEN: runSetup() is called
    // THEN: Falls back to /workspace with warning
    // AND: Setup continues without error
  });
});
```

**Mocking strategy:**
- Mock Docker commands (spawn, execFileSync)
- Spy on `startDockerCompose()` to verify env passed
- Partial mocks to test real integration

### End-to-End Test (Deferred to Post-MVP)

**File:** `packages/maproom-mcp/tests/e2e/setup-devcontainer.e2e.test.ts`

#### E2E Test: Full Setup Flow

```javascript
describe('E2E: Setup in devcontainer', () => {
  it('should complete setup and allow container to access workspace', async () => {
    // GIVEN: Clean environment (no existing containers)
    // AND: Running inside devcontainer
    // WHEN: npx @crewchief/maproom-mcp setup --provider=openai
    // THEN: Setup completes successfully
    // AND: Containers are running
    // AND: docker exec maproom-mcp ls /workspace succeeds
    // AND: Files are accessible in container
  }, 60000); // 60s timeout for full setup
});
```

**Requirements:**
- Only runs in actual devcontainer (skip if not detected)
- Cleans up after itself (stops/removes containers)
- Verifies actual Docker operations work

## Test Execution

### Running Tests

```bash
# Unit tests only (fast)
pnpm test workspace-path-detection

# Integration tests (medium)
pnpm test workspace-path-detection.integration

# E2E test (slow, requires devcontainer)
pnpm test:e2e setup-devcontainer
```

### CI/CD Integration

**GitHub Actions workflow:**
```yaml
- name: Unit Tests
  run: pnpm test workspace-path-detection

- name: Integration Tests
  run: pnpm test workspace-path-detection.integration

# E2E test only runs in devcontainer environment
- name: E2E Tests
  if: env.IN_DEVCONTAINER == 'true'
  run: pnpm test:e2e setup-devcontainer
```

## Mocking Strategy

### Tools and Libraries

**Test framework:** Vitest (already in use)

**Mocking approach:**
```javascript
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';

describe('workspace-path-detection', () => {
  beforeEach(() => {
    // Reset mocks before each test
    vi.clearAllMocks();
  });

  afterEach(() => {
    // Clean up environment variables
    delete process.env.WORKSPACE_HOST_PATH;
  });
});
```

### Key Mocks

**File system operations:**
```javascript
vi.mock('fs', () => ({
  existsSync: vi.fn(),
  readFileSync: vi.fn()
}));
```

**Child process operations:**

**Note on CommonJS Mocking:** The functions being tested are in `bin/cli.cjs` (CommonJS format using `require()`), but our tests are TypeScript/ESM. Vitest handles this seamlessly - we can mock CommonJS modules from ESM tests using the same patterns:

```javascript
// Mocking child_process for CommonJS code testing
// Works even though bin/cli.cjs uses: const { execFileSync } = require('child_process')
vi.mock('child_process', () => ({
  execFileSync: vi.fn(),
  spawn: vi.fn()
}));

// Example usage in tests:
import { execFileSync } from 'child_process';

it('should discover host path from docker inspect', () => {
  // Mock hostname command
  vi.mocked(execFileSync).mockReturnValueOnce('container-abc123');

  // Mock docker inspect command
  vi.mocked(execFileSync).mockReturnValueOnce('/host_mnt/Users/user/project');

  const result = getWorkspaceHostPath();

  expect(result).toBe('/host_mnt/Users/user/project');
  expect(execFileSync).toHaveBeenCalledWith('hostname', [], expect.objectContaining({
    encoding: 'utf8',
    timeout: 5000,
    maxBuffer: 1024
  }));
  expect(execFileSync).toHaveBeenCalledWith('docker', [
    'inspect',
    'container-abc123',
    '--format',
    '{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}'
  ], expect.objectContaining({
    encoding: 'utf8',
    timeout: 10000,
    maxBuffer: 10240
  }));
});
```

**Process environment:**
```javascript
const originalEnv = process.env;
beforeEach(() => {
  process.env = { ...originalEnv };
});
afterEach(() => {
  process.env = originalEnv;
});
```

## Test Coverage Goals

**Target coverage:** 80% of new code

**Critical paths (must be 100%):**
- ✅ isInsideDocker() - all branches
- ✅ getWorkspaceHostPath() - all error paths
- ✅ resolveWorkspacePath() - all priority tiers
- ✅ Integration with runSetup() - env variable setting

**Nice to have (70% acceptable):**
- Error message formatting
- Diagnostic logging
- Edge cases (multiple mounts, etc.)

## Manual Testing Checklist

After automated tests pass, verify manually:

### Devcontainer Environment

- [ ] Start fresh devcontainer
- [ ] Run `npx @crewchief/maproom-mcp setup --provider=openai`
- [ ] Verify console shows: `✓ Workspace path: /host_mnt/...`
- [ ] Verify containers started: `docker ps | grep maproom`
- [ ] Verify volume mount: `docker inspect maproom-mcp | grep workspace`
- [ ] Verify file access: `docker exec maproom-mcp ls /workspace`
- [ ] Run scan: `npx @crewchief/maproom-mcp scan`
- [ ] Verify files indexed in database

### Host Environment (macOS/Linux)

- [ ] Exit devcontainer (run on host)
- [ ] Run `npx @crewchief/maproom-mcp setup --provider=openai`
- [ ] Verify console shows: `✓ Workspace path: /Users/.../project`
- [ ] Verify containers started
- [ ] Verify volume mount points to current directory
- [ ] Verify file access works

### Error Cases

- [ ] Set invalid WORKSPACE_HOST_PATH manually
- [ ] Verify warning message appears
- [ ] Verify setup continues (doesn't crash)
- [ ] Set valid WORKSPACE_HOST_PATH manually
- [ ] Verify override works (uses manual value)

## Regression Testing

### Existing Functionality to Verify

After implementing the fix, ensure these still work:

- [ ] Setup with Ollama provider
- [ ] Setup with Google provider
- [ ] Setup with OpenAI provider
- [ ] Scan command after setup
- [ ] Watch command after setup
- [ ] Database initialization
- [ ] Schema validation
- [ ] Health checks for containers

## Test Data Management

### Mock Data

**Example devcontainer mount output:**
```javascript
const mockDockerInspect = {
  devcontainer: '/host_mnt/Users/danielbushman/git/manifoldlogic/crewchief',
  codespace: '/workspaces/crewchief',
  podman: '/var/home/containers/storage/volumes/workspace/_data'
};
```

**Example cgroup content:**
```javascript
const mockCgroup = {
  docker: '12:memory:/docker/abc123...',
  containerd: '12:memory:/system.slice/containerd.service',
  host: '12:memory:/'
};
```

### Test Fixtures

**Create test fixtures directory:**
```
tests/fixtures/
├── docker-inspect-output.json
├── cgroup-docker.txt
├── cgroup-containerd.txt
└── cgroup-host.txt
```

## Performance Testing

### Benchmarks

**Target performance:**
- Detection overhead: <100ms
- No impact on existing setup time
- No network calls (only local docker inspect)

**Measurement:**
```javascript
it('should complete detection in under 100ms', () => {
  const start = Date.now();
  const result = resolveWorkspacePath();
  const duration = Date.now() - start;

  expect(duration).toBeLessThan(100);
  expect(result).toBeDefined();
});
```

## Failure Scenarios

### Expected Failures (Should Handle Gracefully)

| Scenario | Expected Behavior | Test Coverage |
|----------|------------------|---------------|
| Docker socket not accessible | Fall back to `/workspace` with warning | ✅ Unit test |
| No workspace mount | Fall back to `/workspace` with warning | ✅ Unit test |
| docker inspect returns empty | Use fallback path | ✅ Unit test |
| hostname command fails | Return null gracefully | ✅ Unit test |
| Permission denied on /proc | Detection fails, use fallback | ✅ Unit test |

### Unexpected Failures (Should Crash Safely)

| Scenario | Expected Behavior | Test Coverage |
|----------|------------------|---------------|
| docker-compose.yml missing | Setup fails with clear error | ✅ Existing tests |
| Docker daemon not running | Setup fails with clear error | ✅ Existing tests |
| Invalid provider specified | Setup fails with clear error | ✅ Existing tests |

## Documentation Requirements

### Code Documentation

- [ ] JSDoc comments for all new functions
- [ ] Inline comments for complex logic
- [ ] Error messages are clear and actionable

### User Documentation

- [ ] Update README with devcontainer support
- [ ] Add troubleshooting section for path detection failures
- [ ] Document WORKSPACE_HOST_PATH override option

### Developer Documentation

- [ ] Update DOCKER_WORKSPACE_SOLUTION.md with implemented approach
- [ ] Document test strategy in TESTING.md
- [ ] Add migration guide for existing users

## Success Criteria

1. ✅ All 16 tests pass (12 unit + 3 integration + 1 E2E)
2. ✅ Code coverage ≥80% for new functions
3. ✅ Manual testing checklist completed
4. ✅ No regressions in existing functionality
5. ✅ Clear error messages for failure cases
6. ✅ Documentation updated
7. ✅ E2E test proves user workflow works

## Risk Mitigation

### High Risk Areas

1. **Docker detection fails in edge cases**
   - Mitigation: Graceful fallback to `/workspace`
   - Test coverage: All error paths tested

2. **Breaking change for existing users**
   - Mitigation: Respects manual WORKSPACE_HOST_PATH override
   - Test coverage: Backward compatibility tests

3. **Performance regression**
   - Mitigation: Lazy evaluation, benchmarks
   - Test coverage: Performance test

4. **Platform-specific issues (Windows, macOS, Linux)**
   - Mitigation: Platform detection in tests
   - Test coverage: Run tests on multiple platforms in CI

## Rollback Plan

If the fix causes issues:

1. **Immediate rollback:** Revert commit
2. **User workaround:** Set WORKSPACE_HOST_PATH manually
3. **Investigation:** Review test failures and user reports
4. **Re-implementation:** Address issues found, add more tests
5. **Gradual rollout:** Beta test with subset of users first
