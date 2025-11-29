# Quality Strategy: Auto Container Startup Integration

## Testing Philosophy

**Core Principle**: Build confidence in the integration points, not exhaustive coverage of already-tested components.

**Why**:
- DockerManager already tested (VSMAP-1001, 6,318 bytes of tests)
- ProcessOrchestrator already tested (VSMAP-1003)
- MCPConfigWriter already tested (MCPINIT-1001, 14,777 bytes of tests)

**What We Test**: The ~50 lines of NEW integration code that wires these together.

## Risk Assessment

### High-Risk Areas (Must Test)

#### 1. Docker Startup Integration
**Risk**: Extension calls Docker manager but doesn't handle errors properly

**Test Coverage**:
- ✅ Success path: `ensureDockerRunning()` completes without error
- ✅ Error path: Docker Desktop not running → User-friendly error shown
- ✅ Error path: `docker compose` fails → Error surfaced to user
- ✅ Error path: Health check timeout → Clear timeout message
- ✅ Cleanup: `dispose()` calls `DockerManager.stop()`

**Why Critical**:
- First thing users encounter
- Blocking error (nothing works without Docker)
- Must have clear recovery instructions

#### 2. Activation Flow Sequencing
**Risk**: Watch processes start before Docker ready

**Test Coverage**:
- ✅ Docker starts BEFORE PostgreSQL check
- ✅ PostgreSQL check BEFORE watch processes
- ✅ Each step waits for previous to complete
- ✅ Error in step N prevents step N+1

**Why Critical**:
- This bug caused the user's issue
- Timing-dependent (race conditions possible)
- Must guarantee ordering

#### 3. Idempotency
**Risk**: Extension starts duplicate containers

**Test Coverage**:
- ✅ Services already running → No-op (doesn't fail)
- ✅ Extension reloaded → Doesn't create duplicate containers
- ✅ Multiple workspaces → Share same Docker services

**Why Critical**:
- Docker port conflicts are cryptic
- Users might reload extension frequently
- Resource leaks if not idempotent

### Medium-Risk Areas (Should Test)

#### 4. Cleanup on Deactivation
**Risk**: Docker containers left orphaned

**Test Coverage**:
- ✅ Deactivate extension → Containers stop
- ✅ VSCode quits → Containers stop
- ✅ Extension crashes → Containers stop (best effort)

**Why Important**:
- Resource leak (RAM, disk)
- Port conflicts on next activation
- Professional polish

#### 5. Progress Notifications
**Risk**: User sees "Starting..." forever if hung

**Test Coverage**:
- ✅ Progress shows during Docker startup
- ✅ Progress shows during health checks
- ✅ Progress completes or shows error (never hangs)

**Why Important**:
- User experience (feedback during wait)
- Debugging (logs show which step failed)

### Low-Risk Areas (Nice to Test)

#### 6. Error Message Quality
**Risk**: Cryptic errors confuse users

**Manual Verification**:
- Error messages use plain language
- Error messages suggest recovery actions
- "Show Logs" button works

**Why Low Risk**:
- Doesn't break functionality
- Easy to fix post-release
- Subjective (hard to automate)

## Test Coverage Strategy

### Unit Tests (80% of testing effort)

**File**: `packages/vscode-maproom/src/extension.test.ts` (new test suite)

**Test Cases**:

```typescript
describe('ensureDockerRunning', () => {
  it('starts Docker services successfully', async () => {
    // Mock DockerManager
    const mockDockerManager = {
      ensureServicesRunning: jest.fn().mockResolvedValue(undefined),
      stop: jest.fn()
    }

    // Call function
    await ensureDockerRunning(mockContext)

    // Verify
    expect(mockDockerManager.ensureServicesRunning).toHaveBeenCalled()
    expect(mockContext.subscriptions).toContain(expect.objectContaining({
      dispose: expect.any(Function)
    }))
  })

  it('shows error when Docker not running', async () => {
    // Mock DockerManager to fail
    const mockDockerManager = {
      ensureServicesRunning: jest.fn().mockRejectedValue(
        new Error('Docker Desktop is not running')
      )
    }

    // Call function (should throw)
    await expect(ensureDockerRunning(mockContext)).rejects.toThrow()

    // Verify error shown to user
    expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
      expect.stringContaining('Docker Desktop'),
      expect.any(String),
      expect.any(String)
    )
  })

  it('registers cleanup on deactivation', async () => {
    const mockDockerManager = {
      ensureServicesRunning: jest.fn().mockResolvedValue(undefined),
      stop: jest.fn().mockResolvedValue(undefined)
    }

    await ensureDockerRunning(mockContext)

    // Get dispose function
    const disposeRegistration = mockContext.subscriptions[0]

    // Call dispose
    await disposeRegistration.dispose()

    // Verify stop called
    expect(mockDockerManager.stop).toHaveBeenCalled()
  })
})

describe('initializeServices flow', () => {
  it('calls ensureDockerRunning before ensurePostgresAvailable', async () => {
    const callOrder: string[] = []

    // Mock functions that track call order
    global.ensureDockerRunning = jest.fn(() => {
      callOrder.push('docker')
      return Promise.resolve()
    })
    global.ensurePostgresAvailable = jest.fn(() => {
      callOrder.push('postgres')
      return Promise.resolve()
    })

    // Run initialization
    await initializeServices(mockContext, '/test/workspace')

    // Verify ordering
    expect(callOrder).toEqual(['docker', 'postgres', ...])
  })

  it('stops initialization if Docker startup fails', async () => {
    global.ensureDockerRunning = jest.fn().mockRejectedValue(
      new Error('Docker failed')
    )
    global.ensurePostgresAvailable = jest.fn()

    // Should throw before reaching PostgreSQL check
    await expect(
      initializeServices(mockContext, '/test/workspace')
    ).rejects.toThrow('Docker failed')

    // PostgreSQL check should NOT be called
    expect(global.ensurePostgresAvailable).not.toHaveBeenCalled()
  })
})

describe('runFirstTimeSetup flow', () => {
  it('starts Docker after setup wizard completes', async () => {
    // Mock setup wizard (returns provider)
    global.runSetupWizard = jest.fn().mockResolvedValue('openai')
    global.ensureDockerRunning = jest.fn().mockResolvedValue(undefined)

    // Run first-time setup
    await runFirstTimeSetup(mockContext, '/test/workspace')

    // Verify Docker started after wizard
    expect(global.ensureDockerRunning).toHaveBeenCalled()
  })

  it('skips Docker startup if user cancels setup', async () => {
    // Mock setup wizard (user cancels)
    global.runSetupWizard = jest.fn().mockResolvedValue(null)
    global.ensureDockerRunning = jest.fn()

    // Run first-time setup
    await runFirstTimeSetup(mockContext, '/test/workspace')

    // Docker should NOT start
    expect(global.ensureDockerRunning).not.toHaveBeenCalled()
  })
})
```

**Coverage Target**: 90% of new integration code

**Why High Coverage**: Only ~50 lines to test, critical path, high ROI

### Integration Tests (15% of testing effort)

**File**: `packages/vscode-maproom/src/integration.test.ts` (extend existing)

**Test Cases**:

```typescript
describe('Docker integration (E2E)', () => {
  beforeEach(async () => {
    // Stop any existing containers
    await exec('docker compose -f test/fixtures/docker-compose.yml down')
  })

  it('starts Docker services and spawns watch processes', async () => {
    // Activate extension
    const extension = vscode.extensions.getExtension('manifoldlogic.vscode-maproom')
    await extension.activate()

    // Wait for initialization
    await waitForCondition(() =>
      statusBar.text.includes('Watching')
    , 30000)

    // Verify containers running
    const containers = await exec('docker ps --format "{{.Names}}"')
    expect(containers).toContain('maproom-postgres')
    expect(containers).toContain('maproom-mcp')

    // Verify watch processes spawned
    expect(orchestrator.getStatus().get('watch').running).toBe(true)
  })

  it('handles Docker Desktop not running', async () => {
    // Stop Docker Desktop (manually, before test)
    // Run: killall Docker

    // Activate extension (should fail gracefully)
    const extension = vscode.extensions.getExtension('manifoldlogic.vscode-maproom')
    await extension.activate()

    // Verify error shown (check notification)
    await waitForCondition(() =>
      lastShownError?.includes('Docker Desktop')
    )

    // Verify watch processes NOT spawned
    expect(orchestrator).toBeUndefined()
  })

  it('stops containers on deactivation', async () => {
    // Activate extension
    const extension = vscode.extensions.getExtension('manifoldlogic.vscode-maproom')
    await extension.activate()
    await waitForCondition(() => statusBar.text.includes('Watching'))

    // Deactivate extension
    await extension.deactivate()

    // Wait for cleanup
    await new Promise(resolve => setTimeout(resolve, 2000))

    // Verify containers stopped
    const containers = await exec('docker ps --format "{{.Names}}"')
    expect(containers).not.toContain('maproom-postgres')
    expect(containers).not.toContain('maproom-mcp')
  })
})
```

**Prerequisites**:
- Docker Desktop installed and running
- Test fixtures: `test/fixtures/docker-compose.yml`
- CI environment: Docker in Docker (DinD) setup

**Coverage Target**: Critical paths only (startup, error, cleanup)

**Why Minimal**: Integration tests are slow, brittle, expensive to maintain

### Manual Testing (5% of testing effort)

**Checklist** (before release):

**Scenario 1: Fresh Install**
- [ ] Install extension in clean VSCode
- [ ] Open workspace
- [ ] Setup wizard appears
- [ ] Select OpenAI provider
- [ ] Enter API key
- [ ] Progress notification: "Starting Docker services..."
- [ ] Docker containers start (verify with `docker ps`)
- [ ] Progress notification: "Running initial scan..."
- [ ] Status bar: "Watching: X files"
- [ ] No errors in Output panel

**Scenario 2: Docker Not Running**
- [ ] Stop Docker Desktop
- [ ] Reload VSCode
- [ ] Error notification appears: "Maproom requires Docker Desktop to be running."
- [ ] Click "Open Docker Desktop" → Docker Desktop opens
- [ ] Start Docker Desktop
- [ ] Reload extension
- [ ] Extension initializes successfully

**Scenario 3: Services Already Running**
- [ ] Start containers manually: `docker compose up -d`
- [ ] Activate extension
- [ ] Extension detects existing containers (idempotent)
- [ ] No duplicate containers created
- [ ] Watch processes start successfully

**Scenario 4: Extension Reload**
- [ ] Extension running normally
- [ ] Reload extension (`Developer: Reload Window`)
- [ ] Containers remain running (not stopped/restarted)
- [ ] Watch processes restart
- [ ] No errors

**Scenario 5: Deactivation**
- [ ] Extension running normally
- [ ] Disable extension
- [ ] Containers stop gracefully
- [ ] No orphaned containers (`docker ps` empty)
- [ ] No zombie processes

**Scenario 6: Multiple Workspaces**
- [ ] Open workspace A (extension starts containers)
- [ ] Open workspace B in new window
- [ ] Workspace B detects existing containers (shares)
- [ ] Close workspace A
- [ ] Containers remain running (workspace B still needs them)
- [ ] Close workspace B
- [ ] Containers stop (last workspace closed)

**Time Budget**: ~30 minutes for full manual pass

## Testing Tools

### Unit Testing
- **Framework**: Vitest (already configured)
- **Mocking**: `vi.fn()`, `vi.mock()`
- **Coverage**: `vitest --coverage`

### Integration Testing
- **Framework**: `@vscode/test-electron`
- **Fixtures**: `test/fixtures/` directory
- **Setup**: Docker in Docker (for CI)

### Manual Testing
- **Environment**: Local macOS, Linux (CI), Windows (pre-release)
- **Docker**: Docker Desktop latest stable
- **VSCode**: Latest stable + Insiders

## CI/CD Integration

### GitHub Actions Workflow

**File**: `.github/workflows/test-vscode-extension.yml`

```yaml
name: Test VSCode Extension

on:
  pull_request:
    paths:
      - 'packages/vscode-maproom/**'
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      docker:
        image: docker:dind
        options: --privileged

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install pnpm
        uses: pnpm/action-setup@v4

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Run unit tests
        run: pnpm --filter @crewchief/vscode-maproom test

      - name: Run integration tests
        run: pnpm --filter @crewchief/vscode-maproom test:integration
        env:
          DOCKER_HOST: tcp://docker:2375

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./packages/vscode-maproom/coverage/coverage-final.json
```

**Triggers**:
- Pull requests touching vscode-maproom
- Pushes to main branch

**Requirements**:
- Docker available in CI (services.docker)
- Integration tests use DOCKER_HOST for DinD

## Acceptance Criteria (Final Verification)

Before marking project complete:

### Functional Requirements
- [ ] Extension activates successfully with Docker running
- [ ] Extension shows clear error when Docker not running
- [ ] Docker services start automatically on activation
- [ ] Docker services stop on deactivation
- [ ] Watch processes start after Docker healthy
- [ ] Status bar shows correct state throughout

### Quality Requirements
- [ ] Unit tests: >90% coverage of new code
- [ ] Integration tests: All critical paths passing
- [ ] Manual tests: Full checklist completed
- [ ] CI: All tests passing on Linux

### User Experience Requirements
- [ ] Error messages are clear and actionable
- [ ] Progress notifications show during startup
- [ ] No hanging states (always completes or errors)
- [ ] Documentation updated with Docker requirements

## Known Limitations

**Accepted for MVP**:
1. **Requires Docker Desktop**: No embedded database option
   - Mitigation: Clear error message, documented requirement
   - Future: SQLite embedded option (Phase 10+)

2. **30s timeout for health checks**: May fail on very slow machines
   - Mitigation: Timeout configurable via setting (future)
   - Future: Adaptive timeout based on machine performance

3. **No automatic Docker Desktop installation**: User must install manually
   - Mitigation: Extension README has installation instructions
   - Future: Detect missing Docker, link to download page

4. **Port 5432/3000 conflicts**: Fails if ports in use
   - Mitigation: Docker Compose error surfaced to user
   - Future: Configurable ports via settings

**Not Testing**:
- Multi-OS (Windows, Linux) - will test in pre-release
- Network proxies/firewalls - document as limitation
- Docker Desktop beta versions - require stable only
- ARM64 vs x64 compatibility - assumed via Docker

## Regression Prevention

**Before Each Release**:
1. Run full manual test checklist
2. Verify CI passing on all platforms
3. Test on fresh VSCode install (clean state)
4. Document any new known issues

**After Each Release**:
1. Monitor GitHub issues for Docker-related bugs
2. Track activation success rate (telemetry, future)
3. Update test cases for newly discovered edge cases

## Summary

**Testing Effort Distribution**:
- Unit tests: 80% (high ROI, fast feedback)
- Integration tests: 15% (critical paths, slower)
- Manual tests: 5% (UX polish, pre-release only)

**Coverage Goals**:
- New integration code: 90% (small surface area)
- Existing components: Already tested (reuse)
- Critical paths: 100% (via integration tests)

**Risk Mitigation**:
- High-risk areas: Comprehensive automated tests
- Medium-risk areas: Integration tests + manual spot checks
- Low-risk areas: Manual testing only

**Total Testing Time**: ~4 hours (writing tests + manual verification)

This is a **high-quality, low-effort** testing strategy because we're testing a thin integration layer over already-tested components.
