# RSTFIX Ticket Index

Ticket tracking for the **Rust Build Cleanup** project.

## Overview

| Metric | Before | Target |
|--------|--------|--------|
| Rust warnings | ~58 | 0 |
| Test failures | 1 | 0 |
| Clippy issues | Unknown | 0 |

## Phase 1: Unused Imports (Low Risk)

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [RSTFIX-1001](RSTFIX-1001_auto-fix-imports.md) | Auto-fix imports and cleanup warnings | rust-indexer-engineer | Pending |

**Plan Reference:** Phase 1 in `planning/plan.md`

## Phase 2: Unused Variables (Medium Risk)

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [RSTFIX-2001](RSTFIX-2001_fix-unused-variables-search.md) | Fix unused variables in search executors | rust-indexer-engineer | Pending |
| [RSTFIX-2002](RSTFIX-2002_fix-unused-variables-context.md) | Fix unused variables in context module | rust-indexer-engineer | Pending |
| [RSTFIX-2003](RSTFIX-2003_fix-unused-variables-incremental.md) | Fix unused variables in incremental module | rust-indexer-engineer | Pending |

**Plan Reference:** Phase 2 in `planning/plan.md`

**Dependencies:** RSTFIX-1001 must complete first (import removal may surface additional variable issues)

## Phase 3: Dead Code (Higher Risk)

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [RSTFIX-3001](RSTFIX-3001_remove-dead-functions.md) | Remove dead functions and methods | rust-indexer-engineer | Pending |
| [RSTFIX-3002](RSTFIX-3002_remove-unused-structs.md) | Remove unused struct fields | rust-indexer-engineer | Pending |

**Plan Reference:** Phase 3 in `planning/plan.md`

**Dependencies:** Phase 1 and Phase 2 should complete first

## Phase 4: Test Fix

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [RSTFIX-4001](RSTFIX-4001_fix-config-validation-test.md) | Fix config validation test failure | rust-indexer-engineer | Pending |

**Plan Reference:** Phase 4 in `planning/plan.md`

**Dependencies:** Can run in parallel with Phases 2-3

## Phase 5: Final Verification

| Ticket | Title | Agent | Status |
|--------|-------|-------|--------|
| [RSTFIX-5001](RSTFIX-5001_final-verification.md) | Final build and test verification | unit-test-runner | Pending |

**Plan Reference:** Phase 5 in `planning/plan.md`

**Dependencies:** All previous tickets must complete

## Dependency Graph

```
RSTFIX-1001 (Phase 1: Imports)
    │
    ├──> RSTFIX-2001 (Phase 2: Search vars)
    ├──> RSTFIX-2002 (Phase 2: Context vars)
    ├──> RSTFIX-2003 (Phase 2: Incremental vars)
    │
    └──> RSTFIX-3001 (Phase 3: Dead functions)
         └──> RSTFIX-3002 (Phase 3: Dead structs)

RSTFIX-4001 (Phase 4: Test fix) - Can run in parallel

All above ──> RSTFIX-5001 (Phase 5: Final verification)
```

## Execution Order

**Recommended sequence:**
1. RSTFIX-1001 (required first)
2. RSTFIX-2001, RSTFIX-2002, RSTFIX-2003, RSTFIX-4001 (can run in parallel)
3. RSTFIX-3001, RSTFIX-3002 (after Phase 2)
4. RSTFIX-5001 (final verification)

## Summary

- **Total tickets:** 8
- **Primary agent:** rust-indexer-engineer
- **Verification agent:** unit-test-runner
- **Estimated scope:** Single session

---

Next step: Run `/review-tickets RSTFIX` to validate quality or proceed to `/work-on-project RSTFIX` to execute tickets
