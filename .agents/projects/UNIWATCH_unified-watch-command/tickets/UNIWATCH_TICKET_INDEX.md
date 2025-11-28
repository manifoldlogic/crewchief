# UNIWATCH Ticket Index

**Project:** Unified Watch Command
**Status:** Ready for Execution
**Created:** 2025-01-28

## Overview

This project adds runtime branch switch detection to the `maproom watch` command so that when users run `git checkout`, the watch command automatically detects the switch and re-indexes to the correct worktree.

## Tickets by Phase

### Phase 0: Module Exports (Prerequisite)

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [UNIWATCH-0001](UNIWATCH-0001_export-indexer-components.md) | Export Indexer Module Components | rust-indexer-engineer | Pending |

**Plan Reference:** [Phase 0 - Module Exports](../planning/plan.md#phase-0-module-exports-prerequisite)

---

### Phase 1: Dynamic Worktree State

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [UNIWATCH-1001](UNIWATCH-1001_dynamic-worktree-state.md) | Add Dynamic Worktree State Tracking | rust-indexer-engineer | Pending |

**Plan Reference:** [Phase 1 - Dynamic Worktree State](../planning/plan.md#phase-1-dynamic-worktree-state)

---

### Phase 2: HEAD Watcher Integration

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [UNIWATCH-2001](UNIWATCH-2001_head-watcher-integration.md) | Integrate HEAD Watcher into Event Loop | rust-indexer-engineer | Pending |

**Plan Reference:** [Phase 2 - HEAD Watcher Integration](../planning/plan.md#phase-2-head-watcher-integration)

---

### Phase 3: Branch Switch Handler

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [UNIWATCH-3001](UNIWATCH-3001_branch-switch-handler.md) | Implement Branch Switch Handler | rust-indexer-engineer | Pending |

**Plan Reference:** [Phase 3 - Branch Switch Handler](../planning/plan.md#phase-3-branch-switch-handler)

---

### Phase 4: Testing

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [UNIWATCH-4001](UNIWATCH-4001_enable-disabled-tests.md) | Enable and Migrate Disabled Unit Tests | rust-indexer-engineer | Pending |
| [UNIWATCH-4002](UNIWATCH-4002_integration-tests.md) | Create Integration Tests | integration-tester | Pending |
| [UNIWATCH-4003](UNIWATCH-4003_e2e-test-migration.md) | Migrate E2E Test Script to SQLite | rust-indexer-engineer | Pending |

**Plan Reference:** [Phase 4 - Testing](../planning/plan.md#phase-4-testing)

---

## Execution Order

```
UNIWATCH-0001 (exports)
    ↓
UNIWATCH-1001 (dynamic state)
    ↓
UNIWATCH-2001 (HEAD watcher)
    ↓
UNIWATCH-3001 (handler)
    ↓
UNIWATCH-4001 (unit tests)
    ↓
UNIWATCH-4002 (integration tests)
    ↓
UNIWATCH-4003 (E2E tests)
```

## Agent Summary

| Agent | Tickets |
|-------|---------|
| rust-indexer-engineer | UNIWATCH-0001, 1001, 2001, 3001, 4001, 4003 |
| integration-tester | UNIWATCH-4002 |
| verify-ticket | All tickets (verification phase) |
| commit-ticket | All tickets (commit phase) |

## Success Criteria (from Plan)

- [ ] `maproom watch` detects branch switches within 2 seconds
- [ ] File changes after switch index to the new worktree
- [ ] Rapid branch switches (< 2s apart) are debounced
- [ ] `BranchSwitchEvent` NDJSON emitted on branch change
- [ ] No regressions to existing file watching
- [ ] All tests pass

## Related Documents

- [README](../README.md) - Project overview
- [Architecture](../planning/architecture.md) - Technical design
- [Plan](../planning/plan.md) - Implementation phases
- [Quality Strategy](../planning/quality-strategy.md) - Test strategy
- [Review Updates](../planning/review-updates.md) - Changes from review

---

**Updated:** 2025-01-28 - Fixed test count discrepancy and added implementation notes based on tickets review.

**Next step:** Run `/work-on-project UNIWATCH` to execute tickets
