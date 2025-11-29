# DOCKERUP: Automatic Docker Container Startup

**Status**: ✅ Complete (Archived 2025-11-26)
**Slug**: `DOCKERUP`
**Timeline**: 2-3 hours focused work
**Complexity**: Trivial (integration only)

## Problem Statement

The VSCode Maproom extension has **all infrastructure built but not connected**:

**What Exists** ✅:
- `DockerManager` class (VSMAP-1001, Nov 16) - Starts/stops Docker Compose services
- `ProcessOrchestrator` (VSMAP-1003) - Spawns watch processes
- `MCPConfigWriter` (MCPINIT-1001, Nov 23) - Writes MCP config
- `SetupWizard` (MCPINIT-1002) - Provider selection UI

**What's Missing** ❌:
- **Integration**: DockerManager never called in activation flow
- **Sequencing**: Extension tries to start watch processes before Docker ready
- **User Experience**: Manual `npx @crewchief/maproom-mcp setup` required

**User Report**:
```
Starting watch processes...
ERROR: DATABASE_URL env var is required
```

Extension skipped Docker startup, went straight to watch processes → failed.

## Solution

**Wire existing DockerManager into extension activation flow** (~50 lines of code):

```typescript
// NEW FUNCTION
async function ensureDockerRunning(context: vscode.ExtensionContext) {
  const dockerManager = new DockerManager(outputChannel)
  await dockerManager.ensureServicesRunning()  // ← Already implemented!
  context.subscriptions.push({ dispose: () => dockerManager.stop() })
}

// CALL IN ACTIVATION
async function initializeServices(context, workspaceRoot) {
  await ensureDockerRunning(context)  // ← NEW LINE
  await ensurePostgresAvailable()     // ← Existing
  await startWatchProcesses()         // ← Existing
}
```

**That's it.** All heavy lifting already done in VSMAP-1001.

## Key Insights

### Insight 1: Zero New Infrastructure Needed

**From analysis.md**:
> "This is a **trivial integration task** masquerading as a project. The hard work was already done in VSMAP-1001 (DockerManager implementation)."

**Evidence**:
- DockerManager: 14,041 bytes, tested, production-ready
- ProcessOrchestrator: Tested, needs DATABASE_URL (fixed in commit 58ed3ba6)
- All components individually verified

### Insight 2: Simple Sequencing Fix

**From architecture.md**:
> "The only missing piece: Calling `DockerManager.ensureServicesRunning()` in activation flow."

**Current Flow** (Broken):
```
activate() → ensurePostgresAvailable() → FAILS (Docker not started)
```

**Fixed Flow**:
```
activate() → ensureDockerRunning() → ensurePostgresAvailable() → startWatchProcesses()
```

### Insight 3: Documentation vs Reality Gap

**README claims**:
- "Docker Integration - Managed PostgreSQL, Ollama, and MCP services with **zero manual setup**"
- "Docker services start in the background"

**Reality**:
- User must run: `npx @crewchief/maproom-mcp setup --provider=openai`
- Extension fails if containers not manually started

**This project closes the gap**: Make code match documentation promise.

## Scope

### In Scope (MVP)

✅ **Integration Code** (~50 lines):
- `ensureDockerRunning()` function (~20 lines)
- Call in `initializeServices()` (+2 lines)
- Call in `runFirstTimeSetup()` (+2 lines)
- Error handling (reuse existing DockerManager errors)

✅ **Tests** (~300 lines):
- Unit tests for `ensureDockerRunning()`
- Flow tests for activation sequencing
- Manual verification checklist

✅ **Documentation Updates**:
- README: Docker Desktop requirement
- README: Troubleshooting section
- CHANGELOG: Automatic startup feature

### Out of Scope

❌ **New Docker Features** (already implemented):
- Health checks → DockerManager handles
- Error messages → DockerManager provides
- Cleanup on deactivation → DockerManager implements

❌ **Modifications to Existing Components**:
- DockerManager → Keep as-is (tested)
- ProcessOrchestrator → Keep as-is (fixed in commit 58ed3ba6)
- MCPConfigWriter → Keep as-is (tested)

❌ **Enterprise Features** (post-MVP):
- Custom PostgreSQL passwords
- Configurable ports
- Remote PostgreSQL support
- SQLite embedded option

## Architecture

### Integration Point

**File**: `packages/vscode-maproom/src/extension.ts`

**What Changes**:
```typescript
// Before (line 232-306):
async function initializeServices(context, workspaceRoot) {
  await ensurePostgresAvailable()  // ← Fails if Docker not started
  await startWatchProcesses()
}

// After:
async function initializeServices(context, workspaceRoot) {
  await ensureDockerRunning(context)  // ← NEW
  await ensurePostgresAvailable()     // ← Now succeeds
  await startWatchProcesses()
}
```

**What Doesn't Change**:
- DockerManager implementation (VSMAP-1001)
- Health check logic (already in DockerManager)
- Error handling (DockerManager provides messages)
- Cleanup logic (DockerManager.stop() already exists)

### Component Reuse

```
ensureDockerRunning()  ← NEW (~20 lines)
    │
    ↓ calls
DockerManager.ensureServicesRunning()  ← EXISTING (VSMAP-1001)
    │
    ↓ spawns
docker compose up -d  ← EXISTING (docker-compose.yml bundled)
    │
    ↓ health checks
PostgreSQL + MCP server  ← EXISTING (DockerManager logic)
```

**Complexity**: Trivial (function call chain, no new logic)

## Quality Strategy

### Testing Approach

**Philosophy**: Test the ~50 lines of new integration code, not the existing components

**Unit Tests** (80% effort):
- `ensureDockerRunning()` success path
- `ensureDockerRunning()` error handling (Docker not running)
- Cleanup registration (dispose calls stop)
- Activation flow sequencing (Docker before PostgreSQL)

**Integration Tests** (15% effort):
- Full flow: Activate → Docker starts → Watch starts
- Error flow: Docker not running → Clear error
- Cleanup flow: Deactivate → Containers stop

**Manual Tests** (5% effort):
- Fresh install → Setup wizard → Docker auto-starts
- Docker not running → Error with recovery instructions
- Services already running → Idempotent (no duplicates)

**Coverage Target**: >90% of new code (only ~50 lines to test)

### Risk Mitigation

**High-Risk Areas** (Must Test):
1. ✅ Docker startup sequencing (before PostgreSQL check)
2. ✅ Error handling (Docker not running)
3. ✅ Cleanup (containers stop on deactivation)
4. ✅ Idempotency (no duplicate containers)

**Low-Risk Areas** (Existing, Tested):
- DockerManager health checks → Already tested (VSMAP-1002)
- Docker Compose spawning → Already tested
- Error message quality → Already implemented

## Security Review

**Security Impact**: **Neutral** (reusing secure components)

**What's New**:
- Calling `DockerManager.ensureServicesRunning()` in activation
- No new file operations
- No new command construction
- No new credential handling

**What's Unchanged**:
- Docker socket access (standard pattern)
- PostgreSQL default password (documented limitation)
- Container isolation (Docker namespaces)
- Network binding (localhost only)

**Verdict**: ✅ Safe to ship (no new security surface)

## Implementation Plan

### Single Ticket Execution

**Agent**: `vscode-extension-specialist`

**Steps**:
1. Implement `ensureDockerRunning()` function
2. Add call in `initializeServices()`
3. Add call in `runFirstTimeSetup()`
4. Write unit tests
5. Run manual test checklist
6. Update documentation

**Estimated Time**: 2-3 hours

**Workflow**:
```
vscode-extension-specialist (implements)
  ↓
unit-test-runner (executes tests)
  ↓
verify-ticket (checks acceptance criteria)
  ↓
commit-ticket (creates commit)
```

### Acceptance Criteria

**Functional**:
- [ ] Extension with Docker running → Watch processes start
- [ ] Extension without Docker → Clear error shown
- [ ] Setup wizard → Docker starts automatically
- [ ] Deactivation → Containers stop gracefully

**Quality**:
- [ ] Unit tests: >90% coverage of new code
- [ ] Manual tests: All scenarios passing
- [ ] No regressions: Existing tests still pass

**Documentation**:
- [ ] README updated with Docker requirement
- [ ] Troubleshooting section added
- [ ] CHANGELOG entry created

## Success Metrics

### MVP Success (Objective)

**Must Have**:
- [ ] Extension starts Docker automatically
- [ ] Watch processes start after Docker ready
- [ ] Clear error when Docker not running
- [ ] Containers stop on deactivation
- [ ] All unit tests passing

### MVP Success (Subjective)

**Should Have**:
- [ ] Users report "it just works"
- [ ] Zero manual `npx` commands needed
- [ ] Error messages guide recovery

### Post-Release (1 month)

- GitHub issues related to Docker startup: <3
- Support requests for setup: <5
- User sentiment: Positive

## Related Work

### VSMAP Project (Foundation)

**Milestone 1.1: Docker Manager** (Nov 16):
- VSMAP-1001: DockerManager implementation ✅
- VSMAP-1002: DockerManager tests ✅

**What Exists**:
- `src/docker/manager.ts` - 14,041 bytes
- `src/docker/manager.test.ts` - 6,318 bytes
- Health checks, error handling, cleanup all implemented

**What Was Missing**: Calling ensureServicesRunning() in activation

### MCPINIT Project (MCP Config)

**Tickets** (Nov 23):
- MCPINIT-1001: MCPConfigWriter ✅
- MCPINIT-1002: Setup wizard integration ✅

**What Exists**:
- `src/config/mcp-writer.ts` - Writes `.vscode/mcp.json`
- `src/ui/setupWizard.ts` - Provider selection UI

**What Was Assumed**: Docker would be handled separately (gap)

### DOCKERUP Project (This)

**Purpose**: Close the gap between VSMAP and MCPINIT
**Scope**: ~50 lines of integration code
**Result**: Complete end-to-end automation

## Next Steps

1. **Create Ticket**: Run `/create-project-tickets DOCKERUP`
2. **Execute**: Run `/work-on-project DOCKERUP` (2-3 hours)
3. **Verify**: Manual test checklist
4. **Ship**: Version bump, publish to marketplace

## Project Metadata

**Created**: 2025-01-24
**Priority**: High (eliminates #1 onboarding friction)
**Risk Level**: Minimal (reusing tested components)
**Estimated Effort**: 2-3 hours focused work

**Key Decision**: This is an integration task, not a feature build. All infrastructure exists.

---

## Planning Documents

- [Analysis](planning/analysis.md) - Problem definition, existing components inventory
- [Architecture](planning/architecture.md) - Integration design, component reuse
- [Quality Strategy](planning/quality-strategy.md) - Testing approach, risk mitigation
- [Security Review](planning/security-review.md) - Security impact assessment
- [Plan](planning/plan.md) - Implementation details, timeline, acceptance criteria

---

*This project follows the CrewChief ticket-based workflow. See `.crewchief/README.md` for workflow details.*
