# SQLIMPL Ticket Index

## Project Overview
Complete SQLite implementation for maproom indexer by migrating tests, wiring search executors, implementing incremental updates, and enabling the watch command.

## Ticket Summary
- **Total Tickets:** 19
- **Core MVP:** 15 tickets (Phases 1, 2, 3, 5)
- **Optional Enhancement:** 4 tickets (Phase 4)

---

## Phase 1: Test Infrastructure (5 tickets)
*Objective: Enable test compilation so all subsequent phases can be validated*

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [SQLIMPL-1001](SQLIMPL-1001_migrate-test-common-module.md) | Migrate Test Common Module + Triage | Not Started | None |
| [SQLIMPL-1002](SQLIMPL-1002_migrate-test-batch-1-integration.md) | Migrate Test Files Batch 1 (Integration) | Not Started | SQLIMPL-1001 |
| [SQLIMPL-1003](SQLIMPL-1003_migrate-test-batch-2-search.md) | Migrate Test Files Batch 2 (Search) | Not Started | SQLIMPL-1001 |
| [SQLIMPL-1004](SQLIMPL-1004_migrate-test-batch-3-incremental.md) | Migrate Test Files Batch 3 (Incremental) | Not Started | SQLIMPL-1001 |
| [SQLIMPL-1005](SQLIMPL-1005_migrate-test-batch-4-remaining.md) | Migrate Test Files Batch 4 (Remaining) | Not Started | SQLIMPL-1001 |

**Phase Gate:** `cargo test -p crewchief-maproom --no-run` compiles successfully

---

## Phase 2: Search Wiring (4 tickets)
*Objective: Wire search executors to existing SqliteStore methods*

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [SQLIMPL-2001](SQLIMPL-2001_wire-fts-executor.md) | Wire FTS Executor to SqliteStore | Not Started | Phase 1 Complete |
| [SQLIMPL-2002](SQLIMPL-2002_wire-vector-executor.md) | Wire Vector Executor to SqliteStore | Not Started | Phase 1 Complete |
| [SQLIMPL-2003](SQLIMPL-2003_wire-graph-executor.md) | Wire Graph Executor to SqliteStore | Not Started | Phase 1 Complete |
| [SQLIMPL-2004](SQLIMPL-2004_implement-signals-executor.md) | Implement Signals Executor | Not Started | Phase 1 Complete |

**Phase Gate:** `cargo run -- search "function"` returns non-empty, ranked results

---

## Phase 3: Incremental Updates (4 tickets)
*Objective: Make file change detection and persistence work*

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [SQLIMPL-3001](SQLIMPL-3001_implement-change-detector.md) | Implement Change Detector | Not Started | Phase 1 Complete |
| [SQLIMPL-3002](SQLIMPL-3002_implement-processor.md) | Implement Processor | Not Started | SQLIMPL-3001 |
| [SQLIMPL-3003](SQLIMPL-3003_implement-edge-updater.md) | Implement Edge Updater | Not Started | SQLIMPL-3001 |
| [SQLIMPL-3004](SQLIMPL-3004_implement-tree-sha-update.md) | Implement Tree SHA Update | Not Started | SQLIMPL-3001, SQLIMPL-3002 |

**Phase Gate:** Modify a file, run `upsert`, verify database updated correctly

---

## Phase 4: Context Assembly (4 tickets) - OPTIONAL
*Objective: Make context expansion and caching work*
*Status: Optional enhancement - defer if timeline pressure*

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [SQLIMPL-4001](SQLIMPL-4001_implement-context-cache.md) | Implement Context Cache | Not Started | Phase 1 Complete |
| [SQLIMPL-4002](SQLIMPL-4002_implement-context-graph.md) | Implement Context Graph | Not Started | Phase 1 Complete |
| [SQLIMPL-4003](SQLIMPL-4003_implement-language-detectors.md) | Implement Language Detectors | Not Started | Phase 1 Complete |
| [SQLIMPL-4004](SQLIMPL-4004_implement-language-strategies.md) | Implement Language Strategies | Not Started | SQLIMPL-4002, SQLIMPL-4003 |

**Phase Gate:** Context assembly returns expanded results with related chunks

---

## Phase 5: Watch Command (2 tickets)
*Objective: Enable continuous file monitoring*

| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [SQLIMPL-5001](SQLIMPL-5001_enable-watch-command.md) | Enable Watch Command in CLI | Not Started | Phase 3 Complete |
| [SQLIMPL-5002](SQLIMPL-5002_validate-watch-integration.md) | Validate Watch Integration | Not Started | SQLIMPL-5001 |

**Phase Gate:** `cargo run -- watch` monitors and updates continuously

---

## Dependency Graph

```
Phase 1 (Tests) ──┬──▶ Phase 2 (Search)
                  ├──▶ Phase 3 (Incremental) ──▶ Phase 5 (Watch)
                  └──▶ Phase 4 (Context) [Optional]
```

## Success Criteria

### Minimum Viable Completion (Core MVP)
- [ ] All tests compile: `cargo test -p crewchief-maproom --no-run`
- [ ] All tests pass: `cargo test -p crewchief-maproom`
- [ ] Search returns results: Non-empty, ranked
- [ ] Incremental updates persist: File changes detected and indexed
- [ ] Watch command works: Monitors and updates

### Extended Goals (Optional)
- [ ] Context assembly complete: Related chunks returned
- [ ] Language-specific detection: JSX, hooks, Python patterns

---

## References
- [Plan](../planning/plan.md)
- [Architecture](../planning/architecture.md)
- [Analysis](../planning/analysis.md)
- [Quality Strategy](../planning/quality-strategy.md)
- [Project Review](../planning/project-review.md)
