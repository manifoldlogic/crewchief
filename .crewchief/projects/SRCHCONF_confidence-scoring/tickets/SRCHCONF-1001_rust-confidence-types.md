# Ticket: [SRCHCONF-1001]: Rust Confidence Types and Computation Module

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 12/12 confidence module tests passing
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create the foundational confidence scoring types and computation logic in Rust, including the `ConfidenceSignals` struct with 3 core fields (source_count, score_gap, is_exact_match) and the confidence computation module.

## Background
This ticket implements Phase 1 of the confidence scoring project. Users currently receive search results with relevance scores but cannot assess result quality or confidence. This work creates the core Rust infrastructure for computing confidence signals from existing search pipeline data.

This implements the "Core Confidence Infrastructure" phase from plan.md, focusing on MVP with 3 essential signals that provide transparency without requiring complex tuning.

## Acceptance Criteria
- [x] `ConfidenceSignals` struct exists in `crates/maproom/src/search/results.rs` with exactly 3 fields: source_count (usize), score_gap (f32), is_exact_match (bool)
- [x] New `crates/maproom/src/search/confidence.rs` module created with `compute_result_confidence()` function
- [x] `compute_result_confidence()` correctly computes all 3 confidence fields from FusedResult data
- [x] Confidence module exported from `crates/maproom/src/search/mod.rs`
- [x] All Rust code compiles without warnings (`cargo build -p crewchief-maproom`)
- [x] Zero clippy warnings (`cargo clippy -p crewchief-maproom`)
- [x] Serde serialization works (ConfidenceSignals derives Serialize/Deserialize)
- [x] TYPE_SYNC comment added linking to future TypeScript interface

## Technical Requirements
- `ConfidenceSignals` struct must derive `Debug, Clone, Serialize, Deserialize`
- Add TYPE_SYNC comment: `/// TYPE_SYNC: packages/daemon-client/src/types.ts::ConfidenceSignals`
- `compute_result_confidence()` signature: `fn compute_result_confidence(result: &FusedResult, all_results: &[FusedResult], index: usize, exact_match_multiplier: Option<f32>) -> ConfidenceSignals`
- Computation logic:
  - `source_count = result.source_scores.len()`
  - `score_gap = result.score - all_results[index + 1].score` (or 0.0 if last result)
  - `is_exact_match = exact_match_multiplier.map(|m| m >= 2.9).unwrap_or(false)`
- Handle edge cases gracefully (empty results, single result, None multiplier)
- Use only stdlib (no new dependencies)

## Implementation Notes
Follow the architecture described in architecture.md:
- Place `ConfidenceSignals` in `crates/maproom/src/search/results.rs` alongside other result types
- Create new module `crates/maproom/src/search/confidence.rs` for computation logic
- Confidence is MVP-focused: 3 core signals only (source_count, score_gap, is_exact_match)
- Deferred signals (relative_score, rank) will be added in Phase 2 if validated
- All computation is O(1) per result, using in-memory data only
- No database queries, no heap allocations

Key design decisions:
- Component-based confidence (not single score) for transparency
- In-memory computation only (no database overhead)
- Graceful degradation if exact_match_multiplier unavailable

## Dependencies
- **External**: SRCHTRN (Search Transparency) - COMPLETE (provides QueryUnderstanding pattern)
- **External**: SRCHFLTR (Result Filtering) - COMPLETE (provides cleaner result sets)
- **Internal**: None (first ticket in Phase 1)

## Risk Assessment
- **Risk**: exact_match_multiplier may only be available in debug mode currently
  - **Mitigation**: Use Option<f32> parameter, default is_exact_match to false if None. Ticket SRCHCONF-1002 will make it always available.
- **Risk**: Performance overhead from confidence computation
  - **Mitigation**: O(1) computation per result, benchmark in SRCHCONF-1003 before integration

## Files/Packages Affected
- `crates/maproom/src/search/results.rs` - Add ConfidenceSignals struct
- `crates/maproom/src/search/confidence.rs` - NEW module for computation
- `crates/maproom/src/search/mod.rs` - Export confidence module

## Verification Notes
The verify-ticket agent should check:
- ConfidenceSignals struct has exactly 3 fields with correct types
- TYPE_SYNC comment is present and correctly formatted
- compute_result_confidence() function signature matches specification
- Code compiles without warnings
- No clippy warnings
- Confidence module is exported and accessible
- Edge cases are handled (empty results, None multiplier)
