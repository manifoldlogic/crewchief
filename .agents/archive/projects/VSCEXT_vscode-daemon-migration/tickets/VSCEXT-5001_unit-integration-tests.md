# Ticket: VSCEXT-5001: Unit and integration tests

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (412 tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Write and execute comprehensive unit and integration tests for all new and modified components. This ensures the migration is solid before manual testing.

## Background
The migration introduced new components (OllamaClient, reconciliation) and modified existing ones (ProcessOrchestrator, StatusBarManager). Each needs tests to prevent regressions and verify correct behavior.

Reference: planning/plan.md - Phase 5, Ticket 5001
Reference: planning/quality-strategy.md - Test Pyramid

## Acceptance Criteria
- [x] All unit tests pass (412 tests across 18 files, exceeds target of 14)
- [x] Integration tests pass (8 tests in integration.test.ts, exceeds target of 2)
- [x] No TypeScript compilation errors
- [x] Test coverage for critical paths

## Technical Requirements

### Unit Tests (14 tests)

**BranchSwitchedEvent Tests** (`process/events.test.ts`):
| Test | What it Verifies |
|------|------------------|
| `isWatchEvent returns true for valid branch_switched` | New event type recognized |
| `isWatchEvent returns false for invalid branch_switched` | Rejects malformed events |

**OllamaClient Tests** (`ollama/client.test.ts`):
| Test | What it Verifies |
|------|------------------|
| `isRunning returns true when Ollama responds` | HTTP connection check works |
| `isRunning returns false on connection error` | Graceful failure handling |
| `hasModel returns true for existing model` | Model detection from API response |
| `hasModel returns false for missing model` | Correct negative detection |
| `pullModel streams progress events` | NDJSON parsing during pull |

**ProcessOrchestrator Tests** (`process/orchestrator.test.ts`):
| Test | What it Verifies |
|------|------------------|
| `startWatching spawns single watch process` | Single process, correct flags |
| `stop sends SIGTERM then SIGKILL` | Graceful shutdown sequence |
| `emits branch_switched events` | New event type emitted correctly |

**Reconciliation Tests** (`process/reconcile.test.ts`):
| Test | What it Verifies |
|------|------------------|
| `reconcileChanges skips on first run` | No last commit → no reconciliation |
| `reconcileChanges skips when HEAD unchanged` | Same commit → no work |
| `reconcileChanges upserts changed files` | Spawns upsert with correct args |
| `reconcileChanges updates workspace state` | Last commit stored after success |

### Integration Tests (2 tests)

**Extension Activation** (`extension.integration.test.ts`):
| Test | What it Verifies |
|------|------------------|
| `activation completes under 500ms` | Performance requirement met |
| `status bar shows "watching" after init` | Full flow completes |

## Implementation Notes

### Mocking Strategy

| Component | Mock Strategy |
|-----------|---------------|
| Ollama API | HTTP mock (nock or msw) |
| Watch process | Fake child process with controlled stdout |
| VSCode API | @vscode/test-electron or manual mocks |
| Git commands | Mock exec() to return controlled output |
| File system | memfs or temp directories |

### Test Utilities
Reuse existing test infrastructure in `packages/vscode-maproom/src/test/`

### Running Tests
```bash
cd packages/vscode-maproom
pnpm test        # Unit tests
pnpm test:e2e    # Integration tests (if separate)
```

## Dependencies
- VSCEXT-4002 (All implementation and cleanup complete)

## Risk Assessment
- **Risk**: Tests flaky due to timing
  - **Mitigation**: Use proper async/await, avoid setTimeout where possible
- **Risk**: Integration tests require real VSCode instance
  - **Mitigation**: Use @vscode/test-electron for proper isolation

## Files/Packages Affected
- `packages/vscode-maproom/src/process/events.test.ts` - BranchSwitchedEvent tests
- `packages/vscode-maproom/src/ollama/client.test.ts` - OllamaClient tests
- `packages/vscode-maproom/src/process/orchestrator.test.ts` - Update orchestrator tests
- `packages/vscode-maproom/src/process/reconcile.test.ts` - Reconciliation tests
- `packages/vscode-maproom/src/extension.integration.test.ts` - Integration tests
