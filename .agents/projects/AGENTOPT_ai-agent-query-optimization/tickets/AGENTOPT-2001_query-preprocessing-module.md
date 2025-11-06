# Ticket: AGENTOPT-2001: Implement Query Preprocessing Module (Phase 2 - OPTIONAL)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
**OPTIONAL PHASE 2**: Implement server-side query preprocessing in Rust to normalize queries before search execution. Only proceed if Phase 1 shows ≥50% improvement but <70% target.

## Background
This ticket implements Phase 2 preprocessing from the AGENTOPT project plan (planning/plan.md lines 333-353). Server-side preprocessing provides baseline query optimization for all MCP clients, not just Claude Code. It normalizes whitespace, removes stop words, and applies simple transformations.

**Trigger Condition**: Phase 1 monitoring (AGENTOPT-1006) shows improvement but doesn't hit 70% natural language success target.

## Acceptance Criteria
- [ ] Query preprocessing function implemented in Rust
- [ ] Stop word removal (how, what, where, when, why, does, is, are, the, a)
- [ ] Whitespace normalization and lowercasing
- [ ] Unit tests with 100% coverage
- [ ] Performance: <1ms preprocessing latency (p95)
- [ ] Feature flag for gradual rollout

## Technical Requirements
- Location: `crates/maproom/src/query/preprocessor.rs`
- Implementation pattern from architecture.md lines 226-259
- Stop word list: common English question words and articles
- No external dependencies (use std library only)
- Return preprocessed query as String

## Implementation Notes
See architecture.md lines 219-259 for Rust implementation example.

## Dependencies
- AGENTOPT-1006 (Phase 1 results showing need for Phase 2)

## Risk Assessment
- **Risk**: Preprocessing too aggressive (removes important terms)
  - **Mitigation**: Conservative stop word list, A/B test before full rollout

## Files/Packages Affected
- crates/maproom/src/query/preprocessor.rs (create)
- crates/maproom/src/query/mod.rs (add module)
