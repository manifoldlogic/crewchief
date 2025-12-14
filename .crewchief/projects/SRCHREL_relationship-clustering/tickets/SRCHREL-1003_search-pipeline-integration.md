# Ticket: [SRCHREL-1003]: Search Pipeline Integration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- search-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate relationship expansion into the search pipeline after confidence scoring, with confidence gating, error handling, and the `include_related` parameter.

## Background
The search pipeline needs to conditionally expand relationships for high-confidence results when `include_related=true`. This integration happens after confidence scoring (SRCHCONF) and before result return. Failures must gracefully degrade (log warning, don't fail search).

This implements Phase 1 deliverables: pipeline integration, include_related parameter, confidence gating (source_count >= 2 OR is_exact_match).

## Acceptance Criteria
- [ ] `include_related` boolean parameter added to search options/params struct
- [ ] `related` optional field added to `ChunkSearchResult` with `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] Confidence auto-enabled when `include_related=true` (follows auto-enable pattern)
- [ ] Relationship expansion runs after confidence scoring
- [ ] Only expands results with `confidence.source_count >= 2` OR `confidence.is_exact_match == true`
- [ ] MAX_CONCURRENT_EXPANSIONS = 3 hard cap enforced
- [ ] Graph traversal errors logged but don't fail search (result.related = None)
- [ ] Integration tests pass for confidence gating and backward compatibility

## Technical Requirements

### Add to ChunkSearchResult
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSearchResult {
    // ... existing fields ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceSignals>,  // From SRCHCONF

    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<Vec<RelatedChunkResult>>, // NEW
}
```

### Add to SearchOptions/Params
```rust
pub struct SearchOptions {
    // ... existing fields ...
    pub include_confidence: Option<bool>,
    pub include_related: Option<bool>,  // NEW
}
```

### Integration Point (after confidence scoring)
```rust
// Auto-enable confidence if related chunks requested
let enable_confidence = options.include_confidence.unwrap_or(false)
    || options.include_related.unwrap_or(false);

// Compute confidence (SRCHCONF)
if enable_confidence {
    // ... existing confidence computation ...
}

// NEW: Relationship expansion
const MAX_CONCURRENT_EXPANSIONS: usize = 3;

if options.include_related.unwrap_or(false) {
    let mut expansion_count = 0;

    for result in &mut results {
        if expansion_count >= MAX_CONCURRENT_EXPANSIONS {
            break;  // Hard cap
        }

        // Only expand high-confidence results
        if let Some(conf) = &result.confidence {
            if conf.source_count >= 2 || conf.is_exact_match {
                match find_top_related_chunks(store, result.chunk_id, 5).await {
                    Ok(related) => {
                        result.related = Some(related);
                        expansion_count += 1;
                    }
                    Err(e) => {
                        // Log error but don't fail entire search
                        tracing::warn!("Failed to find related chunks for {}: {}", result.chunk_id, e);
                    }
                }
            }
        }
    }
}
```

### Error Handling
- Use `tracing::warn!` for graph traversal errors
- Result simply has `related: None` on failure
- Search always succeeds even if all expansions fail

## Implementation Notes

Auto-enable pattern from SRCHCONF:
- When `include_related=true`, confidence is automatically enabled
- Simplifies UX (users don't need to specify both parameters)
- Backward compatible (users can still request confidence alone)

Hard cap rationale:
- 3 results × 8ms = 24ms (slightly over 20ms budget but acceptable)
- Prevents performance degradation if confidence thresholds are looser than expected
- Users still get relationships for highest-confidence results

Empty result semantics:
- `result.related = None`: Expansion didn't run (low confidence, disabled, or error)
- `result.related = Some([])`: Expansion ran but found no relationships (valid, informative)

## Dependencies
- SRCHREL-1001 (RelatedChunkResult type)
- SRCHREL-1002 (find_top_related_chunks function)
- SRCHCONF project (ConfidenceSignals, confidence scoring logic)

## Risk Assessment
- **Risk**: Integration point location unclear (pipeline vs executors)
  - **Mitigation**: Review SRCHCONF integration; follow same pattern
- **Risk**: Confidence auto-enable conflicts with existing logic
  - **Mitigation**: Test backward compatibility; ensure `include_confidence=true` alone still works
- **Risk**: MAX_CONCURRENT_EXPANSIONS cap affects user experience
  - **Mitigation**: Document in user-facing docs; justified by performance budget

## Files/Packages Affected
- `crates/maproom/src/search/results.rs` (add related field to ChunkSearchResult)
- `crates/maproom/src/search/pipeline.rs` OR `executors.rs` (integration logic)
- `crates/maproom/src/search/types.rs` OR params file (add include_related parameter)

## Verification Notes
The verify-ticket agent should check:
- `include_related` parameter exists and is optional (default false)
- `related` field exists on ChunkSearchResult with skip_serializing_if
- Integration tests exist:
  - `test_search_with_relationships` (high-confidence results have related field)
  - `test_confidence_gating` (only high-confidence expanded)
  - `test_backward_compatibility` (without parameter, no related field)
  - `test_max_concurrent_expansions_cap` (4+ high-confidence results, only 3 expanded)
  - `test_graceful_degradation` (graph error doesn't fail search)
- Tests pass with `cargo test search`
- Auto-enable confidence works (include_related=true enables confidence)
