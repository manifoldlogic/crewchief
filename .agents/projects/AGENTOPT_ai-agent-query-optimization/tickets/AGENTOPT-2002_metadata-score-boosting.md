# Ticket: AGENTOPT-2002: Implement Metadata Score Boosting (Phase 2 - OPTIONAL)

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
**OPTIONAL PHASE 2**: Enhance search result scoring based on code structure signals (file paths, symbol names, recency).

## Background
This ticket implements Phase 2 metadata boosting from planning/architecture.md lines 262-303. Boosts search scores based on:
- Path signals (prefer src/ over root, de-rank tests)
- Name matching (boost if symbol name contains query term)
- Recency (use existing git metadata)

**Trigger Condition**: Phase 1 + AGENTOPT-2001 deployed but still need additional quality improvement.

## Acceptance Criteria
- [ ] Metadata boosting function implemented
- [ ] Path-based boost (src/ = 1.2x, tests = 0.9x)
- [ ] Name matching boost (1.5x if symbol name contains term)
- [ ] Integration with search scoring pipeline
- [ ] Unit tests with edge cases
- [ ] Performance: <2ms per result

## Technical Requirements
- Location: `crates/maproom/src/query/scoring.rs` or inline in search
- Implementation pattern from architecture.md lines 269-299
- Configurable boost multipliers
- Apply after FTS/vector scoring, before ranking

## Implementation Notes
See architecture.md lines 262-303 for implementation example.

## Dependencies
- AGENTOPT-2001 (query preprocessing)

## Risk Assessment
- **Risk**: Boosts too aggressive, skew results
  - **Mitigation**: Conservative multipliers (1.1x-1.5x range), A/B test

## Files/Packages Affected
- crates/maproom/src/query/scoring.rs (create or modify)
