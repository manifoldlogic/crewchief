# VSCEXT: VSCode Extension Daemon Migration

**Status**: Complete ✅
**Completed**: 2025-11-27
**Slug**: VSCEXT
**Tickets**: 12 tickets across 5 phases (all verified)

## Problem Statement

The VSCode extension (`packages/vscode-maproom`) uses an outdated architecture:

1. **Dual watch processes** - Still spawns separate `watch` and `branch-watch` processes despite the Rust `watch` command being unified
2. **Docker dependency** - Manages PostgreSQL containers when SQLite is the target local database
3. **No startup reconciliation** - Doesn't catch up on file changes made while extension was inactive
4. **No Ollama model management** - Doesn't automatically pull the embedding model

## Proposed Solution

Modernize the extension to use:

1. **Single unified `watch` process** with startup reconciliation
2. **Host Ollama** with automatic model download
3. **SQLite-only** database (remove Docker completely)
4. **Simplified activation flow** without container orchestration

## Architecture Overview

```
Extension activates
       │
       ▼
┌──────────────────┐
│ Check Ollama     │──▶ Pull model if missing
│ (localhost:11434)│
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Reconciliation   │──▶ git diff + upsert (TypeScript)
│ (catch up changes)│
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ ProcessOrchest-  │──▶ Single process: crewchief-maproom watch
│ rator (refactored)│
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ SQLite Database  │    ~/.maproom/maproom.db
└──────────────────┘
```

**Key insight**: Reconciliation is done in TypeScript using `git diff` + existing `upsert` CLI command, not in Rust. This avoids Rust changes entirely.

## Key Changes

| Component | Before | After |
|-----------|--------|-------|
| Watch processes | 2 (watch + branch-watch) | 1 (unified watch) |
| Database | PostgreSQL in Docker | SQLite local file |
| Ollama | Optional Docker container | Host Ollama (required) |
| Startup | Jump to watch | Reconcile then watch |
| Model management | Manual | Auto-pull if missing |

## Phases

| Phase | Description | Tickets |
|-------|-------------|---------|
| 1 | Event Types & Ollama Client | 3 |
| 2 | ProcessOrchestrator Refactor | 2 |
| 3 | Extension Flow Update | 3 |
| 4 | Cleanup | 2 |
| 5 | Testing & Verification | 2 |

**Note**: No Rust changes required. Reconciliation uses TypeScript `git diff` + existing `upsert` CLI.

## Relevant Agents

### Primary Implementation
- **vscode-extension-specialist** - Phases 1-4 (all TypeScript work)

### Testing & Verification
- **unit-test-runner** - Execute test suites
- **verify-ticket** - Final verification
- **commit-ticket** - Create conventional commits

## Planning Documents

| Document | Description |
|----------|-------------|
| [analysis.md](planning/analysis.md) | Problem analysis, current state, research |
| [architecture.md](planning/architecture.md) | Target architecture, design decisions |
| [quality-strategy.md](planning/quality-strategy.md) | Test strategy, acceptance criteria |
| [security-review.md](planning/security-review.md) | Security assessment (approved ✅) |
| [plan.md](planning/plan.md) | Detailed implementation plan |

## Success Criteria

- [x] Extension spawns single `watch` process
- [x] No Docker containers started
- [x] Ollama model auto-pulled if missing
- [x] Changed files indexed on startup (reconciliation)
- [x] Activation time < 500ms (background initialization pattern)
- [x] All tests pass (412 tests)
- [x] No TypeScript errors
- [x] Docker code completely removed (~1,900 lines deleted)

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Watch CLI flags change | Medium | Verified via `--help`, documented in architecture.md |
| Breaking existing users | Medium | Clear migration path, SQLite is fresh index |
| Ollama not installed | Medium | Clear error messages with install link |
| SQLite URL format | Low | Documented: `sqlite://` prefix works |

## Completion Summary

All 12 tickets completed and verified. Key accomplishments:
- Unified watch process (removed branch-watch)
- SQLite-only database support
- OllamaClient with automatic model management
- Startup reconciliation using git diff + upsert CLI
- StatusBarManager state machine integration
- ~1,900 lines of Docker/PostgreSQL code removed
- 412 tests passing across 18 test files

## Dependencies

- UNIWATCH project (completed) - Unified watch command in Rust
- SQLite backend (completed) - sqlite-vec support
- daemon-client (completed) - Reference implementation for Ollama client patterns
