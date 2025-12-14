# Ticket: [SRCHCONF-2002]: Integrate Confidence into Search Pipeline

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
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
Integrate confidence computation into the search pipeline, add `include_confidence` parameter support (default: false), and modify ChunkSearchResult to include optional confidence field.

## Background
With confidence types and computation logic complete (Phase 1), this ticket integrates them into the actual search execution path. Confidence will only be computed when explicitly requested via `include_confidence=true` parameter, maintaining backward compatibility and performance for existing users.

This implements the integration point described in architecture.md: confidence computation happens after RRF fusion, before result assembly.

## Acceptance Criteria
- [x] `include_confidence` parameter added to SearchOptions (bool, default: false)
- [x] ChunkSearchResult has optional confidence field: `pub confidence: Option<ConfidenceSignals>`
- [x] Confidence field uses `#[serde(skip_serializing_if = "Option::is_none")]` attribute
- [x] When include_confidence=true, confidence is computed for each result
- [x] When include_confidence=false, confidence is None (backward compatibility)
- [x] Integration tests pass for both modes (with/without confidence)
- [x] All existing tests still pass (no regressions)
- [x] Zero clippy warnings

## Technical Requirements
**1. Modify ChunkSearchResult**:
```rust
pub struct ChunkSearchResult {
    pub chunk_id: i64,
    pub score: f32,
    pub source_scores: HashMap<SearchSource, f32>,
    // ... existing fields ...

    /// Confidence signals for result quality assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceSignals>,
}
```

**2. Add SearchOptions Parameter**:
```rust
pub struct SearchOptions {
    // ... existing fields ...
    pub include_confidence: bool,  // Default: false
}
```

**3. Integration Point** (in search executors or pipeline):
```rust
// After RRF fusion
let fused_results = fusion.fuse(ranked_results, &weights, options.limit);

// Assemble results
let results: Vec<ChunkSearchResult> = fused_results
    .into_iter()
    .enumerate()
    .map(|(index, fused)| {
        let confidence = if options.include_confidence {
            Some(compute_result_confidence(
                &fused,
                &fused_results,
                index,
                fused.exact_match_multiplier,
            ))
        } else {
            None
        };

        ChunkSearchResult {
            chunk_id: fused.chunk_id,
            score: fused.score,
            source_scores: fused.source_scores.clone(),
            // ... other fields ...
            confidence,
        }
    })
    .collect();
```

**4. Integration Tests** (minimum 4 tests):
- Search with include_confidence=true returns confidence
- Search with include_confidence=false returns None
- Default behavior (parameter omitted) is confidence=None
- Confidence fields have correct values (source_count > 0, score_gap >= 0.0)

## Implementation Notes
Integration follows architecture.md design:
- Confidence computed in-memory after score fusion
- No database queries, no network calls
- O(1) computation per result
- Stateless, no side effects

Key locations (from architecture.md):
- `crates/maproom/src/search/results.rs` - Modify ChunkSearchResult
- `crates/maproom/src/search/executors.rs` - Integration point after fusion
- `crates/maproom/src/search/mod.rs` - Ensure confidence module is accessible

Backward compatibility strategy:
- Optional field with serde skip (JSON omits None)
- Parameter defaults to false (opt-in for MVP)
- Existing consumers unaffected
- Pattern proven by QueryUnderstanding from SRCHTRN

## Dependencies
- **Prerequisite**: SRCHCONF-1001 (confidence types must exist)
- **Prerequisite**: SRCHCONF-1002 (exact_match_multiplier must be available)
- **Prerequisite**: SRCHCONF-1003 (tests validate computation logic)
- **Prerequisite**: SRCHCONF-2001 (TypeScript types must be synced before integration)

## Risk Assessment
- **Risk**: Performance regression from confidence computation
  - **Mitigation**: Only computed when requested (opt-in). Benchmark from 1003 shows <5ms overhead. Monitor p95 latency.
- **Risk**: Integration breaks existing search functionality
  - **Mitigation**: Run full test suite. Optional field ensures backward compatibility. Integration tests cover both modes.
- **Risk**: Type mismatch between Rust and TypeScript after integration
  - **Mitigation**: SRCHCONF-2001 validation tests catch discrepancies. Verify JSON serialization matches TypeScript expectations.

## Files/Packages Affected
- `crates/maproom/src/search/results.rs` - Add confidence field to ChunkSearchResult
- `crates/maproom/src/search/executors.rs` - Call compute_result_confidence()
- `crates/maproom/src/search/mod.rs` - Ensure confidence module exported
- `crates/maproom/tests/integration/search_tests.rs` - Add integration tests

## Verification Notes
The verify-ticket agent should check:
- ChunkSearchResult has optional confidence field with serde skip attribute
- include_confidence parameter exists and defaults to false
- Integration tests demonstrate both modes work correctly
- Test output shows confidence=Some(...) when enabled, confidence=None when disabled
- All existing tests still pass (run full suite)
- No performance regression (p95 < 50ms maintained)
- JSON serialization omits confidence field when None
