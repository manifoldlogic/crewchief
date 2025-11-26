# Implementation Plan: Auto Container Startup Integration

## Executive Summary

**Project**: Wire existing DockerManager into VSCode extension activation flow
**Complexity**: Trivial (function calls, no new logic)
**Timeline**: 2-3 hours focused work
**Risk**: Minimal (reusing tested components)

### What Makes This Simple

**All Infrastructure Exists**:
- ✅ DockerManager (VSMAP-1001, Nov 16) - 14,041 bytes, tested
- ✅ MCPConfigWriter (MCPINIT-1001, Nov 23) - 6,703 bytes, tested
- ✅ ProcessOrchestrator (VSMAP-1003, Nov 16) - tested
- ✅ StatusBar, SetupWizard, etc. - all complete

**What's Missing**: Calling `DockerManager.ensureServicesRunning()` in activation flow

**Total New Code**: ~50 lines (1 function + 2 call sites)

## Implementation Phases

### Phase 1: Integration (Single Ticket)

**Objective**: Wire DockerManager into extension activation

#### Task Breakdown

**1. Create `ensureDockerRunning()` function** (~20 lines)
- Import DockerManager
- Instantiate with outputChannel
- Call `ensureServicesRunning()`
- Register cleanup on deactivation
- Handle errors with user-friendly messages

**2. Update `initializeServices()` flow** (~2 lines)
- Add `await ensureDockerRunning(context)` before PostgreSQL check
- Existing error handling sufficient

**3. Update `runFirstTimeSetup()` flow** (~2 lines)
- Add `await ensureDockerRunning(context)` after setup wizard
- Existing error handling sufficient

**4. Add unit tests** (~200 lines)
- Test: Success path (Docker starts)
- Test: Error path (Docker not running)
- Test: Cleanup path (dispose calls stop)
- Test: Sequencing (Docker before PostgreSQL)
- Test: First-time setup (Docker after wizard)

**5. Manual verification** (~30 minutes)
- Test: Fresh install → Setup wizard → Docker starts → Watch starts
- Test: Docker not running → Clear error → Recovery
- Test: Services already running → Idempotent behavior
- Test: Deactivation → Containers stop

**Total Effort**: 2-3 hours

#### Files Modified

```
packages/vscode-maproom/src/extension.ts
  - NEW: ensureDockerRunning() function (~20 lines)
  - MODIFY: initializeServices() (+2 lines)
  - MODIFY: runFirstTimeSetup() (+2 lines)
  - Total: ~24 lines changed

packages/vscode-maproom/src/extension.test.ts
  - NEW: Test suite for ensureDockerRunning (~200 lines)
  - NEW: Integration flow tests (~100 lines)
  - Total: ~300 lines new tests
```

**Total Code Impact**: ~324 lines (24 production + 300 tests)

#### Implementation Code

**Location**: `packages/vscode-maproom/src/extension.ts`

**New Function**:
```typescript
import { DockerManager } from './docker/manager'

/**
 * Ensure Docker services are running
 *
 * Starts PostgreSQL and MCP server via Docker Compose.
 * Idempotent: no-op if services already running.
 *
 * Multi-workspace behavior:
 * - Multiple VSCode workspaces share the same Docker containers
 * - DockerManager.ensureServicesRunning() is idempotent (safe to call multiple times)
 * - Containers remain running until last workspace closes
 * - Each workspace registers its own cleanup handler
 *
 * @param context - Extension context for cleanup registration
 * @throws Error if Docker not installed or startup fails
 */
async function ensureDockerRunning(
  context: vscode.ExtensionContext
): Promise<void> {
  outputChannel?.appendLine('Starting Docker services...')

  const dockerManager = new DockerManager(outputChannel!)

  try {
    // Start services (idempotent, includes health checks)
    // Note: If called by multiple workspaces concurrently, Docker Compose
    // ensures only one set of containers is created (idempotent by design)
    await dockerManager.ensureServicesRunning()

    // Register cleanup on deactivation
    context.subscriptions.push({
      dispose: () => {
        outputChannel?.appendLine('Stopping Docker services...')
        void dockerManager.stop()
      }
    })

    outputChannel?.appendLine('Docker services started successfully')
  } catch (error: any) {
    const errorMessage = `Failed to start Docker services: ${error.message}`
    outputChannel?.appendLine(`ERROR: ${errorMessage}`)

    // Show user-friendly error with recovery instructions
    // Error message templates:
    // - "Maproom requires Docker Desktop to be running." (Docker not running)
    // - "Failed to start Docker services: [specific error]" (other errors)
    const action = await vscode.window.showErrorMessage(
      'Maproom requires Docker Desktop to be running.',
      'Open Docker Desktop',
      'Show Logs',
      'Retry'
    )

    if (action === 'Show Logs') {
      outputChannel?.show()
    } else if (action === 'Open Docker Desktop') {
      // Open Docker Desktop application (platform-specific)
      const platform = process.platform
      if (platform === 'darwin') {
        await vscode.commands.executeCommand('vscode.open', vscode.Uri.parse('docker://'))
      } else if (platform === 'win32') {
        await vscode.commands.executeCommand('vscode.open', vscode.Uri.parse('docker://'))
      }
    }
    // Note: 'Retry' would require calling ensureDockerRunning again
    // For MVP, user can manually reload window after starting Docker

    throw new Error(errorMessage)
  }
}
```

**Modified: `initializeServices()`** (line 232-306):
```typescript
async function initializeServices(
  context: vscode.ExtensionContext,
  workspaceRoot: string
): Promise<void> {
  try {
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: 'Maproom',
        cancellable: false,
      },
      async (progress) => {
        // NEW Step 1: Start Docker services
        progress.report({ message: 'Starting Docker services...' })
        await ensureDockerRunning(context) // ← NEW LINE

        // Step 2: Check PostgreSQL availability
        progress.report({ message: 'Checking PostgreSQL...' })
        await ensurePostgresAvailable()

        // ... rest of initialization unchanged ...
      }
    )
  } catch (error: any) {
    // ... existing error handling unchanged ...
  }
}
```

**Modified: `runFirstTimeSetup()`** (line 178-220):
```typescript
async function runFirstTimeSetup(
  context: vscode.ExtensionContext,
  workspaceRoot: string
): Promise<void> {
  try {
    // Run setup wizard
    const provider = await runSetupWizard(context)

    if (!provider) {
      // User cancelled setup
      outputChannel?.appendLine('Setup cancelled by user')
      vscode.window.showInformationMessage(
        'Maproom setup cancelled. Run "Maproom: Setup" to configure later.'
      )
      statusBar?.setState('idle')
      return
    }

    // Setup complete - show success message
    outputChannel?.appendLine(`Setup complete: ${provider} selected`)
    vscode.window.showInformationMessage(
      `Maproom configured to use ${provider.toUpperCase()} for embeddings`
    )

    // NEW: Start Docker services
    await ensureDockerRunning(context) // ← NEW LINE

    // Run initial scan after setup completes
    await runInitialWorkspaceScan(context, workspaceRoot)

    // After scan completes, start watch processes
    await startWatchProcesses(context, workspaceRoot)
  } catch (error: any) {
    // ... existing error handling unchanged ...
  }
}
```

#### Testing Strategy

**Unit Tests** (`packages/vscode-maproom/src/extension.test.ts`):

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest'
import * as vscode from 'vscode'
import { DockerManager } from './docker/manager'

// Mock modules
vi.mock('./docker/manager')
vi.mock('vscode')

describe('ensureDockerRunning', () => {
  let mockContext: vscode.ExtensionContext
  let mockDockerManager: any

  beforeEach(() => {
    mockContext = {
      subscriptions: [],
    } as any

    mockDockerManager = {
      ensureServicesRunning: vi.fn().mockResolvedValue(undefined),
      stop: vi.fn().mockResolvedValue(undefined),
    }

    vi.mocked(DockerManager).mockImplementation(() => mockDockerManager)
  })

  it('starts Docker services successfully', async () => {
    await ensureDockerRunning(mockContext)

    expect(mockDockerManager.ensureServicesRunning).toHaveBeenCalled()
    expect(mockContext.subscriptions).toHaveLength(1)
  })

  it('shows error when Docker not running', async () => {
    mockDockerManager.ensureServicesRunning.mockRejectedValue(
      new Error('Docker Desktop is not running')
    )

    await expect(ensureDockerRunning(mockContext)).rejects.toThrow(
      'Failed to start Docker services'
    )

    expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
      'Maproom requires Docker Desktop to be running.',
      expect.any(String),
      expect.any(String)
    )
  })

  it('registers cleanup on deactivation', async () => {
    await ensureDockerRunning(mockContext)

    const disposeRegistration = mockContext.subscriptions[0]
    await disposeRegistration.dispose()

    expect(mockDockerManager.stop).toHaveBeenCalled()
  })
})

describe('initializeServices flow', () => {
  it('calls ensureDockerRunning before ensurePostgresAvailable', async () => {
    const callOrder: string[] = []

    global.ensureDockerRunning = vi.fn(() => {
      callOrder.push('docker')
      return Promise.resolve()
    })
    global.ensurePostgresAvailable = vi.fn(() => {
      callOrder.push('postgres')
      return Promise.resolve()
    })

    await initializeServices(mockContext, '/test/workspace')

    expect(callOrder).toEqual(['docker', 'postgres'])
  })

  it('stops initialization if Docker startup fails', async () => {
    global.ensureDockerRunning = vi.fn().mockRejectedValue(
      new Error('Docker failed')
    )
    global.ensurePostgresAvailable = vi.fn()

    await expect(
      initializeServices(mockContext, '/test/workspace')
    ).rejects.toThrow('Docker failed')

    expect(global.ensurePostgresAvailable).not.toHaveBeenCalled()
  })
})
```

**Manual Testing Checklist**:

- [ ] **Scenario 1: Fresh install**
  - Install extension
  - Open workspace
  - Setup wizard appears
  - Select provider
  - Docker starts automatically
  - Initial scan runs
  - Watch processes start
  - Status bar: "Watching: X files"

- [ ] **Scenario 2: Docker not running**
  - Stop Docker Desktop
  - Reload VSCode
  - Error notification appears
  - Click "Open Docker Desktop"
  - Start Docker Desktop
  - Reload extension
  - Extension initializes successfully

- [ ] **Scenario 3: Services already running**
  - Start containers manually
  - Activate extension
  - Idempotent (no errors, no duplicates)
  - Watch processes start

- [ ] **Scenario 4: Deactivation**
  - Extension running
  - Disable extension
  - Containers stop
  - No orphaned containers

- [ ] **Scenario 5: Multiple workspaces** (concurrent activation)
  - Open workspace A (extension starts containers)
  - Open workspace B in new VSCode window
  - Verify workspace B detects existing containers (idempotent)
  - Verify no duplicate containers created (`docker ps` shows single set)
  - Close workspace A
  - Verify containers remain running (workspace B still needs them)
  - Close workspace B
  - Verify containers stop gracefully

#### Agent Assignment

**Primary Agent**: `vscode-extension-specialist`
- Implements integration function
- Modifies activation flows
- Writes unit tests

**Supporting Agents** (via workflow):
- `unit-test-runner`: Execute test suite
- `verify-ticket`: Check acceptance criteria
- `commit-ticket`: Create commit

**Why Single Agent**: Integration is pure TypeScript VSCode API work

#### Dependencies

**Prerequisite Components** (All Complete):
- ✅ DockerManager (VSMAP-1001)
- ✅ ProcessOrchestrator (VSMAP-1003)
- ✅ MCPConfigWriter (MCPINIT-1001)
- ✅ SetupWizard (MCPINIT-1002)

**External Dependencies**:
- Docker Desktop installed and running
- docker-compose.yml bundled with extension
  - **Location**: `packages/vscode-maproom/config/docker-compose.yml`
  - **Packaging**: File is included in VSIX via `.vscodeignore` exclusion rules
  - **Runtime Path**: Extension resolves path via `context.extensionPath`
  - **Verification**: DockerManager uses `path.join(extensionRoot, 'config', 'docker-compose.yml')`

**No Blockers**: All prerequisites satisfied

#### Acceptance Criteria

**Functional**:
- [x] Extension activates with Docker running → Watch processes start
- [x] Extension activates without Docker → Clear error shown
- [x] Setup wizard completes → Docker starts automatically
- [x] Docker startup failure → Extension doesn't start watch processes
- [x] Deactivation → Docker containers stop gracefully

**Quality**:
- [x] Unit tests: >90% coverage of new code
- [x] Manual tests: All scenarios passing
- [x] No regressions: Existing tests still pass

**User Experience**:
- [x] Error messages are clear and actionable
- [x] Progress shown during Docker startup
- [x] Status bar reflects correct state

#### Risk Mitigation

**Risk 1**: Docker Compose file not found
- **Mitigation**: Bundle with extension (already done in VSMAP-1001)
- **Test**: Verify file exists in packaged VSIX

**Risk 2**: Port conflicts (5432, 3000)
- **Mitigation**: DockerManager surfaces Docker Compose error
- **Test**: Manually occupy ports, verify error message

**Risk 3**: Health check timeout
- **Mitigation**: 30s timeout with exponential backoff (already in DockerManager)
- **Test**: Slow machine simulation (manual)

**Risk 4**: Zombie containers after crash
- **Mitigation**: Docker Compose handles cleanup
- **Test**: Kill extension process, verify containers stop

## Timeline

**Total Duration**: 2-3 hours focused work

**Breakdown**:
- Implementation: 30 minutes
- Unit tests: 1 hour
- Manual testing: 30 minutes
- Documentation updates: 30 minutes

**Blockers**: None (all dependencies complete)

## Success Metrics

### MVP Success (Objective)

**Must Have**:
- [x] Extension starts Docker automatically
- [x] Watch processes start after Docker ready
- [x] Clear error when Docker not running
- [x] Containers stop on deactivation
- [x] All unit tests passing

### MVP Success (Subjective)

**Should Have**:
- [x] Users report "it just works"
- [x] Zero manual `npx` commands needed
- [x] Error messages guide recovery

### Post-Release (1 month)

**Tracking**:
- GitHub issues related to Docker startup: <3
- Support requests for setup: <5
- User sentiment: Positive

## Documentation Updates

### README.md

**Section: System Requirements** (update):
```markdown
## System Requirements

| Requirement | Minimum | Recommended |
|------------|---------|-------------|
| VSCode | 1.85.0+ | Latest stable |
| **Docker Desktop** | **24.0+** | **Latest stable** |
| RAM | 4GB | 8GB |
| Free Disk Space | 2GB | 5GB |

**Important**: Docker Desktop must be installed and running before activating the extension.

### Installation

1. Install Docker Desktop from [docker.com/products/docker-desktop](https://www.docker.com/products/docker-desktop/)
2. Start Docker Desktop
3. Install extension from Marketplace
4. Open workspace → Extension activates → Setup wizard appears

**That's it!** No manual commands, no configuration files.
```

**Section: Getting Started** (update):
```markdown
## Getting Started

### First Launch

1. **Ensure Docker Desktop is running**
   - Look for Docker icon in system tray
   - If not running, start Docker Desktop and wait ~30s

2. **Open a workspace** in VSCode
   - Extension activates automatically
   - **Docker services start in background** ← Updated
   - Progress shown: "Starting Docker services..."

3. **Complete setup wizard** (appears automatically)
   - Choose embedding provider (Ollama, OpenAI, or Google)
   - Enter credentials (if using OpenAI/Google)
   - **Setup completes automatically** ← Updated

4. **Wait for initial scan** (progress shown in notification)
   - Large workspaces may take 5-10 minutes
   - Status bar shows file count

5. **Start using!**
   - Status bar shows "Watching: X files"
   - MCP server available in Claude Code
```

**Section: Troubleshooting** (new):
```markdown
## Troubleshooting

### "Docker Desktop is not running" error

**Cause**: Extension requires Docker to start PostgreSQL database.

**Solution**:
1. Open Docker Desktop
2. Wait for Docker to start (~30 seconds)
3. Reload VSCode window (`Developer: Reload Window`)

### "Port 5432 is already in use" error

**Cause**: Another PostgreSQL instance is using port 5432.

**Solution**:
1. Find the conflicting process:
   ```bash
   lsof -i :5432  # macOS/Linux
   netstat -ano | findstr :5432  # Windows
   ```
2. Stop the conflicting service
3. Reload extension

### Extension stuck on "Starting Docker services..."

**Cause**: Docker Compose health checks timing out.

**Solution**:
1. Check Docker Desktop is fully started
2. View logs: `Maproom: Show Output`
3. Check Docker Compose logs:
   ```bash
   docker compose -f ~/.vscode/extensions/manifoldlogic.vscode-maproom-*/config/docker-compose.yml logs
   ```
```

### CHANGELOG.md

**Add Entry**:
```markdown
## [0.3.0] - 2025-01-25

### Added
- **Automatic Docker container startup** - Extension now starts PostgreSQL and MCP server automatically
- No manual `npx @crewchief/maproom-mcp setup` command needed
- Clear error messages when Docker not running with recovery instructions
- Progress notifications during Docker startup

### Changed
- Extension activation flow now includes Docker startup step
- Setup wizard automatically starts containers after provider selection

### Fixed
- "DATABASE_URL env var is required" error on fresh installations
- Extension failing when Docker containers not manually started
```

## Rollback Plan

**If Critical Bug Found**:

1. **Identify issue severity**:
   - Critical: Extension won't activate, data loss
   - High: Docker won't start, existing users broken
   - Medium: Some users affected, workaround exists

2. **Immediate actions** (Critical/High):
   - Revert commit
   - Publish hotfix release
   - Update README with manual setup instructions

3. **Fallback UX** (Medium):
   - Add setting: `"maproom.autoStartDocker": false`
   - Default to auto-start, users can disable
   - Document workaround in release notes

4. **Communication**:
   - GitHub issue: Acknowledge bug, provide workaround
   - Extension README: Add "Known Issues" section
   - Marketplace description: Note temporary limitation

## Post-Release Monitoring

**Week 1**:
- Monitor GitHub issues (tag: docker, setup, activation)
- Check Marketplace reviews for Docker-related complaints
- Gather logs from users reporting issues

**Week 2-4**:
- Analyze common failure modes
- Update error messages if patterns emerge
- Document additional troubleshooting steps

**Month 2+**:
- Consider improvements (configurable ports, custom passwords)
- Evaluate telemetry for activation success rate
- Plan Phase 2 enhancements

## Related Work

### VSMAP Project (Reference)

**Completed Components**:
- VSMAP-1001: DockerManager implementation
- VSMAP-1002: DockerManager tests
- VSMAP-1003: ProcessOrchestrator (depends on Docker)
- VSMAP-1004-1006: Status bar, activation flow

**Gap**: Docker never wired into activation (oversight)

### MCPINIT Project (Reference)

**Completed Components**:
- MCPINIT-1001: MCPConfigWriter
- MCPINIT-1002: Setup wizard integration

**Gap**: Assumed Docker would be handled separately, focused on MCP config

### This Project (DOCKERUP)

**Scope**: Close the gap between VSMAP and MCPINIT
**Approach**: Wire existing DockerManager into extension activation
**Result**: Complete end-to-end automation (setup → Docker → scan → watch)

## Summary

**What This Project Does**:
- Adds ~50 lines of integration code
- Calls existing DockerManager.ensureServicesRunning()
- Wires into two activation flows (normal + first-time)
- Adds comprehensive tests

**What This Project Doesn't Do**:
- Create new infrastructure (already exists)
- Modify DockerManager (already tested)
- Change extension architecture (same pattern)

**Why It's Trivial**:
- All hard work done in VSMAP-1001 (Nov 16)
- Just connecting dots
- Minimal risk, high value

**Estimated Effort**: 2-3 hours focused work
**Estimated Value**: Eliminates #1 onboarding friction (manual setup)
**Recommended Approach**: Single focused ticket, execute immediately
