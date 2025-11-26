# Analysis: Automatic Docker Container Startup for VSCode Extension

## Problem Definition

### Current State

The VSCode Maproom extension has **all the infrastructure built but not connected**:

**Infrastructure That Exists ✅**:
- `DockerManager` class (implemented in VSMAP-1001, Nov 16)
  - Starts/stops Docker Compose services
  - Health check monitoring for PostgreSQL and MCP server
  - Error handling for "Docker not running" scenarios
  - Located at `packages/vscode-maproom/src/docker/manager.ts` (14,041 bytes)

- `MCPConfigWriter` class (implemented in MCPINIT-1001, Nov 23)
  - Writes `.vscode/mcp.json` to register MCP server
  - Handles provider-specific environment variables
  - Located at `packages/vscode-maproom/src/config/mcp-writer.ts`

- `ProcessOrchestrator` class (implemented in VSMAP-1003)
  - Spawns `crewchief-maproom watch` processes
  - Manages process lifecycle with crash recovery
  - Located at `packages/vscode-maproom/src/process/orchestrator.ts`

**What's Missing ❌**:
- **Integration**: DockerManager is never called in the activation flow
- **Sequencing**: Extension tries to start watch processes before containers are running
- **Error Recovery**: No automatic retry when Docker isn't ready

**Evidence from User Report**:
```
Starting watch processes...
[2025-11-24T04:02:48.095Z] ERROR: Error: DATABASE_URL env var is required
```

The extension skipped Docker startup and went straight to spawning watch processes, which failed because PostgreSQL wasn't running.

### Root Cause

**File**: `packages/vscode-maproom/src/extension.ts`

**Current Flow**:
```
activate()
  ↓
checkPostgresAvailable() → Fails silently or shows error
  ↓
startWatchProcesses() → Fails because DB not ready
```

**Missing**:
```
ensureDockerRunning() ← NOT CALLED
```

The `initializeServices()` function (line 232-306) calls `ensurePostgresAvailable()` but **never** calls `DockerManager.ensureServicesRunning()`.

### Why This Happened

**Project History**:
1. **VSMAP project** (Nov 16): Implemented DockerManager and all infrastructure
2. **MCPINIT project** (Nov 23): Focused on MCP config writing, assumed Docker would be handled
3. **Gap**: No ticket explicitly wired DockerManager into extension activation

**Architectural Confusion**:
The MCPINIT project README states:
> "Extension writes config, VS Code invokes CLI, CLI manages lifecycle"

This is **partially correct** for MCP server lifecycle (VS Code manages it), but **wrong** for PostgreSQL. The extension **must** start PostgreSQL via Docker Compose because:
- PostgreSQL is required for watch processes (not just MCP)
- Watch processes need DATABASE_URL pointing to running PostgreSQL
- MCP server also needs PostgreSQL running

### User Expectations

**From README** (`packages/vscode-maproom/README.md` lines 10, 72):
- "Docker Integration - Managed PostgreSQL, Ollama, and MCP services with **zero manual setup**"
- "Docker services start in the background"

**Current Reality**:
- User must manually run `npx @crewchief/maproom-mcp setup --provider=openai`
- Extension fails if containers aren't already running

**Gap**: Documentation promises automation but code requires manual intervention.

## Existing Solutions Analysis

### Industry Patterns

**VS Code Extensions with Docker**:
1. **Docker Extension** (Microsoft)
   - Manages containers via `dockerode` library
   - Direct API communication, no CLI spawning
   - **Not applicable**: Too heavyweight for our use case

2. **Remote - Containers Extension**
   - Spawns `docker compose` via `child_process.spawn()`
   - Health checks via polling
   - **Applicable**: This matches our DockerManager implementation

3. **Dev Containers CLI**
   - Feature detection: checks if Docker daemon is running before operations
   - Clear error messages with recovery instructions
   - **Applicable**: Good pattern for error handling

**MCP Server Lifecycle** (from VS Code MCP docs):
- Extensions register servers via `.vscode/mcp.json`
- VS Code MCP client handles spawning and lifecycle
- Extensions **don't manage** MCP server processes directly
- **Our situation**: Correct for MCP server, but PostgreSQL is different

### Maproom MCP CLI Analysis

**Command**: `npx @crewchief/maproom-mcp setup --provider=openai`

**What it does** (from `packages/maproom-mcp/bin/cli.cjs`):
1. Validates Docker is running
2. Writes `docker-compose.yml` to `~/.config/crewchief/maproom/`
3. Runs `docker compose up -d`
4. Waits for PostgreSQL health check
5. Runs database migrations
6. Writes MCP config to `.vscode/mcp.json` or `~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json`

**What we can reuse**:
- ✅ Docker Compose file location strategy
- ✅ Health check polling logic
- ✅ Migration running strategy

**What we must NOT duplicate**:
- ❌ CLI has 1,972 lines - too much complexity
- ❌ Global MCP config writing (we use workspace `.vscode/mcp.json`)
- ❌ Interactive prompts (we have setup wizard)

### Current Project Implementation

**DockerManager** (`src/docker/manager.ts`):
```typescript
async ensureServicesRunning(): Promise<void>
  - Spawns: docker compose -f ${composeFile} up -d
  - Health checks: pg_isready for PostgreSQL
  - Health checks: TCP ping for MCP server (port 3000)
  - Timeout: 30s with exponential backoff
```

**Key Features Already Implemented**:
- ✅ Idempotent: No-op if services already running
- ✅ Error handling: Clear messages if Docker not installed/running
- ✅ Logging: All operations logged to Output panel
- ✅ Graceful shutdown: `stop()` method for cleanup

**What's Perfect**:
This implementation is production-ready. We just need to **call it**.

## Current Project State

### Code Inventory

**Completed Components** (from VSMAP and MCPINIT):
- `src/docker/manager.ts` - Docker lifecycle (VSMAP-1001) ✅
- `src/process/orchestrator.ts` - Process spawning (VSMAP-1003) ✅
- `src/ui/statusBar.ts` - Status display (VSMAP-1005) ✅
- `src/config/mcp-writer.ts` - MCP registration (MCPINIT-1001) ✅
- `src/ui/setupWizard.ts` - Provider setup (MCPINIT-1002) ✅

**Activation Flow** (`src/extension.ts`):
- Line 79: `activate()` entry point
- Line 154: Provider check
- Line 158: Calls `initializeServices()` OR `runFirstTimeSetup()`
- Line 203: `ensurePostgresAvailable()` ← Checks but doesn't start
- Line 247: `ensurePostgresAvailable()` ← Same in initializeServices
- **Missing**: `DockerManager.ensureServicesRunning()` call

### Integration Gap

**File**: `packages/vscode-maproom/src/extension.ts`

**Current**: Lines 232-306 (`initializeServices` function)
```typescript
async function initializeServices(context, workspaceRoot) {
  await vscode.window.withProgress({...}, async (progress) => {
    // Step 1: Check PostgreSQL availability
    progress.report({ message: 'Checking PostgreSQL...' })
    await ensurePostgresAvailable() // ← Only checks, doesn't start!

    // Step 2: Create process orchestrator
    // ... fails if PostgreSQL not running
  })
}
```

**Needed**: Add Docker startup BEFORE PostgreSQL check
```typescript
async function initializeServices(context, workspaceRoot) {
  await vscode.window.withProgress({...}, async (progress) => {
    // NEW Step 1: Start Docker services
    progress.report({ message: 'Starting Docker services...' })
    await ensureDockerRunning(context) // ← NEW FUNCTION

    // Step 2: Check PostgreSQL availability
    progress.report({ message: 'Checking PostgreSQL...' })
    await ensurePostgresAvailable()

    // Step 3: Create process orchestrator
    // ... now succeeds!
  })
}
```

### Dependencies Already Satisfied

**From VSMAP planning**:
- ✅ Docker Compose file: `packages/maproom-mcp/config/docker-compose.yml` exists
- ✅ Health check utilities: Implemented in DockerManager
- ✅ Error handling: Clear messages in DockerManager
- ✅ Process spawning: Child process handling tested

**No New Infrastructure Needed**: This is pure integration work.

## Research Findings

### Docker Compose File Location

**Current**: `packages/maproom-mcp/config/docker-compose.yml`

**Options**:
1. **Bundle with extension**: Copy to extension's `config/` directory
   - Pro: Self-contained, version controlled with extension
   - Pro: Extension controls exact service configuration
   - Con: Duplication across packages

2. **Reference from maproom-mcp**: Use existing file
   - Pro: Single source of truth
   - Pro: CLI and extension use same configuration
   - Con: Tight coupling between packages

**Decision**: Bundle with extension (Option 1)
- Extensions should be self-contained for distribution
- Allows extension-specific PostgreSQL configuration (e.g., port binding)
- Matches VSMAP-1001 implementation plan

### Health Check Strategy

**PostgreSQL**:
```bash
docker exec maproom-postgres pg_isready -U maproom
# Returns: accepting connections (success)
```

**MCP Server**:
```bash
nc -zv localhost 3000
# Or: TCP connection attempt
```

**Timing**:
- PostgreSQL: ~5-10s on first start (image pull + init)
- PostgreSQL: ~2-3s on subsequent starts
- MCP Server: ~3-5s (Node.js startup)

**Backoff Strategy** (already in DockerManager):
- Attempts: 1s, 2s, 4s, 8s, 16s (max 30s total)
- User feedback: Progress notification shows "Waiting for services..."

### Error Scenarios

**Docker Not Running**:
```
Error: spawn docker ENOENT
→ Show: "Docker Desktop must be running to use Maproom. Please start Docker Desktop and try again."
```

**Docker Compose Failed**:
```
Error: docker compose exited with code 1
→ Show detailed error from stderr
→ Suggest: Check Docker Desktop, port conflicts
```

**Health Check Timeout**:
```
Error: PostgreSQL not ready after 30 seconds
→ Show: "Database startup timed out. Check Docker Desktop logs."
→ Provide: Command to view logs (docker compose logs)
```

**All Handled**: DockerManager already implements these error cases (VSMAP-1001 lines 80-120).

## Approach Evaluation

### Option 1: Call DockerManager Directly (Recommended)

**Implementation**:
```typescript
// In extension.ts
import { DockerManager } from './docker/manager'

async function ensureDockerRunning(context: vscode.ExtensionContext): Promise<void> {
  const dockerManager = new DockerManager(outputChannel)

  // Start services (idempotent)
  await dockerManager.ensureServicesRunning()

  // Store for cleanup
  context.subscriptions.push({
    dispose: () => dockerManager.stop()
  })
}
```

**Pros**:
- ✅ Reuses 100% of existing DockerManager code
- ✅ No duplication
- ✅ Minimal integration code (~20 lines)
- ✅ Tested and verified (VSMAP-1001)

**Cons**:
- None identified

**Complexity**: Trivial (20 lines of integration code)

### Option 2: Invoke CLI `setup` Command (Not Recommended)

**Implementation**:
```typescript
// Spawn: npx @crewchief/maproom-mcp setup --provider=openai
const child = spawn('npx', ['@crewchief/maproom-mcp', 'setup', `--provider=${provider}`])
```

**Pros**:
- ✅ Delegates to battle-tested CLI (1,972 lines)
- ✅ Includes migration running

**Cons**:
- ❌ Duplicates work (DockerManager already exists)
- ❌ Slower (npx overhead, full setup flow)
- ❌ Interactive prompts conflict with extension UI
- ❌ Global vs workspace MCP config mismatch
- ❌ Extension loses control over Docker lifecycle

**Complexity**: Medium (process management, output parsing)

**Verdict**: Not recommended. We already built the right solution (DockerManager).

### Option 3: Hybrid Approach (Over-engineered)

**Implementation**:
- Use DockerManager for PostgreSQL
- Invoke CLI for migrations only

**Verdict**: Unnecessary complexity. DockerManager + existing migration code is sufficient.

## Key Insights

### Insight 1: All Infrastructure Exists

The VSMAP and MCPINIT projects built everything needed:
- DockerManager (Nov 16) - production ready
- MCPConfigWriter (Nov 23) - working correctly
- ProcessOrchestrator - spawns watch processes

**The only missing piece**: Calling `DockerManager.ensureServicesRunning()` in activation flow.

### Insight 2: Simple Integration Problem

This is **not** a new feature requiring architecture. It's a **10-line integration fix**:

```typescript
// Before:
await ensurePostgresAvailable()

// After:
await ensureDockerRunning(context) // ← NEW (calls DockerManager)
await ensurePostgresAvailable()
```

### Insight 3: User Experience Gap

**What users expect** (from README):
- Install extension → Click activate → Everything works

**What currently happens**:
- Install extension → Error "PostgreSQL not running" → Manual `npx` command → Reload extension

**Fix impact**: Eliminates all manual setup steps (except Docker Desktop installation).

### Insight 4: No New Complexity

**Before this fix**:
- DockerManager exists but unused (dead code)
- Extension fails with confusing error
- Users manually run CLI setup

**After this fix**:
- DockerManager used (no new code, just calling existing method)
- Extension starts services automatically
- Users have zero manual steps

**Complexity delta**: Net reduction (removes manual workaround need).

## Conclusion

This is a **trivial integration task** masquerading as a project. The hard work was already done in VSMAP-1001 (DockerManager implementation).

**Scope**: ~50 lines of integration code across 2 files
**Complexity**: Low (function calls, no new logic)
**Risk**: Minimal (reusing tested components)
**Value**: High (eliminates #1 onboarding friction)

**Recommendation**: Execute as a single focused ticket, not a full project. However, following user request to "avoid duplicating planning," creating minimal project structure with clear references to VSMAP work.
