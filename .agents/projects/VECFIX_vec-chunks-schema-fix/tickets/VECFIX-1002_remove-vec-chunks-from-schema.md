# Ticket: VECFIX-1002: Remove vec_chunks from schema.rs

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
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Clean up legacy `vec_chunks` table definition from schema.rs to complete the migration to the new embeddings architecture.

## Background
The `schema.rs` file contains a deprecated virtual table creation statement for `vec_chunks` (lines 99-105). This table was part of the old architecture where embeddings were stored directly alongside chunk data. Migration 6 already handles dropping this table in production databases, but the schema definition creates confusion for developers and could mislead future modifications.

This ticket is part of Phase 1 of the VECFIX project and follows VECFIX-1001, which removes the deprecated code that uses this schema. Keeping the schema definition after removing all code that references it creates unnecessary maintenance burden and potential for bugs.

This implements the cleanup work described in the [VECFIX plan.md](../planning/plan.md#vecfix-1002-remove-vec_chunks-from-schemars) Phase 1.

## Acceptance Criteria
- [ ] No references to `vec_chunks` in `schema.rs`
- [ ] Code compiles without errors
- [ ] Comments updated to reflect removal of vec_chunks
- [ ] init_schema() function remains valid (even though deprecated)

## Technical Requirements
- Remove the `CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks` statement (lines 99-105)
- Update any comments in schema.rs that reference the old vec_chunks table
- Verify that the schema.rs file maintains its structural integrity after removal
- Ensure the deprecation notice at the top of the file remains accurate

## Implementation Notes

**Location**: `crates/maproom/src/db/sqlite/schema.rs`

**Lines to Remove**: 99-105
```rust
conn.execute(
    "CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
        chunk_id INTEGER PRIMARY KEY,
        code_embedding float[1536],
        text_embedding float[1536]
    )",
    [],
)?;
```

**Important Context**:
- The `init_schema()` function is already marked as deprecated and unused (lines 4-6)
- Schema creation is now handled by the migration system in `migrations.rs`
- This function is kept for reference only, but should not contain misleading legacy definitions
- The migration system (migration 6) already handles dropping `vec_chunks` in production databases
- The new architecture uses `code_embeddings` table (deduped by blob_sha) and `vec_code` virtual table

**Comments to Update**:
- Review lines 95-105 for any comments about vector tables
- Ensure comments accurately reflect that only `vec_code` is used (not `vec_chunks`)

**Verification**:
- Run `cargo build -p crewchief-maproom` to verify compilation
- Run `rg "vec_chunks" crates/maproom/src/db/sqlite/schema.rs` to verify complete removal
- Verify no other files in the schema module reference the removed lines

## Dependencies
- **VECFIX-1001**: Code that references vec_chunks must be removed first
  - Status: Must be completed before this ticket
  - Reason: Prevents compilation errors from code calling removed schema

## Risk Assessment
- **Risk**: Test failures if tests rely on schema.rs for table creation
  - **Mitigation**: Tests use the migration system, not `init_schema()`. The deprecated function is marked `#[allow(dead_code)]` and is not called anywhere in the codebase. Test suite will verify no regressions.

- **Risk**: Confusion about which tables are active vs deprecated
  - **Mitigation**: This removal clarifies the current architecture by removing legacy definitions. The deprecation notice at the top of the file explains that migrations.rs is the source of truth.

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/schema.rs` - Remove vec_chunks table definition (lines 99-105) and update related comments
