# Execution Plan: Search Result Deduplication

## Project Overview

**Project:** SRCHDUP - Search Result Deduplication
**Goal:** Eliminate duplicate search results across worktrees
**Duration:** 3-5 days (8-12 tickets)
**Complexity:** Low - focused feature addition

## Phase Structure

### Phase 1: Core Implementation (1-2 days)

**Objective:** Implement deduplication module with unit tests

**Deliverables:**
- `crates/maproom/src/search/dedup.rs` - New deduplication module
- `ChunkIdentity` struct for result grouping
- `deduplicate()` function with configurable behavior
- `DeduplicationConfig` and `SelectionStrategy` types
- Unit tests for all deduplication logic

**Tickets:**
| ID | Description | Agent |
|----|-------------|-------|
| SRCHDUP-1001 | Create dedup.rs module with ChunkIdentity | rust-indexer-engineer |
| SRCHDUP-1002 | Implement deduplicate() function | rust-indexer-engineer |
| SRCHDUP-1003 | Unit tests for dedup module | rust-indexer-engineer |

**Success Criteria:**
- [ ] `cargo test --lib search::dedup` passes all tests
- [ ] Identity key handles all edge cases (null symbol, etc.)
- [ ] Score-based selection works correctly

---

### Phase 2: Pipeline Integration (1 day)

**Objective:** Integrate deduplication into search pipeline

**Deliverables:**
- Modified `SearchOptions` with `deduplicate` field
- Pipeline calls dedup after fusion
- Search results automatically deduplicated (default on)
- Integration tests verify E2E behavior

**Tickets:**
| ID | Description | Agent |
|----|-------------|-------|
| SRCHDUP-2001 | Extend SearchOptions with deduplicate flag | rust-indexer-engineer |
| SRCHDUP-2002 | Integrate dedup into SearchPipeline | rust-indexer-engineer |
| SRCHDUP-2003 | Integration tests for pipeline dedup | integration-tester |

**Success Criteria:**
- [ ] Default search results are deduplicated
- [ ] `SearchOptions::without_dedup()` disables deduplication
- [ ] All existing search tests continue passing

---

### Phase 3: MCP Exposure (1 day)

**Objective:** Expose deduplication control via MCP API

**Deliverables:**
- MCP search tool accepts `deduplicate` parameter
- TypeScript schema updated
- E2E tests verify MCP behavior

**Tickets:**
| ID | Description | Agent |
|----|-------------|-------|
| SRCHDUP-3001 | Add deduplicate parameter to MCP search schema | vscode-extension-specialist |
| SRCHDUP-3002 | Pass deduplicate param to Rust daemon | vscode-extension-specialist |
| SRCHDUP-3003 | MCP E2E tests for deduplication | integration-tester |

**Success Criteria:**
- [ ] `search({query: "...", deduplicate: false})` returns all results
- [ ] Default behavior deduplicates
- [ ] MCP tests pass

---

### Phase 4: Documentation & Cleanup (0.5-1 day)

**Objective:** Document feature and verify quality

**Deliverables:**
- Updated search documentation
- Benchmark results documented
- Code review cleanup

**Tickets:**
| ID | Description | Agent |
|----|-------------|-------|
| SRCHDUP-4001 | Add dedup benchmarks | rust-indexer-engineer |
| SRCHDUP-4002 | Update search documentation | technical-researcher |
| SRCHDUP-4003 | Final verification and cleanup | verify-ticket |

**Success Criteria:**
- [ ] Benchmarks show <10ms for 1000 results
- [ ] Documentation updated
- [ ] All acceptance criteria verified

---

## Ticket Numbering

| Range | Phase | Description |
|-------|-------|-------------|
| 1001-1003 | Phase 1 | Core implementation |
| 2001-2003 | Phase 2 | Pipeline integration |
| 3001-3003 | Phase 3 | MCP exposure |
| 4001-4003 | Phase 4 | Documentation & cleanup |

## Agent Assignments

| Agent | Role | Tickets |
|-------|------|---------|
| rust-indexer-engineer | Core Rust implementation | 1001, 1002, 1003, 2001, 2002, 4001 |
| integration-tester | Integration and E2E tests | 2003, 3003 |
| vscode-extension-specialist | TypeScript MCP changes | 3001, 3002 |
| technical-researcher | Documentation | 4002 |
| verify-ticket | Verification | 4003 |
| commit-ticket | Commits | After each ticket |

## Dependencies

```
Phase 1 (Core)
    ↓
Phase 2 (Pipeline) ────→ Phase 3 (MCP)
    ↓                         ↓
Phase 4 (Docs & Cleanup) ←────┘
```

**Critical Path:** 1001 → 1002 → 2002 → (3001, 3002 parallel)

## Risk Management

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance regression | Low | Medium | Benchmarks in Phase 4 |
| Breaking existing tests | Low | High | Run full test suite after Phase 2 |
| MCP schema incompatibility | Low | Low | Optional parameter, backward compatible |

## Testing Checkpoints

| Checkpoint | Phase | Command | Expected |
|------------|-------|---------|----------|
| Unit tests | Phase 1 | `cargo test --lib search::dedup` | All pass |
| Integration | Phase 2 | `cargo test --test search` | All pass |
| Full suite | Phase 2 | `cargo test` | All pass |
| MCP tests | Phase 3 | `pnpm test` (maproom-mcp) | All pass |
| Benchmarks | Phase 4 | `cargo bench dedup` | <10ms/1000 results |

## Definition of Done

**Project Complete When:**
- [ ] All 12 tickets completed and verified
- [ ] All tests pass (unit, integration, E2E)
- [ ] Benchmarks meet performance targets
- [ ] Documentation updated
- [ ] No regression in existing search functionality
- [ ] Manual verification: duplicate search returns single result

## Rollback Plan

If issues arise post-deployment:
1. Set `deduplicate: false` in callers
2. Remove dedup call from pipeline (1 line change)
3. Revert commits if needed

## Timeline Summary

| Phase | Duration | Cumulative |
|-------|----------|------------|
| Phase 1: Core | 1-2 days | Days 1-2 |
| Phase 2: Pipeline | 1 day | Days 2-3 |
| Phase 3: MCP | 1 day | Days 3-4 |
| Phase 4: Docs | 0.5-1 day | Days 4-5 |

**Total Estimate:** 3-5 days
