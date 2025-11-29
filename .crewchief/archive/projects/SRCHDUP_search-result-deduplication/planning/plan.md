# Execution Plan: Search Result Deduplication

## Project Overview

**Project:** SRCHDUP - Search Result Deduplication
**Goal:** Eliminate duplicate search results across worktrees
**Duration:** 3-5 days (10 tickets)
**Complexity:** Low - focused feature addition

> **Note:** This plan was revised after project review to address integration gaps
> (daemon-client, CLI flags) identified in project-review.md.

## Phase Structure

### Phase 1: Core Implementation (1 day)

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
| SRCHDUP-1001 | Create dedup.rs module with ChunkIdentity and deduplicate() | rust-indexer-engineer |
| SRCHDUP-1002 | Unit tests for dedup module | rust-indexer-engineer |

> **Note:** Consolidated from 3 tickets to 2. Module creation and function implementation
> are tightly coupled and should be done together.

**Success Criteria:**
- [ ] `cargo test --lib search::dedup` passes all tests
- [ ] Identity key handles all edge cases (null symbol, etc.)
- [ ] Score-based selection works correctly

---

### Phase 2: Pipeline Integration (1 day)

**Objective:** Integrate deduplication into search pipeline and CLI

**Deliverables:**
- Modified `SearchOptions` with `deduplicate` field
- Pipeline calls dedup after fusion (before limit)
- CLI `--deduplicate` flag support
- Search results automatically deduplicated (default on)
- Integration tests verify E2E behavior

**Tickets:**
| ID | Description | Agent |
|----|-------------|-------|
| SRCHDUP-2001 | Extend SearchOptions with deduplicate flag | rust-indexer-engineer |
| SRCHDUP-2002 | Integrate dedup into SearchPipeline | rust-indexer-engineer |
| SRCHDUP-2003 | Add --deduplicate CLI flag to search command | rust-indexer-engineer |
| SRCHDUP-2004 | Integration tests for pipeline dedup | integration-tester |

> **Note:** Added CLI flag ticket (SRCHDUP-2003) per review recommendation.

**Success Criteria:**
- [ ] Default search results are deduplicated
- [ ] `SearchOptions::without_dedup()` disables deduplication
- [ ] `crewchief-maproom search --no-deduplicate` works
- [ ] All existing search tests continue passing

---

### Phase 3: MCP Exposure (1 day)

**Objective:** Expose deduplication control via MCP API through the full integration stack

**Deliverables:**
- Daemon-client `SearchParams` interface updated with `deduplicate`
- Rust daemon JSON-RPC handler accepts `deduplicate` param
- MCP search tool schema updated
- E2E tests verify MCP behavior

**Tickets:**
| ID | Description | Agent |
|----|-------------|-------|
| SRCHDUP-3001 | Update daemon-client SearchParams interface | vscode-extension-specialist |
| SRCHDUP-3002 | Update Rust daemon JSON-RPC handler for deduplicate | rust-indexer-engineer |
| SRCHDUP-3003 | Add deduplicate parameter to MCP search schema | vscode-extension-specialist |
| SRCHDUP-3004 | MCP E2E tests for deduplication | integration-tester |

> **Note:** Split into 4 tickets to cover the full integration stack:
> daemon-client → JSON-RPC → MCP schema → E2E tests.
> This addresses the integration gap identified in review.

**Success Criteria:**
- [ ] `search({query: "...", deduplicate: false})` returns all results
- [ ] Default behavior deduplicates
- [ ] MCP tests pass
- [ ] Full integration path works: MCP → daemon-client → JSON-RPC → Rust

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
| 1001-1002 | Phase 1 | Core implementation (2 tickets) |
| 2001-2004 | Phase 2 | Pipeline integration + CLI (4 tickets) |
| 3001-3004 | Phase 3 | MCP exposure + daemon-client (4 tickets) |
| 4001-4003 | Phase 4 | Documentation & cleanup (3 tickets) |

**Total: 13 tickets**

## Agent Assignments

| Agent | Role | Tickets |
|-------|------|---------|
| rust-indexer-engineer | Core Rust implementation | 1001, 1002, 2001, 2002, 2003, 3002, 4001 |
| integration-tester | Integration and E2E tests | 2004, 3004 |
| vscode-extension-specialist | TypeScript changes | 3001, 3003 |
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

**Critical Path:** 1001 → 1002 → 2001 → 2002 → 2003 → 3001 → 3002 → 3003

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
- [ ] All 13 tickets completed and verified
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
