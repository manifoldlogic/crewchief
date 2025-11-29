# Implementation Plan: VSCode Extension Daemon Migration

## Project Overview

Migrate the VSCode extension to use:
1. Single unified `watch` process (refactored ProcessOrchestrator)
2. Host Ollama with automatic model management
3. SQLite-only database (remove Docker/PostgreSQL dependency)
4. TypeScript-based startup reconciliation using existing `upsert` command

## Phase Overview

| Phase | Description | Tickets | Agent |
|-------|-------------|---------|-------|
| 1 | Event Types & Ollama Client | 3 | vscode-extension-specialist |
| 2 | ProcessOrchestrator Refactor | 2 | vscode-extension-specialist |
| 3 | Extension Flow Update | 3 | vscode-extension-specialist |
| 4 | Cleanup | 2 | vscode-extension-specialist |
| 5 | Testing & Verification | 2 | unit-test-runner, verify-ticket |

**Total: ~12 tickets**

---

## Phase 1: Event Types & Ollama Client

**Goal**: Add missing event type and create Ollama management client.

### Ticket 1001: Add BranchSwitchedEvent to events.ts

**Description**: Add the `branch_switched` event type that the unified watch command emits.

**Implementation**:
1. Add `BranchSwitchedEvent` interface to `src/process/events.ts`
2. Update `WatchEvent` union type to include `BranchSwitchedEvent`
3. Add case to `isWatchEvent()` type guard
4. Add unit tests for the new event type validation

**Event Schema** (from Rust binary):
```typescript
interface BranchSwitchedEvent {
  type: 'branch_switched'
  timestamp: string
  repo: string
  old_branch: string
  new_branch: string
  old_worktree_id: number
  new_worktree_id: number
  worktree_created: boolean
}
```

**Acceptance Criteria**:
- [ ] `BranchSwitchedEvent` interface defined
- [ ] `WatchEvent` union includes new type
- [ ] `isWatchEvent()` validates branch_switched events
- [ ] Unit tests for valid and invalid branch_switched events

### Ticket 1002: Implement OllamaClient class

**Description**: HTTP client for Ollama API operations, building on existing `detectOllama()`.

**Implementation**:
1. Create `src/ollama/client.ts`
2. Implement `isRunning()` (extends pattern from `setupWizard.ts:detectOllama()`)
3. Implement `hasModel(name)` via `/api/tags` endpoint
4. Implement `pullModel(name, onProgress)` with NDJSON streaming
5. Hardcode localhost:11434 for security (not configurable)
6. Add model name validation regex

**Acceptance Criteria**:
- [ ] Hardcoded to localhost:11434 (security requirement)
- [ ] `isRunning()` detects running Ollama (2s timeout)
- [ ] `hasModel()` checks model existence via API
- [ ] `pullModel()` streams progress via callback
- [ ] Model name validated with regex
- [ ] Unit tests with mocked HTTP

### Ticket 1003: Implement model management flow

**Description**: Ensure required model is available before watch starts.

**Implementation**:
```typescript
// src/ollama/model-manager.ts
export async function ensureOllamaModel(modelName: string): Promise<void> {
  const client = new OllamaClient()

  if (!await client.isRunning()) {
    throw new OllamaNotRunningError()
  }

  if (!await client.hasModel(modelName)) {
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Downloading embedding model...',
      cancellable: false,
    }, async (progress) => {
      await client.pullModel(modelName, (status) => {
        progress.report({ message: status })
      })
    })
  }
}
```

**Error Handling**:
- `OllamaNotRunningError` → Show "Install Ollama" button with link
- Network error → Show retry button
- User cancellation → Graceful abort

**Acceptance Criteria**:
- [ ] Shows VSCode progress notification during pull
- [ ] Handles Ollama not running with helpful error
- [ ] Handles network errors with retry
- [ ] Skips pull if model exists

---

## Phase 2: ProcessOrchestrator Refactor

**Goal**: Refactor ProcessOrchestrator to spawn single unified watch.

### Ticket 2001: Refactor ProcessOrchestrator for single watch

**Description**: Update ProcessOrchestrator to spawn single `watch` process instead of `watch` + `branch-watch`.

**Changes**:
1. Remove `branch-watch` process spawning
2. Update `startWatching()` to spawn single watch with `--path` flag
3. Add handling for `branch_switched` events
4. Remove dual-process coordination logic
5. Keep existing: `StdoutParser`, `CrashRecovery`, platform detection

**Watch Invocation**:
```typescript
spawn(binaryPath, ['watch', '--path', workspaceRoot], {
  env: {
    MAPROOM_DATABASE_URL: `sqlite://${databasePath}`,
    MAPROOM_EMBEDDING_PROVIDER: provider,
  }
})
```

**Acceptance Criteria**:
- [ ] Spawns single watch process (not watch + branch-watch)
- [ ] Uses verified CLI flags: `--path`, optional `--repo`, `--throttle`
- [ ] Parses branch_switched events correctly
- [ ] Reuses existing StdoutParser and CrashRecovery
- [ ] Clean shutdown on extension deactivation

### Ticket 2002: Update StatusBarManager integration

**Description**: Connect status bar to refactored ProcessOrchestrator.

**Changes**:
- Handle `branch_switched` events to update branch display
- Show reconciliation status during startup
- Keep existing state machine (starting → watching → error)

**Acceptance Criteria**:
- [ ] Shows current branch from branch_switched events
- [ ] Shows "Reconciling..." during startup reconciliation
- [ ] Transitions to "Watching" after ready
- [ ] Error states display correctly

---

## Phase 3: Extension Flow Update

**Goal**: Update extension.ts with new activation flow including reconciliation.

### Ticket 3001: Implement startup reconciliation

**Description**: TypeScript-based reconciliation that runs before watch starts.

**Implementation** (`src/process/reconcile.ts`):
```typescript
export async function reconcileChanges(context: ExtensionContext): Promise<void> {
  const workspaceRoot = getWorkspaceRoot()
  const repoName = await getRepoName(workspaceRoot)
  const branchName = await getBranchName(workspaceRoot)

  // 1. Get last indexed commit (stored in extension state)
  const lastCommit = context.workspaceState.get<string>('maproom.lastIndexedCommit')

  if (!lastCommit) {
    // First run - skip reconciliation, watch will handle it
    return
  }

  // 2. Get current HEAD
  const headCommit = await exec('git rev-parse HEAD', { cwd: workspaceRoot })

  if (lastCommit === headCommit.trim()) {
    // No changes since last run
    return
  }

  // 3. Get changed files
  const diffResult = await exec(
    `git diff --name-only ${lastCommit}..HEAD`,
    { cwd: workspaceRoot }
  )
  const changedFiles = diffResult.split('\n').filter(Boolean)

  if (changedFiles.length === 0) {
    return
  }

  // 4. Run upsert for changed files
  await spawnUpsert(changedFiles, repoName, branchName, workspaceRoot, headCommit)

  // 5. Update last indexed commit
  await context.workspaceState.update('maproom.lastIndexedCommit', headCommit)
}
```

**Acceptance Criteria**:
- [ ] Reads last indexed commit from workspace state
- [ ] Uses `git diff --name-only` to find changed files
- [ ] Spawns `crewchief-maproom upsert` with correct arguments
- [ ] Updates last indexed commit after success
- [ ] Gracefully handles first run (no last commit)

### Ticket 3002: Rewrite extension activation

**Description**: New activation flow without Docker, with Ollama and reconciliation.

**New Flow**:
```
activate() → fast sync setup → return
  ↓ (background)
Check provider → Ensure Ollama model → Reconcile → Start watch → Ready
```

**Changes to extension.ts**:
1. Remove `ensureDockerRunning()` calls
2. Add `ensureOllamaModel()` call (only for ollama provider)
3. Add `reconcileChanges()` call before watch
4. Update orchestrator instantiation

**Acceptance Criteria**:
- [ ] No Docker containers started
- [ ] Ollama model checked/pulled before watch (ollama provider only)
- [ ] Reconciliation runs before watch starts
- [ ] Activation completes < 500ms (sync portion)
- [ ] Background init shows progress

### Ticket 3003: Update setup wizard

**Description**: Simplify setup wizard for SQLite + Ollama flow.

**Changes**:
- Remove PostgreSQL provider references (if any)
- Preserve existing `showNoSqliteGuidance()` for first run
- Simplify to: select provider → validate → save

**Acceptance Criteria**:
- [ ] Setup wizard works for Ollama/OpenAI/Google
- [ ] No Docker or PostgreSQL references
- [ ] First run shows SQLite guidance (existing behavior)
- [ ] Re-run setup works correctly

---

## Phase 4: Cleanup

**Goal**: Remove deprecated code and settings.

### Ticket 4001: Remove Docker code

**Description**: Delete all Docker-related code.

**Files to Delete**:
- `src/docker/manager.ts`
- `src/docker/index.ts`
- `src/docker/example-usage.ts`
- `src/docker/manager.test.ts`

**Files to Update**:
- Remove imports/references in `extension.ts`
- Remove from `src/services/index.ts` (if exists)

**Acceptance Criteria**:
- [ ] `src/docker/` directory deleted
- [ ] No imports from docker module
- [ ] No Docker references in code
- [ ] TypeScript compiles without errors

### Ticket 4002: Remove PostgreSQL code and settings

**Description**: Delete PostgreSQL-specific code and settings.

**Files to Delete**:
- `src/services/postgres-checker.ts`

**Settings to Remove from package.json**:
- `maproom.database.provider`
- `maproom.database.host`
- `maproom.database.port`
- `maproom.database.user`
- `maproom.database.password`
- `maproom.database.name`

**Acceptance Criteria**:
- [ ] postgres-checker.ts deleted
- [ ] PostgreSQL settings removed from package.json
- [ ] No PostgreSQL references in code
- [ ] SQLite remains as only database option

---

## Phase 5: Testing & Verification

**Goal**: Ensure quality and correctness.

### Ticket 5001: Unit and integration tests

**Description**: Write tests for new and modified components.

**Tests to Add**:
- `BranchSwitchedEvent` validation (2 tests: valid, invalid)
- `OllamaClient` unit tests (5 tests)
- `ProcessOrchestrator` single watch tests (2 tests)
- `reconcileChanges()` tests (3 tests)

**Test Count**:
- Unit tests: ~12 tests
- Integration tests: 2 tests (activation, reconciliation)

**Acceptance Criteria**:
- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] No TypeScript errors

### Ticket 5002: Manual testing and verification

**Description**: Execute manual test scenarios.

**Scenarios**:
1. **Fresh install** - Full flow with model pull
2. **Returning user with offline changes** - Reconciliation works
3. **Ollama not running** - Error handling with install link
4. **Model missing** - Auto-pull with progress
5. **Branch switch** - Status bar updates

**Acceptance Criteria**:
- [ ] All scenarios pass
- [ ] No regressions from previous functionality
- [ ] Performance meets requirements (< 500ms activation)

---

## Dependencies

### External (Stable)
- Ollama HTTP API (localhost:11434)
- SQLite (via Rust binary, `crewchief-maproom`)
- VSCode Extension API
- Git CLI (for `git diff`, `git rev-parse`)

### Internal (This Project Creates)
- Phase 1 → Phase 2 (BranchSwitchedEvent needed for orchestrator)
- Phase 1 → Phase 3 (OllamaClient needed for activation)
- Phase 2 → Phase 3 (Refactored orchestrator needed for activation)

### Execution Order

```
Phase 1 (Events + Ollama) ──▶ Phase 2 (Orchestrator) ──▶ Phase 3 (Extension Flow) ──▶ Phase 4 (Cleanup) ──▶ Phase 5 (Testing)
```

Phases must be sequential due to dependencies.

---

## Reuse Summary

### Components Preserved (No Changes)

| Component | Reason |
|-----------|--------|
| `StdoutParser` | Already handles NDJSON correctly |
| `CrashRecovery` | Already implements backoff |
| `SecretsManager` | API key storage works as-is |
| `showNoSqliteGuidance()` | First-run UX preserved |

### Components Refactored

| Component | Change |
|-----------|--------|
| `ProcessOrchestrator` | Remove branch-watch, add single watch |
| `detectOllama()` | Pattern extracted to OllamaClient |

### Components Added

| Component | Purpose |
|-----------|---------|
| `BranchSwitchedEvent` | New event type for unified watch |
| `OllamaClient` | Model management |
| `reconcileChanges()` | Startup reconciliation |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Watch CLI changes | CLI verified: `--path`, `--repo`, `--throttle` |
| SQLite URL format | Documented: `sqlite://` prefix works |
| Reconciliation timing | Run before watch, use workspace state |
| Model pull slow | Progress notification, cancellable |
| First run edge case | Skip reconciliation, show guidance |

---

## Success Metrics

1. **Activation time**: < 500ms (sync portion)
2. **Single process**: Only one `crewchief-maproom` process running
3. **No Docker**: `docker ps` shows no maproom containers
4. **Model auto-pull**: New users get model automatically (Ollama)
5. **Offline changes indexed**: Changes made outside VSCode are caught up
6. **Branch display**: Status bar shows current branch

---

## Agent Assignments

| Agent | Phases | Role |
|-------|--------|------|
| **vscode-extension-specialist** | 1, 2, 3, 4 | All TypeScript work |
| **unit-test-runner** | 5 | Execute test suites |
| **verify-ticket** | 5 | Final verification |
| **commit-ticket** | All | Create commits |
