# BRWATCH: Automatic Branch Switch Detection - Ticket Index

**Project Status**: ✅ Tickets Created → Ready for Execution
**Total Tickets**: 15 work tickets + 1 index
**Total Phases**: 4 phases
**Dependency**: BRANCHX (Branch-Aware Indexing) must be complete
**Timeline**: 3-4 days (1 day per phase + 1 buffer day)

## Summary

This project implements automatic branch switch detection for the maproom indexer. When developers run `maproom watch`, the system monitors `.git/HEAD` for changes and automatically triggers incremental indexing when branches are switched. This eliminates the manual `maproom scan` step and provides seamless, automatic code search updates.

**Key Features**:
- OS-level file watching (not polling) for <1s detection latency
- Automatic incremental updates using BRANCHX infrastructure
- Resource efficient: <5% CPU idle, <20MB memory
- Graceful error handling with retry logic
- CLI command with Ctrl+C shutdown

## Ticket Organization

Tickets follow phase-based numbering:
- **Phase 1 (1xxx)**: File Watcher Implementation
- **Phase 2 (2xxx)**: Branch Switch Handler
- **Phase 3 (3xxx)**: CLI Command
- **Phase 4 (4xxx)**: Documentation
- **Test Tickets (x9xx)**: Critical path validation

## Phase 1: File Watcher Implementation (Day 1)

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| BRWATCH-1001 | Add notify and ctrlc dependencies | ⬜ | rust-indexer-engineer |
| BRWATCH-1002 | Implement BranchWatcher struct and file watching | ⬜ | rust-indexer-engineer |
| BRWATCH-1003 | Implement branch name parsing | ⬜ | rust-indexer-engineer |
| BRWATCH-1901 | Unit tests for file watcher | ⬜ | unit-test-runner |

**Phase 1 Success Criteria**:
- Watcher detects .git/HEAD changes
- Current branch extracted correctly
- All unit tests pass

## Phase 2: Branch Switch Handler (Day 2)

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| BRWATCH-2001 | Implement handle_branch_switch method | ⬜ | rust-indexer-engineer |
| BRWATCH-2002 | Add error handling and retry logic | ⬜ | rust-indexer-engineer |
| BRWATCH-2003 | Add debouncing for rapid switches | ⬜ | rust-indexer-engineer |
| BRWATCH-2901 | Integration tests for branch switching | ⬜ | unit-test-runner |

**Phase 2 Success Criteria**:
- Auto-update triggers incremental_update
- Errors logged, watcher continues
- Integration tests pass

## Phase 3: CLI Command (Day 3)

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| BRWATCH-3001 | Add watch command to CLI | ⬜ | rust-indexer-engineer |
| BRWATCH-3002 | Implement graceful shutdown with Ctrl+C | ⬜ | rust-indexer-engineer |
| BRWATCH-3003 | Add logging and metrics output | ⬜ | rust-indexer-engineer |
| BRWATCH-3901 | E2E tests for CLI command | ⬜ | unit-test-runner |
| BRWATCH-3902 | Performance tests (CPU, memory, latency) | ⬜ | unit-test-runner |

**Phase 3 Success Criteria**:
- CLI command works
- Graceful shutdown (Ctrl+C)
- Performance benchmarks met (<5% CPU, <20MB RAM)

## Phase 4: Documentation (Day 4)

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| BRWATCH-4001 | Create user documentation | ⬜ | general-purpose |
| BRWATCH-4002 | Add Rustdoc comments to watcher code | ⬜ | general-purpose |
| BRWATCH-4003 | Update CHANGELOG and README | ⬜ | general-purpose |

**Phase 4 Success Criteria**:
- Usage guide complete
- Code documentation complete
- CHANGELOG updated

## Critical Path Tests

From quality-strategy.md, 4 critical tests MUST pass:

1. ✅ `test_watcher_detects_all_switches` - Reliability (BRWATCH-1901)
2. ✅ `test_handler_continues_after_error` - Resilience (BRWATCH-2901)
3. ✅ `test_idle_resource_usage` - Efficiency (BRWATCH-3902)
4. ✅ `test_rapid_switching` - Concurrency (BRWATCH-2901)

## Dependencies

**Project Dependencies**:
- BRANCHX complete (incremental_update, get_or_create_worktree)

**Ticket Dependencies**:
- Phase 2 depends on Phase 1 complete
- Phase 3 depends on Phase 2 complete
- Phase 4 can run in parallel with final testing

## Planning References

- `/workspace/.crewchief/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Implementation plan
- `/workspace/.crewchief/projects/BRWATCH_branch-switch-detection/planning/architecture.md` - Technical architecture
- `/workspace/.crewchief/projects/BRWATCH_branch-switch-detection/planning/quality-strategy.md` - Testing strategy
- `/workspace/.crewchief/projects/BRWATCH_branch-switch-detection/planning/analysis.md` - Problem analysis

## Execution Strategy

Execute tickets sequentially by phase:
1. Complete Phase 1 (1001-1003, test with 1901)
2. Complete Phase 2 (2001-2003, test with 2901)
3. Complete Phase 3 (3001-3003, test with 3901, 3902)
4. Complete Phase 4 (4001-4003)

Use `/work-on-project BRWATCH` to execute all tickets systematically.
