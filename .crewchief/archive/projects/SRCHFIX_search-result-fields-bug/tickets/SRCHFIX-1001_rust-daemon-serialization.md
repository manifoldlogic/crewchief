# Ticket: [SRCHFIX-1001]: Update Rust Daemon Serialization

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - all daemon and search tests passing (1000+ tests)
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
Add `chunk_id` field to daemon JSON response serialization so search results include the chunk identifier needed for context retrieval.

## Background
The Rust daemon currently omits the `chunk_id` field when serializing SearchHit results to JSON, even though the field is present in the Rust struct and database. This causes downstream TypeScript clients to receive invalid chunk_id values (0), breaking context retrieval functionality.

This ticket implements Task 1.1 from the execution plan: Update Rust Daemon Serialization.

## Acceptance Criteria
- [x] Daemon serializes `chunk_id` field in search hit JSON responses
- [x] `cargo build` succeeds with no errors
- [x] `cargo clippy` passes with no warnings related to the change
- [x] Serialized JSON includes all required fields: chunk_id, score, start_line, end_line, symbol_name, kind, file_path

## Technical Requirements
- Add `"chunk_id": hit.chunk_id` to the `serde_json::json!` macro in daemon search response serialization
- Maintain existing field order and formatting
- No changes to SearchHit struct definition (already has chunk_id field)
- No changes to database queries (already retrieve chunk_id)

## Implementation Notes
**File**: `/workspace/crates/maproom/src/daemon/mod.rs` (line 332-340)

**Current code**:
```rust
.map(|hit| {
    serde_json::json!({
        "score": hit.score,
        "start_line": hit.start_line,
        "end_line": hit.end_line,
        "symbol_name": hit.symbol_name,
        "kind": hit.kind,
        "file_path": hit.file_relpath,
    })
})
```

**Updated code**:
```rust
.map(|hit| {
    serde_json::json!({
        "chunk_id": hit.chunk_id,        // ADD THIS LINE
        "score": hit.score,
        "start_line": hit.start_line,
        "end_line": hit.end_line,
        "symbol_name": hit.symbol_name,
        "kind": hit.kind,
        "file_path": hit.file_relpath,
    })
})
```

**Pattern**: This follows the existing serialization pattern used throughout the daemon codebase.

## Dependencies
- None (Phase 1 tasks run in parallel)

## Risk Assessment
- **Risk**: Adding chunk_id may cause type errors if TypeScript interface isn't updated
  - **Mitigation**: SRCHFIX-1002 updates TypeScript interface before integration
- **Risk**: Existing JSON consumers may not expect chunk_id field
  - **Mitigation**: JSON fields are additive; existing parsers ignore unknown fields

## Files/Packages Affected
- `/workspace/crates/maproom/src/daemon/mod.rs`

## Verification Notes
Verify the JSON response includes the chunk_id field by:
1. Checking the serialization code includes the line
2. Running `cargo build` and `cargo clippy` successfully
3. Confirm no regression in existing daemon functionality
