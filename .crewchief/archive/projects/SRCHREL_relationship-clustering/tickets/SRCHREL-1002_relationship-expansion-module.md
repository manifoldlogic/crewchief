# Ticket: [SRCHREL-1002]: Relationship Expansion Module

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 9 passed, 0 failed (cargo test -p crewchief-maproom relationships --lib)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-expert
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the relationship expansion module with edge weight computation, module proximity detection, and top-N related chunk selection with relevance scoring.

## Background
The relationship expansion module performs shallow graph traversal (depth=2) to find related chunks, computes weighted relevance scores based on edge type and module proximity, and returns the top 5 most relevant chunks. This is the core logic for relationship-aware search.

This implements Phase 1 deliverables: relationships.rs module, find_top_related_chunks(), compute_edge_weight(), module proximity boost.

## Acceptance Criteria
- [ ] New file `crates/maproom/src/search/relationships.rs` created
- [ ] `find_top_related_chunks()` function implemented (async, returns Vec<RelatedChunkResult>)
- [ ] `compute_edge_weight()` function implemented (edge_type, target_kind → weight)
- [ ] `extract_parent_dir()` function implemented (path → parent directory string)
- [ ] `truncate_preview()` function implemented (content → 100-char preview with "...")
- [ ] Edge weight constants defined: EDGE_WEIGHT_DEFAULT (1.0), EDGE_WEIGHT_TEST_PENALTY (0.5), EDGE_WEIGHT_INHERITANCE_BOOST (1.1)
- [ ] Module proximity boost (1.2×) applied for same-directory chunks
- [ ] Unit tests pass for all helper functions
- [ ] Module exported in `crates/maproom/src/search/mod.rs`

## Technical Requirements

### Edge Weight Constants
```rust
const EDGE_WEIGHT_DEFAULT: f32 = 1.0;
const EDGE_WEIGHT_TEST_PENALTY: f32 = 0.5;
const EDGE_WEIGHT_INHERITANCE_BOOST: f32 = 1.1;
```

### compute_edge_weight() Implementation
```rust
fn compute_edge_weight(edge_type: &str, target_kind: &str) -> f32 {
    match (edge_type, target_kind) {
        ("extends" | "implements", _) => EDGE_WEIGHT_INHERITANCE_BOOST,
        (_, kind) if kind.contains("test") => EDGE_WEIGHT_TEST_PENALTY,
        _ => EDGE_WEIGHT_DEFAULT,
    }
}
```

### find_top_related_chunks() Signature
```rust
pub async fn find_top_related_chunks(
    store: &SqliteStore,
    source_chunk_id: i64,
    limit: usize,
) -> Result<Vec<RelatedChunkResult>>
```

### Algorithm Steps
1. Get source chunk metadata (for module detection)
2. Call existing `find_related_chunks(store, source_chunk_id, depth=2, None)`
3. Compute relevance: base_relevance × edge_weight × module_boost
4. Sort by relevance descending
5. Take top N (limit parameter)
6. Convert to RelatedChunkResult

## Implementation Notes

Reuse existing graph traversal infrastructure:
- `find_related_chunks()` function already handles depth limiting and cycle detection
- Focus on relevance scoring and top-N selection logic

Module proximity detection:
```rust
fn extract_parent_dir(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string()
}
```

Apply module boost:
```rust
let module_boost = if extract_parent_dir(&chunk.relpath) == source_dir {
    1.2
} else {
    1.0
};
```

Preview truncation:
```rust
fn truncate_preview(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        content.to_string()
    } else {
        format!("{}...", &content[..max_length])
    }
}
```

## Dependencies
- SRCHREL-1001 (RelatedChunkResult type must exist)
- Existing graph infrastructure: `find_related_chunks()` function

## Risk Assessment
- **Risk**: Existing `find_related_chunks()` signature incompatible or missing
  - **Mitigation**: Review existing context tool implementation; adapt if needed
- **Risk**: Relevance scoring produces unexpected rankings
  - **Mitigation**: Unit tests with known edge weights and module structures validate scoring

## Files/Packages Affected
- `crates/maproom/src/search/relationships.rs` (new file)
- `crates/maproom/src/search/mod.rs` (add module export)

## Verification Notes
The verify-ticket agent should check:
- All 5 functions are implemented
- Unit tests exist for:
  - `test_edge_weight_computation` (all edge types)
  - `test_module_proximity_boost` (same vs different directory)
  - `test_relevance_sorting` (higher relevance ranks first)
  - `test_preview_truncation` (content > 100 chars)
  - `test_empty_related_chunks` (no relationships found)
  - `test_fewer_than_limit` (2 chunks found, limit=5)
- Tests pass with `cargo test relationships`
- Module compiles without warnings
