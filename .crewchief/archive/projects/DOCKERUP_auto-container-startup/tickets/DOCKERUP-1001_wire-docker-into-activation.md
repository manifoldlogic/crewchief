# Ticket: DOCKERUP-1001: Wire DockerManager into Extension Activation Flow

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Integrate DockerManager into VSCode extension activation flow to automatically start Docker containers (PostgreSQL + MCP server) before initializing services. This is a pure integration task connecting existing, tested infrastructure.

## Background

The VSCode Maproom extension has all Docker infrastructure built (VSMAP-1001, Nov 16) but not connected to the activation flow. Currently:

- **Problem**: Extension activation fails with "DATABASE_URL env var is required" because Docker containers never start
- **Root Cause**: DockerManager.ensureServicesRunning() is never called
- **Impact**: Users must manually run `npx crewchief-maproom daemon start` before using extension
- **Solution**: Call DockerManager at activation time, before ProcessOrchestrator attempts to connect

This ticket implements Phase 1 of the DOCKERUP project plan (lines 81-203 in plan.md), specifically the automatic Docker startup integration described in the implementation section.

**Why This is Trivial**:
- All infrastructure exists from VSMAP-1001 (Nov 16) - DockerManager is production-ready
- DockerManager.ensureServicesRunning() does all the work (health checks, retries, error handling)
- Just calling existing methods in correct order (Docker → PostgreSQL → Watch processes)
- Zero new logic, pure integration (~50 lines of production code)

## Acceptance Criteria

### Functional Requirements
- [ ] Extension activates with Docker running → Watch processes start successfully (no errors)
- [ ] Extension activates without Docker → Shows error notification: "Maproom requires Docker Desktop to be running."
- [ ] Error notification has three actionable buttons: "Open Docker Desktop", "Show Logs", "Retry"
- [ ] Setup wizard completes → Docker starts automatically → Initial scan runs (no manual commands)
- [ ] Extension deactivation → Docker containers stop gracefully (no orphaned containers)
- [ ] Multiple workspaces → Share containers (idempotent behavior, no duplicate containers)

### Quality Requirements
- [ ] Unit tests: Greater than 90% coverage of new ensureDockerRunning() function
- [ ] Unit tests: Verify Docker called before PostgreSQL in initializeServices() flow
- [ ] Unit tests: Verify cleanup handler registered (dispose calls dockerManager.stop())
- [ ] Unit tests: Verify error handling (Docker not running, connection failures)
- [ ] Manual tests: All 5 scenarios passing (fresh install, Docker not running, services already running, deactivation, multiple workspaces)
- [ ] No regressions: All existing extension.test.ts tests still pass

### Documentation Requirements
- [ ] README.md updated with Docker Desktop requirement in System Requirements section
- [ ] README.md has new Troubleshooting section with Docker-related error scenarios
- [ ] CHANGELOG.md entry: "[0.3.0] - Automatic Docker container startup - Extension now starts PostgreSQL and MCP server automatically"

## Technical Requirements

### Files to Modify

1. **packages/vscode-maproom/src/extension.ts** (~40 lines added)
   - Add import: `import { DockerManager } from './docker/manager'`
   - Create: `ensureDockerRunning(context: vscode.ExtensionContext): Promise<void>` function (~30 lines with error handling)
   - Modify: `initializeServices()` - Add Docker startup before PostgreSQL check (+2 lines)
   - Modify: `runFirstTimeSetup()` - Add Docker startup after wizard, before initial scan (+2 lines)

2. **packages/vscode-maproom/src/extension.test.ts** (~300 lines added)
   - Add test suite: `describe('ensureDockerRunning', () => {...})` (~150 lines)
     - Test: Docker starts successfully, cleanup registered
     - Test: Docker not running, shows error notification
     - Test: Error notification buttons work correctly
     - Test: Dispose handler calls dockerManager.stop()
   - Add flow tests: `describe('initializeServices flow', () => {...})` (~100 lines)
     - Test: Docker called before PostgreSQL check
     - Test: Progress messages appear in correct order
     - Test: Docker failure prevents service initialization
   - Add setup tests: `describe('runFirstTimeSetup flow', () => {...})` (~50 lines)
     - Test: Docker starts after wizard, before initial scan
     - Test: Docker failure shows error, aborts setup

3. **packages/vscode-maproom/README.md** (~50 lines added)
   - Add Docker Desktop to System Requirements table (1 line)
   - Add "Troubleshooting" section with 3 scenarios:
     - "Maproom requires Docker Desktop to be running" error
     - Port conflicts (5432, 3000)
     - Containers not stopping on deactivation

4. **packages/vscode-maproom/CHANGELOG.md** (~5 lines added)
   - Add [0.3.0] section with automatic Docker startup feature
   - Note: Extension now handles Docker lifecycle automatically

### Implementation Details

#### New Function: ensureDockerRunning()

```typescript
/**
 * Ensure Docker services are running
 *
 * Multi-workspace behavior:
 * - Multiple VSCode workspaces share the same Docker containers
 * - DockerManager.ensureServicesRunning() is idempotent (safe to call multiple times)
 * - Containers remain running until last workspace closes
 * - Each workspace registers its own cleanup handler
 */
async function ensureDockerRunning(context: vscode.ExtensionContext): Promise<void> {
  const dockerManager = new DockerManager(outputChannel!)

  try {
    await dockerManager.ensureServicesRunning()
    context.subscriptions.push({
      dispose: () => void dockerManager.stop()
    })
  } catch (error: any) {
    // Error message templates:
    // - "Maproom requires Docker Desktop to be running." (Docker not running)
    // - "Failed to start Docker services: [specific error]" (other errors)
    const action = await vscode.window.showErrorMessage(
      'Maproom requires Docker Desktop to be running.',
      'Open Docker Desktop',
      'Show Logs',
      'Retry'
    )

    if (action === 'Show Logs') outputChannel?.show()
    throw new Error(`Failed to start Docker services: ${error.message}`)
  }
}
```

#### Integration Point 1: initializeServices()

```typescript
async function initializeServices(context, workspaceRoot) {
  await vscode.window.withProgress({...}, async (progress) => {
    // NEW: Start Docker services
    progress.report({ message: 'Starting Docker services...' })
    await ensureDockerRunning(context)  // ← ADD THIS

    // Existing code
    progress.report({ message: 'Checking PostgreSQL...' })
    await ensurePostgresAvailable()
    // ... rest unchanged
  })
}
```

#### Integration Point 2: runFirstTimeSetup()

```typescript
async function runFirstTimeSetup(context, workspaceRoot) {
  const provider = await runSetupWizard(context)
  if (!provider) return

  await ensureDockerRunning(context)  // ← ADD THIS
  await runInitialWorkspaceScan(context, workspaceRoot)
  await startWatchProcesses(context, workspaceRoot)
}
```

### Docker Compose Configuration

- **File Location**: `packages/vscode-maproom/config/docker-compose.yml`
- **Packaging**: Already included in VSIX via `.vscodeignore` exclusion rules
- **Runtime Resolution**: Extension resolves via `context.extensionPath`
- **Verification**: DockerManager uses `path.join(extensionRoot, 'config', 'docker-compose.yml')`

### Error Handling

- **Reuse DockerManager errors**: All error messages already implemented in VSMAP-1001
- **User-friendly notifications**: Add VSCode notifications with actionable buttons
- **Platform-specific actions**: "Open Docker Desktop" uses platform-specific URLs
  - macOS: `docker://` URI scheme
  - Windows: `docker://` URI scheme
  - Linux: Show command to start Docker daemon

## Implementation Notes

### What NOT to Change

- **DockerManager implementation** (VSMAP-1001) - Already tested and production-ready, do not modify
- **Health check logic** - Already in DockerManager with 30s timeout and exponential backoff
- **Docker Compose configuration** - Already bundled in extension, working correctly
- **ProcessOrchestrator** - Already fixed in commit 58ed3ba6, do not modify

### Multi-Workspace Behavior

Docker Compose is naturally idempotent and handles multiple callers:
- **Shared containers**: All VSCode workspaces use the same PostgreSQL and MCP server containers
- **Safe to call multiple times**: DockerManager.ensureServicesRunning() detects existing containers
- **Cleanup**: Each workspace registers cleanup handler, but containers stop when last workspace closes
- **No special handling needed**: Docker Compose handles reference counting automatically

### Testing Strategy

**Unit Tests** (~300 lines):
- Mock DockerManager to verify correct call sequence
- Test error notification buttons trigger correct actions
- Verify cleanup handlers registered in context.subscriptions
- Test progress messages appear in correct order

**Manual Tests** (5 scenarios, ~30 minutes):
1. Fresh install: Setup wizard → Docker starts → Initial scan
2. Docker not running: Error notification → Actionable recovery
3. Services already running: Idempotent behavior (no errors)
4. Deactivation: Containers stop, no orphans
5. Multiple workspaces: Shared containers, proper cleanup

## Dependencies

### Prerequisites (All Complete)
- ✅ **VSMAP-1001** (Nov 16): DockerManager class with full Docker Compose orchestration
- ✅ **VSMAP-1003** (Nov 16): ProcessOrchestrator for watch process management
- ✅ **MCPINIT-1001** (Nov 23): MCPConfigWriter for cline_mcp_settings.json
- ✅ **MCPINIT-1002** (Nov 23): SetupWizard for first-time setup flow

### External Requirements
- **Docker Desktop 24.0+**: Must be installed (already documented in README)
- **docker-compose.yml**: Already bundled with extension at `config/docker-compose.yml`
- **PostgreSQL image**: postgres:16-alpine (automatically pulled by Docker Compose)
- **Available ports**: 5432 (PostgreSQL), 3000 (MCP server) - conflicts handled by DockerManager

## Risk Assessment

### Risk 1: Port conflicts (5432, 3000)
- **Probability**: Medium (users may have existing PostgreSQL installations)
- **Impact**: Medium (extension fails to start, but with clear error message)
- **Mitigation**: DockerManager surfaces Docker Compose error with port information
- **Test Plan**: Manually occupy ports 5432 and 3000, verify error message is clear and actionable

### Risk 2: Health check timeout on slow machines
- **Probability**: Low (30s timeout is generous for PostgreSQL startup)
- **Impact**: Medium (first activation is slow, may appear frozen)
- **Mitigation**: Progress messages keep user informed; 30s timeout with exponential backoff already in DockerManager
- **Test Plan**: Manual verification on slow VM or throttled environment

### Risk 3: Zombie containers after VSCode crash
- **Probability**: Low (VSCode cleanup handlers are reliable)
- **Impact**: Low (containers remain running, consume resources)
- **Mitigation**: Docker Compose handles cleanup automatically; containers stop when no client connected
- **Test Plan**: Kill VSCode process forcefully, verify containers eventually stop

### Risk 4: Docker Desktop not installed
- **Probability**: Low (documented prerequisite)
- **Impact**: High (extension completely non-functional)
- **Mitigation**: Early detection with clear error message directing to Docker Desktop download
- **Test Plan**: Test on system without Docker, verify error message includes download link

## Manual Test Checklist

### Scenario 1: Fresh Install (Happy Path)
- [ ] Install extension in clean VSCode workspace
- [ ] Open workspace → Setup wizard appears
- [ ] Select embedding provider → Docker starts automatically
- [ ] Progress notifications: "Starting Docker services..." → "Checking PostgreSQL..." → "Running initial scan..."
- [ ] Status bar shows: "Watching: X files"
- [ ] No errors in Output panel
- [ ] Verify containers running: `docker ps` shows postgres and MCP server

### Scenario 2: Docker Not Running (Error Handling)
- [ ] Stop Docker Desktop completely
- [ ] Reload VSCode window → Error notification appears
- [ ] Error message: "Maproom requires Docker Desktop to be running."
- [ ] Click "Open Docker Desktop" → Docker Desktop launches
- [ ] Start Docker Desktop manually
- [ ] Click "Retry" button → Extension initializes successfully
- [ ] Status bar shows: "Watching: X files"

### Scenario 3: Services Already Running (Idempotency)
- [ ] Manually start containers: `cd packages/vscode-maproom/config && docker compose up -d`
- [ ] Verify containers running: `docker ps`
- [ ] Activate extension → No errors in Output panel
- [ ] Extension initializes successfully
- [ ] Verify no duplicate containers: `docker ps` shows only one postgres, one MCP server
- [ ] Status bar shows: "Watching: X files"

### Scenario 4: Extension Deactivation (Cleanup)
- [ ] Extension running with active watch processes
- [ ] Disable extension via Extensions panel
- [ ] Wait 5 seconds
- [ ] Verify containers stopped: `docker ps` shows no maproom containers
- [ ] No orphaned containers: `docker ps -a | grep maproom` shows nothing running

### Scenario 5: Multiple Workspaces (Container Sharing)
- [ ] Open workspace A → Containers start
- [ ] Verify containers running: `docker ps`
- [ ] Open workspace B in new VSCode window → Detects existing containers
- [ ] No errors in Output panel for workspace B
- [ ] Verify still only one set of containers: `docker ps` shows one postgres, one MCP server
- [ ] Close workspace A → Containers remain running (workspace B still open)
- [ ] Close workspace B → Containers stop (no active workspaces)
- [ ] Verify all stopped: `docker ps` shows no maproom containers

## Time Estimate

### Breakdown
- **Implementation**: 30 minutes (~50 lines of production code)
  - ensureDockerRunning() function: 15 minutes
  - Integration points (initializeServices, runFirstTimeSetup): 10 minutes
  - Error handling and notifications: 5 minutes
- **Unit tests**: 1 hour (~300 lines of test code)
  - ensureDockerRunning() test suite: 30 minutes
  - Integration flow tests: 20 minutes
  - Error handling tests: 10 minutes
- **Manual testing**: 30 minutes (5 scenarios)
  - Fresh install: 5 minutes
  - Docker not running: 5 minutes
  - Services already running: 5 minutes
  - Deactivation: 5 minutes
  - Multiple workspaces: 10 minutes
- **Documentation**: 30 minutes (README, CHANGELOG)
  - README System Requirements: 5 minutes
  - README Troubleshooting section: 15 minutes
  - CHANGELOG entry: 10 minutes

**Total**: 2.5 hours focused work

## Success Criteria

### Must Have (Blocking)
- Extension starts Docker automatically without manual `npx` commands
- Watch processes start successfully after Docker containers are ready
- Clear, actionable error message when Docker not running
- Containers stop gracefully on extension deactivation
- All unit tests passing with >90% coverage of new code
- No regressions in existing tests

### Should Have (Non-blocking)
- Users report "it just works" experience (no manual intervention)
- Zero manual Docker or daemon commands needed for typical usage
- Error messages guide users to successful recovery
- Multi-workspace scenarios work smoothly with shared containers

### Nice to Have (Future Work)
- Docker Desktop installation detection and one-click install (future enhancement)
- Automatic port conflict resolution (use alternative ports if 5432/3000 occupied)
- Health check progress feedback (show PostgreSQL initialization progress)

## Related Documentation

- **Project README**: `/workspace/.crewchief/projects/DOCKERUP_auto-container-startup/README.md`
- **Implementation Plan**: `/workspace/.crewchief/projects/DOCKERUP_auto-container-startup/planning/plan.md` (lines 81-203)
- **Quality Strategy**: `/workspace/.crewchief/projects/DOCKERUP_auto-container-startup/planning/quality-strategy.md`
- **DockerManager Source**: `packages/vscode-maproom/src/docker/manager.ts` (VSMAP-1001, Nov 16)
- **Architecture Doc**: `/workspace/.crewchief/projects/DOCKERUP_auto-container-startup/planning/architecture.md`
- **Extension Activation Flow**: `packages/vscode-maproom/src/extension.ts` (existing implementation)

## Planning References

- `/workspace/.crewchief/projects/DOCKERUP_auto-container-startup/planning/plan.md` - Phase 1 implementation details (lines 81-203)
- `/workspace/.crewchief/projects/DOCKERUP_auto-container-startup/planning/architecture.md` - System integration design
- `/workspace/.crewchief/projects/DOCKERUP_auto-container-startup/planning/quality-strategy.md` - Testing approach
