# Quality Strategy: VSCode Extension Daemon Migration

## Testing Philosophy

This project modernizes existing functionality rather than building new features. The quality strategy focuses on:

1. **Preventing regressions** - Existing extension behavior must not break
2. **Verifying new flows** - Ollama model management, single watch process, reconciliation
3. **Confidence over coverage** - Test critical paths, not every line
4. **Leveraging existing infrastructure** - Reuse test utilities and patterns

## Test Pyramid

```
                    ┌───────────┐
                    │  Manual   │ 5 scenarios
                    │  Testing  │
                   ─┴───────────┴─
                 ┌─────────────────┐
                 │   Integration   │ 4 tests
                 │     Tests       │
               ─┬┴─────────────────┴┬─
              ┌─┴───────────────────┴──┐
              │      Unit Tests        │ 14 tests
              └────────────────────────┘
```

## Unit Tests (14 tests)

### BranchSwitchedEvent Tests (`process/events.test.ts`)

| Test | What it Verifies |
|------|------------------|
| `isWatchEvent returns true for valid branch_switched` | New event type recognized |
| `isWatchEvent returns false for invalid branch_switched` | Rejects malformed events |

### OllamaClient Tests (`ollama/client.test.ts`)

| Test | What it Verifies |
|------|------------------|
| `isRunning returns true when Ollama responds` | HTTP connection check works |
| `isRunning returns false on connection error` | Graceful failure handling |
| `hasModel returns true for existing model` | Model detection from API response |
| `hasModel returns false for missing model` | Correct negative detection |
| `pullModel streams progress events` | NDJSON parsing during pull |

### ProcessOrchestrator Tests (`process/orchestrator.test.ts`)

| Test | What it Verifies |
|------|------------------|
| `startWatching spawns single watch process` | Single process, correct flags |
| `stop sends SIGTERM then SIGKILL` | Graceful shutdown sequence |
| `emits branch_switched events` | New event type emitted correctly |

### Reconciliation Tests (`process/reconcile.test.ts`)

| Test | What it Verifies |
|------|------------------|
| `reconcileChanges skips on first run` | No last commit → no reconciliation |
| `reconcileChanges skips when HEAD unchanged` | Same commit → no work |
| `reconcileChanges upserts changed files` | Spawns upsert with correct args |
| `reconcileChanges updates workspace state` | Last commit stored after success |

## Integration Tests (4 tests)

### Extension Activation (`extension.integration.test.ts`)

| Test | What it Verifies |
|------|------------------|
| `activation completes under 500ms` | Performance requirement met |
| `status bar shows "watching" after init` | Full flow completes |

### Reconciliation Flow (`reconcile.integration.test.ts`)

| Test | What it Verifies |
|------|------------------|
| `reconciliation indexes changed files` | git diff + upsert works end-to-end |
| `reconciliation handles empty diff` | No changed files → no upsert |

## Manual Testing Checklist (5 scenarios)

### Scenario 1: Fresh Install

**Setup**: Clean machine with Ollama installed, no existing index

**Steps**:
1. Install extension
2. Open workspace folder
3. Observe setup wizard appears
4. Select "Ollama" provider
5. Wait for model pull (if needed)
6. Verify status bar shows "Watching"
7. Make a code change
8. Verify file is indexed (check output channel)

**Pass Criteria**:
- [ ] Setup wizard appears on first run
- [ ] Model pull shows progress notification
- [ ] Status bar transitions: Starting → Watching
- [ ] File changes trigger indexing events

### Scenario 2: Returning User with Offline Changes

**Setup**: Extension previously configured, SQLite index exists

**Steps**:
1. Close VSCode
2. Make file changes outside VSCode (edit a .ts file)
3. Reopen VSCode
4. Observe startup reconciliation in output channel
5. Verify changed files are re-indexed

**Pass Criteria**:
- [ ] Startup reconciliation runs automatically
- [ ] Only changed files are indexed (not full re-scan)
- [ ] Watch mode begins after reconciliation

### Scenario 3: Ollama Not Running

**Setup**: Ollama not started, extension configured for ollama provider

**Steps**:
1. Ensure Ollama is not running (`pkill ollama`)
2. Open VSCode with workspace
3. Observe error notification
4. Click "Install Ollama" or "Start Ollama" button
5. Start Ollama
6. Retry (command palette: "Maproom: Setup")

**Pass Criteria**:
- [ ] Error shown with actionable button
- [ ] Link to https://ollama.ai works
- [ ] Retry after starting Ollama succeeds

### Scenario 4: Model Missing

**Setup**: Ollama running, `nomic-embed-text` not downloaded

**Steps**:
1. Remove model: `ollama rm nomic-embed-text`
2. Open VSCode with workspace
3. Observe progress notification for model download
4. Wait for download to complete
5. Verify status bar shows "Watching"

**Pass Criteria**:
- [ ] Progress notification appears
- [ ] Download progress updates shown
- [ ] Watch starts after download completes

### Scenario 5: Branch Switch

**Setup**: Extension watching, on `main` branch

**Steps**:
1. Verify status bar shows current branch
2. Switch to different branch: `git checkout -b test-branch`
3. Observe status bar update
4. Switch back: `git checkout main`
5. Verify status bar reflects main

**Pass Criteria**:
- [ ] Status bar shows branch name
- [ ] Branch switch detected automatically
- [ ] Status bar updates to new branch

## Critical Paths

These paths MUST work correctly:

### Path 1: Happy Path Activation
```
activate() → check provider → ensure Ollama → ensure model → reconcile → start watch → watching
```

### Path 2: Ollama Not Running
```
activate() → check provider → ensure Ollama → FAIL → show error with link → user starts Ollama → retry
```

### Path 3: Model Missing
```
activate() → ensure Ollama → has model? NO → pull model (progress) → reconcile → start watch
```

### Path 4: Watch Process Crash
```
watching → crash → CrashRecovery backoff → auto-restart → back to watching (or error after 5 retries)
```

### Path 5: Reconciliation
```
activate() → get last commit → git diff → changed files? YES → upsert → update state → start watch
```

## What NOT to Test

- Docker functionality (being removed)
- PostgreSQL connections (being removed)
- MCP server integration (separate component)
- Rust binary internals (tested separately)
- `branch-watch` command (deprecated)

## Test Infrastructure

### Mocking Strategy

| Component | Mock Strategy |
|-----------|---------------|
| Ollama API | HTTP mock server (nock or msw) |
| Watch process | Fake child process with controlled stdout |
| VSCode API | @vscode/test-electron or manual mocks |
| Git commands | Mock exec() to return controlled output |
| File system | memfs or temp directories |

### Test Utilities

Reuse existing test utilities:
- `packages/vscode-maproom/src/test/` - Existing test infrastructure
- Mock factories for VSCode context, secrets, etc.
- Existing `StdoutParser` tests as reference

## Acceptance Criteria

### Functional Requirements

- [ ] **F1**: Extension spawns single `watch` process (not `watch` + `branch-watch`)
- [ ] **F2**: No Docker containers are started by the extension
- [ ] **F3**: Ollama model is pulled automatically if missing (ollama provider)
- [ ] **F4**: Startup reconciliation runs before watch starts
- [ ] **F5**: Extension activates in < 500ms (sync portion)
- [ ] **F6**: `branch_switched` events update status bar

### Non-Functional Requirements

- [ ] **NF1**: No TypeScript compilation errors
- [ ] **NF2**: All unit tests pass
- [ ] **NF3**: All integration tests pass
- [ ] **NF4**: No console errors during normal operation
- [ ] **NF5**: Memory usage stable (no leaks)

### Cleanup Requirements

- [ ] **C1**: `src/docker/` directory removed
- [ ] **C2**: `src/services/postgres-checker.ts` removed
- [ ] **C3**: PostgreSQL settings removed from `package.json`
- [ ] **C4**: No references to Docker in code
- [ ] **C5**: No references to `branch-watch` command

## Risk-Based Testing Focus

| Risk | Test Focus |
|------|------------|
| `branch_switched` event new | Unit test for type guard, integration test for status bar |
| Ollama integration is new | Heavy testing of OllamaClient (5 tests) |
| ProcessOrchestrator refactor | Verify single process spawn, event routing |
| Startup reconciliation is new | Unit tests + integration with git repo |
| Removal might leave dangling code | Grep verification for Docker, PostgreSQL, branch-watch |

## Definition of Done

A ticket is complete when:
1. Implementation matches architecture
2. Unit tests pass
3. Integration tests pass (if applicable)
4. Manual testing passes (for user-facing changes)
5. No TypeScript errors
6. Code reviewed for:
   - Reuse of existing components (StdoutParser, CrashRecovery)
   - No Docker/PostgreSQL/branch-watch references
   - Proper error handling
