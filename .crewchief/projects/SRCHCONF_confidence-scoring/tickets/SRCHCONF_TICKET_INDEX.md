# SRCHCONF Ticket Index

Project: Confidence Scoring for Search Results
Status: Ready for execution
Created: 2025-12-14

## Overview

This project adds confidence scoring to maproom search results through a 3-phase incremental delivery approach. Each phase delivers independent value while building toward complete confidence transparency.

**Timeline**: 3 phases, estimated 2-3 days total development time
**Approach**: MVP-first (3 core signals), incremental delivery, backward compatible

## Phase 1: Core Confidence Infrastructure (3 tickets)

Foundation - Rust types, computation logic, and validation

### SRCHCONF-1001: Rust Confidence Types and Computation Module
**Status**: Not started
**Agents**: rust-engineer, unit-test-runner, verify-ticket, commit-ticket
**Summary**: Create ConfidenceSignals struct (3 fields: source_count, score_gap, is_exact_match) and confidence computation module
**Estimated**: 3-4 hours
**Files**: `crates/maproom/src/search/results.rs`, `crates/maproom/src/search/confidence.rs`
**Dependencies**: None

### SRCHCONF-1002: Make Exact Match Multiplier Always Available
**Status**: Not started
**Agents**: rust-engineer, unit-test-runner, verify-ticket, commit-ticket
**Summary**: Modify FTS semantic ranking to compute exact_match_multiplier unconditionally (not just debug mode)
**Estimated**: 2-3 hours
**Files**: `crates/maproom/src/search/fts.rs`, `crates/maproom/src/search/fusion.rs`
**Dependencies**: SRCHCONF-1001

### SRCHCONF-1003: Confidence Unit Tests and Performance Benchmarks
**Status**: Not started
**Agents**: rust-engineer, unit-test-runner, verify-ticket, commit-ticket
**Summary**: Create 8+ unit tests for confidence computation and benchmarks proving <5ms overhead
**Estimated**: 3-4 hours
**Files**: `crates/maproom/src/search/confidence.rs` (tests), `crates/maproom/benches/confidence_overhead.rs`
**Dependencies**: SRCHCONF-1001, SRCHCONF-1002

## Phase 2: TypeScript Type Sync and Integration (2 tickets)

Synchronize types and integrate into search pipeline

### SRCHCONF-2001: TypeScript Type Sync and Validation Tests
**Status**: Not started
**Agents**: typescript-engineer, unit-test-runner, verify-ticket, commit-ticket
**Summary**: Create TypeScript ConfidenceSignals interface matching Rust, add TYPE_SYNC comments and validation tests
**Estimated**: 2-3 hours
**Files**: `packages/daemon-client/src/types.ts`, `packages/daemon-client/src/types.test.ts`
**Dependencies**: SRCHCONF-1001 (Rust types must exist), Phase 1 complete

### SRCHCONF-2002: Integrate Confidence into Search Pipeline
**Status**: Not started
**Agents**: rust-engineer, unit-test-runner, verify-ticket, commit-ticket
**Summary**: Add include_confidence parameter, integrate confidence computation into search execution, add optional confidence field to ChunkSearchResult
**Estimated**: 4-5 hours
**Files**: `crates/maproom/src/search/results.rs`, `crates/maproom/src/search/executors.rs`
**Dependencies**: SRCHCONF-1001, SRCHCONF-1002, SRCHCONF-1003, SRCHCONF-2001

## Phase 3: MCP Tool and Documentation (3 tickets)

Expose via MCP and document usage

### SRCHCONF-3001: MCP Tool Integration and Parameter Passing
**Status**: Not started
**Agents**: typescript-engineer, unit-test-runner, verify-ticket, commit-ticket
**Summary**: Update MCP search tool to accept include_confidence parameter and pass to daemon
**Estimated**: 2-3 hours
**Files**: `packages/daemon-client/src/client.ts`, `packages/maproom-mcp/src/tools/search_schema.ts`, `packages/maproom-mcp/src/tools/search.ts`
**Dependencies**: SRCHCONF-2001, SRCHCONF-2002, Phase 2 complete

### SRCHCONF-3002: Confidence Scoring Documentation
**Status**: Not started
**Agents**: documentation-writer, verify-ticket, commit-ticket
**Summary**: Create comprehensive documentation explaining confidence signals, interpretation, and usage patterns
**Estimated**: 3-4 hours
**Files**: `packages/maproom-mcp/docs/confidence-scoring.md`
**Dependencies**: SRCHCONF-3001

### SRCHCONF-3003: End-to-End Integration Tests
**Status**: Not started
**Agents**: typescript-engineer, unit-test-runner, verify-ticket, commit-ticket
**Summary**: Create 4+ end-to-end tests validating full stack from MCP tool to Rust search and back
**Estimated**: 3-4 hours
**Files**: `packages/maproom-mcp/tests/integration/confidence.test.ts`
**Dependencies**: SRCHCONF-3001, SRCHCONF-2002, All Phase 1-2 tickets complete

## Execution Order

**Sequential dependencies**:
1. Phase 1 must complete before Phase 2
2. Phase 2 must complete before Phase 3
3. Within each phase, follow ticket number order

**Recommended execution**:
```
Phase 1: SRCHCONF-1001 → SRCHCONF-1002 → SRCHCONF-1003
Phase 2: SRCHCONF-2001 → SRCHCONF-2002
Phase 3: SRCHCONF-3001 → SRCHCONF-3002 → SRCHCONF-3003
```

**Parallel opportunities**:
- SRCHCONF-3002 (documentation) can be drafted during Phase 2
- Planning for SRCHCONF-3003 (integration tests) can happen during Phase 2

## Success Criteria

### Phase 1 Complete
- [ ] All Rust structs compile without warnings
- [ ] 100% unit test coverage for confidence computation
- [ ] Performance benchmarks show <5ms overhead
- [ ] Exact match detection works without debug mode

### Phase 2 Complete
- [ ] TypeScript types match Rust types field-for-field
- [ ] Type sync validation tests pass
- [ ] Confidence computed when include_confidence=true
- [ ] Backward compatibility maintained (confidence=None when disabled)

### Phase 3 Complete
- [ ] MCP tool accepts include_confidence parameter
- [ ] Documentation explains all 3 confidence signals
- [ ] End-to-end tests pass for both modes (with/without confidence)
- [ ] Performance target met (<50ms p95 total latency)

## Overall Project Success

- [ ] All 8 tickets complete and verified
- [ ] Zero breaking changes to existing API
- [ ] Performance target met (<50ms p95 total latency)
- [ ] Documentation explains confidence signals clearly
- [ ] CI pipeline includes confidence validation tests

## Risk Mitigation

| Risk | Mitigation | Tickets Affected |
|------|-----------|------------------|
| Type sync breaks | TYPE_SYNC comments, validation tests | SRCHCONF-2001 |
| Performance regression | Early benchmarking in Phase 1 | SRCHCONF-1003 |
| Backward compatibility | Optional fields, integration tests | SRCHCONF-2002, SRCHCONF-3001 |
| Exact match unavailable | Graceful degradation, dedicated ticket | SRCHCONF-1002 |

## Notes

- **MVP Focus**: 3 core signals only (source_count, score_gap, is_exact_match)
- **Deferred Signals**: relative_score, rank, query summary moved to future phase if validated
- **Default Behavior**: include_confidence defaults to false (opt-in rollout)
- **Performance Target**: <5ms overhead, <50ms p95 total latency
- **External Dependencies**: SRCHTRN (complete), SRCHFLTR (complete)
