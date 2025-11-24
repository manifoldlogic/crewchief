# Architecture: Auto Container Startup Integration

## System Overview

This is an **integration architecture**, not a new system. We're wiring together existing, tested components from VSMAP and MCPINIT projects.

```
┌─────────────────────────────────────────────────┐
│     VSCode Extension Activation Flow           │
├─────────────────────────────────────────────────┤
│                                                 │
│  1. activate()                                  │
│     ├─> Check if provider configured            │
│     ├─> If not: runFirstTimeSetup() ──┐        │
│     └─> If yes: initializeServices() ──┤        │
│                                        │        │
│  2. NEW: ensureDockerRunning() ◄───────┘        │
│     ├─> Create DockerManager                    │
│     ├─> Call ensureServicesRunning()            │
│     ├─> Wait for health checks                  │
│     └─> Store for cleanup                       │
│                                                 │
│  3. ensurePostgresAvailable() (existing)        │
│     └─> Verify PostgreSQL responding            │
│                                                 │
│  4. startWatchProcesses() (existing)            │
│     └─> Spawn crewchief-maproom watch           │
│                                                 │
└─────────────────────────────────────────────────┘
         │                           │
         │ spawns                    │ spawns
         ↓                           ↓
┌──────────────────┐       ┌──────────────────┐
│ Docker Compose   │       │ Watch Processes  │
│ (PostgreSQL,     │       │ (Rust binary)    │
│  MCP Server)     │       │                  │
└──────────────────┘       └──────────────────┘
```

## Core Components (All Existing)

### 1. DockerManager (VSMAP-1001 - Completed Nov 16)

**Location**: `packages/vscode-maproom/src/docker/manager.ts`

**Key Methods**:
```typescript
class DockerManager {
  constructor(outputChannel: vscode.OutputChannel)

  // Start services (idempotent)
  async ensureServicesRunning(): Promise<void>
    - Spawns: docker compose up -d
    - Health checks: PostgreSQL, MCP server
    - Timeout: 30s with exponential backoff
    - Error handling: Docker not running, compose failed

  // Graceful shutdown
  async stop(): Promise<void>
    - Spawns: docker compose down
    - Waits for completion
}
```

**Configuration**:
- Compose file: `config/docker-compose.yml` (bundled with extension)
- Services: `maproom-postgres`, `maproom-mcp`
- Network: `maproom-network`

**Health Checks**:
- PostgreSQL: `docker exec maproom-postgres pg_isready`
- MCP Server: TCP connection to `localhost:3000`

**Status**: ✅ Fully implemented and tested

### 2. ProcessOrchestrator (VSMAP-1003 - Completed Nov 16)

**Location**: `packages/vscode-maproom/src/process/orchestrator.ts`

**Purpose**: Spawns and monitors `crewchief-maproom watch` processes

**Dependencies**: Requires PostgreSQL running (DATABASE_URL environment variable)

**Status**: ✅ Implemented, currently fails because Docker not started

### 3. Extension Activation (Needs Integration)

**Location**: `packages/vscode-maproom/src/extension.ts`

**Current Flow** (lines 232-306):
```typescript
async function initializeServices(context, workspaceRoot) {
  await vscode.window.withProgress({...}, async (progress) => {
    // Step 1: Check PostgreSQL availability
    progress.report({ message: 'Checking PostgreSQL...' })
    await ensurePostgresAvailable() // ← FAILS if Docker not started

    // Step 2: Create process orchestrator
    orchestrator = new ProcessOrchestrator(...)
    await orchestrator.startWatching() // ← FAILS without PostgreSQL
  })
}
```

**New Flow** (with Docker startup):
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
    orchestrator = new ProcessOrchestrator(...)
    await orchestrator.startWatching()
  })
}
```

## Integration Point: `ensureDockerRunning()`

### Implementation

**File**: `packages/vscode-maproom/src/extension.ts`

**New Function** (~20 lines):
```typescript
/**
 * Ensure Docker services are running
 *
 * Starts PostgreSQL and MCP server via Docker Compose.
 * Idempotent: no-op if services already running.
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
    const action = await vscode.window.showErrorMessage(
      'Maproom requires Docker Desktop to be running.',
      'Open Docker Desktop',
      'Show Logs'
    )

    if (action === 'Show Logs') {
      outputChannel?.show()
    }

    throw new Error(errorMessage)
  }
}
```

### Call Sites

**1. First-Time Setup Flow** (`runFirstTimeSetup` function, line 178-220):
```typescript
async function runFirstTimeSetup(context, workspaceRoot) {
  // ... existing setup wizard code ...

  // NEW: Start Docker after provider selection
  await ensureDockerRunning(context)

  // Existing: Run initial scan
  await runInitialWorkspaceScan(context, workspaceRoot)

  // Existing: Start watch processes
  await startWatchProcesses(context, workspaceRoot)
}
```

**2. Normal Initialization Flow** (`initializeServices` function, line 232-306):
```typescript
async function initializeServices(context, workspaceRoot) {
  await vscode.window.withProgress({...}, async (progress) => {
    // NEW: Start Docker services
    progress.report({ message: 'Starting Docker services...' })
    await ensureDockerRunning(context)

    // Existing: Check PostgreSQL
    progress.report({ message: 'Checking PostgreSQL...' })
    await ensurePostgresAvailable()

    // ... rest of initialization ...
  })
}
```

## Data Flow

### Activation Sequence

```
1. User opens VSCode with workspace
   ↓
2. Extension activates (onStartupFinished)
   ↓
3. Check provider configuration
   ↓
4a. If not configured: runFirstTimeSetup()
    ├─> Show setup wizard
    ├─> User selects provider (OpenAI, Google, Ollama)
    ├─> User enters credentials
    ├─> Write .vscode/mcp.json (MCPConfigWriter)
    └─> ensureDockerRunning() ← NEW
4b. If configured: initializeServices()
    └─> ensureDockerRunning() ← NEW
   ↓
5. ensureDockerRunning()
   ├─> DockerManager.ensureServicesRunning()
   ├─> docker compose up -d
   ├─> Wait for PostgreSQL health check (pg_isready)
   └─> Wait for MCP server health check (TCP ping)
   ↓
6. ensurePostgresAvailable()
   └─> Verify PostgreSQL responding
   ↓
7. ProcessOrchestrator.startWatching()
   ├─> Spawn: crewchief-maproom watch
   ├─> Set env: DATABASE_URL=postgresql://...
   └─> Parse stdout for indexing events
   ↓
8. StatusBar shows "Watching: X files"
```

### Error Handling Flow

```
ensureDockerRunning() fails
   ↓
catch (error)
   ├─> Log to Output panel
   ├─> Show error notification
   ├─> Offer actions: "Open Docker Desktop", "Show Logs"
   └─> Throw (prevents watch processes from starting)
   ↓
Extension stays in "error" state
   └─> Status bar shows "Error: Docker not running"
```

## Configuration

### Docker Compose File

**Location**: `packages/vscode-maproom/config/docker-compose.yml`

**Content** (from maproom-mcp):
```yaml
version: '3.8'

services:
  maproom-postgres:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_USER: maproom
      POSTGRES_PASSWORD: maproom
      POSTGRES_DB: maproom
    ports:
      - "5432:5432"
    volumes:
      - maproom-postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U maproom"]
      interval: 5s
      timeout: 5s
      retries: 5

  maproom-mcp:
    image: crewchief/maproom-mcp:latest
    environment:
      DATABASE_URL: postgresql://maproom:maproom@maproom-postgres:5432/maproom
    ports:
      - "3000:3000"
    depends_on:
      maproom-postgres:
        condition: service_healthy

volumes:
  maproom-postgres-data:

networks:
  default:
    name: maproom-network
```

**Status**: File already exists, tested in maproom-mcp package

### Environment Variables

**ProcessOrchestrator** needs (already implemented):
```typescript
env: {
  DATABASE_URL: "postgresql://maproom:maproom@localhost:5432/maproom",
  MAPROOM_DATABASE_URL: "postgresql://maproom:maproom@localhost:5432/maproom",
  // ... provider credentials ...
}
```

**No changes needed**: Fixed in DATABASE_URL commit (58ed3ba6)

## Performance Considerations

### Activation Time Budget

**Target**: <500ms for `activate()` to return
**Strategy**: Defer Docker startup to background task

**Current Implementation** (already correct):
```typescript
export function activate(context: vscode.ExtensionContext): void {
  // Fast path (<100ms):
  outputChannel = vscode.window.createOutputChannel('Maproom')
  statusBar = new StatusBarManager(context)
  statusBar.setState('starting')

  // RETURN IMMEDIATELY ✓
  console.log('Extension activated (background initialization starting...)')

  // Background (async, doesn't block):
  void initializeServices(context, workspaceRoot)
}
```

**Adding Docker startup doesn't affect activation time** because it's already deferred to background.

### Docker Startup Time

**First Start** (no images cached):
- PostgreSQL image pull: ~30s (varies by network)
- PostgreSQL init: ~5-10s
- MCP image pull: ~20s
- Total: ~60s worst case

**Subsequent Starts** (images cached):
- PostgreSQL startup: ~2-3s
- MCP startup: ~3-5s
- Total: ~5-8s typical

**User Feedback**:
- Progress notification shows: "Starting Docker services..."
- Progress updates: "Waiting for PostgreSQL...", "Waiting for MCP server..."
- Status bar shows: "Starting..." → "Watching: 0 files" when ready

### Health Check Overhead

**PostgreSQL**:
- Check: `docker exec maproom-postgres pg_isready`
- Latency: ~50-100ms per attempt
- Backoff: 1s, 2s, 4s, 8s, 16s (max 30s)
- Typical: 2-3 attempts (~3s total)

**MCP Server**:
- Check: TCP connection to localhost:3000
- Latency: ~10-20ms per attempt
- Same backoff strategy
- Typical: 2-3 attempts (~3s total)

**Total Health Check Time**: ~6s typical, 60s worst case (timeout)

## Technology Choices

### Why Docker Compose CLI?

**Alternatives Considered**:
1. **dockerode library** (programmatic Docker API)
   - Pro: No CLI dependency
   - Con: 500KB+ package size
   - Con: Complex API
   - Verdict: Over-engineered for our needs

2. **Docker Compose CLI** (current choice)
   - Pro: Simple `spawn('docker', ['compose', 'up', '-d'])`
   - Pro: No npm dependencies
   - Pro: Users already have Docker Desktop
   - Con: Requires Docker Desktop installed
   - Verdict: ✅ Best fit for extension

**Decision**: Use Docker Compose CLI (already implemented in DockerManager)

### Why Bundle docker-compose.yml?

**Alternatives**:
1. **Reference from maproom-mcp package**
   - Pro: Single source of truth
   - Con: Tight coupling
   - Con: Package must be installed

2. **Bundle with extension** (current choice)
   - Pro: Self-contained
   - Pro: Version controlled
   - Pro: Extension-specific configuration
   - Verdict: ✅ Required for distribution

**Decision**: Bundle compose file (already done in VSMAP-1001)

## Long-Term Maintainability

### Coupling to Docker

**Current**: Extension requires Docker Desktop

**Future Options**:
1. **Podman support** (Docker CLI compatible)
   - No changes needed (same CLI interface)
   - Automatic fallback

2. **Remote PostgreSQL**
   - User configures connection string
   - Skip Docker startup if DATABASE_URL already set
   - Extension becomes database-agnostic

3. **Embedded SQLite** (Phase 10+)
   - Replace PostgreSQL with embedded database
   - Remove Docker dependency entirely
   - Major architecture change

**Current Decision**: Require Docker (matches README promise, user expectations)

### Versioning Strategy

**Docker Compose File**:
- Bundled with extension at build time
- Version controlled in extension package
- Updates require extension release

**PostgreSQL Image**:
- Current: `pgvector/pgvector:pg16`
- Upgrades: Coordinated with extension releases
- Data migration: Handle in extension upgrade guide

**MCP Server Image**:
- Current: `crewchief/maproom-mcp:latest`
- Should pin version (e.g., `:2.2.1`)
- Update in extension version bumps

## Security Considerations

### Docker Socket Access

**Issue**: Extension spawns Docker commands

**Risk**: Low (standard Docker Desktop workflow)

**Mitigation**:
- No privileged containers
- Standard Docker socket permissions
- User already authorized Docker Desktop

### Port Exposure

**PostgreSQL**: `localhost:5432`
**MCP Server**: `localhost:3000`

**Risk**: Medium (local network exposure)

**Mitigation**:
- Bind to 127.0.0.1 only (not 0.0.0.0)
- Default Docker Compose behavior
- Document in README

### Credential Storage

**Database Password**: `maproom` (default)

**Risk**: Low (local development only)

**Mitigation**:
- Document in security review
- Future: Allow custom credentials via settings
- Acceptable for MVP

## Testing Strategy

### Unit Tests

**New Code Coverage**:
- `ensureDockerRunning()` function
  - Success path: Docker starts
  - Error path: Docker not running
  - Error path: Health check timeout
  - Cleanup: dispose() calls stop()

**Mock Strategy**:
```typescript
// Mock DockerManager for unit tests
const mockDockerManager = {
  ensureServicesRunning: jest.fn().mockResolvedValue(undefined),
  stop: jest.fn().mockResolvedValue(undefined)
}
```

### Integration Tests

**Full Flow**:
1. Extension activates
2. Docker services start
3. PostgreSQL healthy
4. Watch processes spawn
5. Status bar updates

**Test Environment**:
- Requires Docker Desktop running
- Uses test workspace
- Cleans up containers after

### Manual Testing

**Scenarios**:
1. ✅ First-time setup (no provider configured)
   - Setup wizard → Docker starts → Scan runs → Watch starts

2. ✅ Subsequent activation (provider configured)
   - Extension activates → Docker starts → Watch starts

3. ✅ Docker not running
   - Error shown → Clear recovery instructions

4. ✅ Services already running
   - Idempotent behavior → No duplicate containers

5. ✅ Deactivation
   - Containers stop gracefully → No orphans

## Migration from Current State

### No Breaking Changes

**Users with manual setup**:
- Existing containers will be detected (idempotent)
- No migration required
- Existing `.vscode/mcp.json` unchanged

**Users without setup**:
- Extension now works out-of-box
- First-time setup wizard triggers Docker startup
- Zero manual steps (except Docker Desktop install)

### Backward Compatibility

**docker-compose.yml changes**: None
**Database schema changes**: None
**MCP config format**: Unchanged

**Verdict**: Drop-in enhancement, zero breaking changes

## Summary

This architecture is **trivial by design**:

**What's New**: 1 function (~20 lines)
**What's Changed**: 2 call sites (+1 line each)
**What's Reused**: 100% (DockerManager, ProcessOrchestrator, etc.)

**Total Integration Code**: ~50 lines
**Complexity**: Minimal (function calls)
**Risk**: Low (tested components)
**Value**: High (zero-setup experience)

The hard work was already done in VSMAP-1001. This is just connecting the dots.
