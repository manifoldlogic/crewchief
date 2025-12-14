# Ticket: [SRCHREL-1001]: RelatedChunkResult Type Definition

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - type definition only, no behavioral tests; compilation verified, no regressions (1 pre-existing test failure unrelated)
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
Define the `RelatedChunkResult` struct in Rust with TYPE_SYNC comments and comprehensive field documentation for lightweight relationship metadata.

## Background
Relationship-aware search needs a lightweight type to represent related chunks discovered via graph traversal. This type must contain metadata only (no file content) to keep responses small and fast. The type will be mirrored in TypeScript for daemon client communication.

This implements the foundational type from Phase 1 (Rust Core Infrastructure) of the plan.

## Acceptance Criteria
- [ ] `RelatedChunkResult` struct defined in `crates/maproom/src/search/results.rs`
- [ ] All fields present: chunk_id, relpath, symbol_name, kind, start_line, end_line, preview, depth, relevance, relationship_type
- [ ] TYPE_SYNC comment references TypeScript type location
- [ ] Comprehensive field documentation including empty result semantics (None vs Some([]))
- [ ] Serde derives (Serialize, Deserialize) present
- [ ] Struct compiles without errors

## Technical Requirements
- Add `RelatedChunkResult` struct to `crates/maproom/src/search/results.rs`
- Include all fields specified in architecture.md
- Add `#[derive(Debug, Clone, Serialize, Deserialize)]` annotations
- Add TYPE_SYNC comment: `/// TYPE_SYNC: packages/daemon-client/src/types.ts::RelatedChunkResult`
- Document empty result semantics:
  - `Option::None`: Expansion did not run (confidence too low or disabled)
  - `Option::Some(vec![])`: Expansion ran but found no relationships
- Add inline documentation for relevance score computation formula

## Implementation Notes
Follow the exact structure from architecture.md Component 1:

```rust
/// Lightweight metadata for a related chunk discovered via graph traversal.
///
/// Contains only metadata (no file content) to keep responses small and fast.
/// Users can invoke context tool to retrieve full content for specific chunks.
///
/// Empty Result Semantics:
/// - `Option::None`: Expansion did not run (confidence too low or disabled)
/// - `Option::Some(vec![])`: Expansion ran but found no relationships
///
/// TYPE_SYNC: packages/daemon-client/src/types.ts::RelatedChunkResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedChunkResult {
    /// Chunk ID for requesting full context via context tool
    pub chunk_id: i64,

    /// File path relative to repository root
    pub relpath: String,

    /// Symbol name (function, class, etc.)
    pub symbol_name: Option<String>,

    /// Symbol kind (function, class, interface, etc.)
    pub kind: String,

    /// Start line in file (1-based)
    pub start_line: i32,

    /// End line in file (1-based)
    pub end_line: i32,

    /// Content preview (first 100 characters)
    pub preview: String,

    /// Graph traversal depth from source chunk (1 or 2)
    pub depth: i32,

    /// Decay-adjusted relevance score (0.0-1.0)
    ///
    /// Computed as: base_decay × edge_weight × module_boost
    /// - base_decay: 0.7^depth (depth 1: 0.7, depth 2: 0.49)
    /// - edge_weight: 0.5-1.1 based on edge type and target kind
    /// - module_boost: 1.2 if same module, else 1.0
    pub relevance: f32,

    /// Relationship type: "import", "call", "extends", "implements"
    pub relationship_type: String,
}
```

This type follows the pattern from SRCHCONF's `ConfidenceSignals` struct.

## Dependencies
- None (foundational type)

## Risk Assessment
- **Risk**: Field names or types diverge from TypeScript during implementation
  - **Mitigation**: TYPE_SYNC comment makes divergence explicit; Phase 2 validation tests will catch mismatches

## Files/Packages Affected
- `crates/maproom/src/search/results.rs` (add RelatedChunkResult struct)
- `crates/maproom/src/search/mod.rs` (may need to export if not already re-exported)

## Verification Notes
The verify-ticket agent should check:
- Struct compiles successfully with `cargo build`
- All 10 fields are present with correct types
- TYPE_SYNC comment is present and correctly formatted
- Field documentation is comprehensive
- Empty result semantics are documented
- Serde derives are present
