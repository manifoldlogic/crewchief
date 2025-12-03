# Ticket: [DIM1024-1001]: Database Foundation for 1024-Dimensional Embeddings

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
- rust-developer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add database support for 1024-dimensional embeddings by creating Migration #10 for the vec_code_1024 virtual table and updating dimension constants across the codebase.

## Background
This ticket implements Phase 1 of the DIM1024 project to enable mxbai-embed-large model support. The implementation follows the established pattern from Migration #7 (768-dim support), adding 1024 to the existing dimension infrastructure.

The mxbai-embed-large model requires 1024-dimensional embedding storage, and this ticket creates the database foundation by adding a new virtual table and updating all dimension mapping logic consistently across embeddings.rs, vector.rs, and columns.rs.

References: `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/plan.md` (Phase 1), `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/architecture.md` (Component Design).

## Acceptance Criteria
- [ ] Migration #10 is added to migrations list in migrations.rs
- [ ] `SUPPORTED_DIMENSIONS` contains [768, 1024, 1536] in embeddings.rs
- [ ] `SUPPORTED_DIMENSIONS` contains [768, 1024, 1536] in vector.rs
- [ ] `SUPPORTED_DIMENSIONS` contains [768, 1024, 1536] in columns.rs
- [ ] `get_vec_table_name(1024)` returns "vec_code_1024" in embeddings.rs
- [ ] `get_vec_table_name(1024)` returns "vec_code_1024" in vector.rs
- [ ] Unit test: 1024-dim embedding stored successfully
- [ ] Unit test: 1024-dim embedding syncs to vec_code_1024 table
- [ ] All existing unit tests still pass (768/1536 dimensions unaffected)
- [ ] Migration #10 runs successfully and is idempotent

## Technical Requirements
- Add Migration #10 with version 10, name "add_vec_code_1024"
- Migration up: `CREATE VIRTUAL TABLE vec_code_1024 USING vec0(embedding float[1024]);`
- Migration down: `DROP TABLE IF EXISTS vec_code_1024;`
- Update SUPPORTED_DIMENSIONS constant to `&[768, 1024, 1536]` in three locations
- Add match case `1024 => Ok("vec_code_1024")` in dimension mapping functions
- Update error messages to include 1024 in supported dimensions list
- Add ColumnSet::MXBAI mapping for dimension 1024 in columns.rs

## Implementation Notes
**Follow Existing Pattern**: This implementation follows the exact pattern established by Migration #7 for 768-dimensional support. Review Migration #7 code before implementing.

**Migration Order**: Migration #10 is the next sequential number after Migration #9 (context_cache).

**Virtual Table Requirement**: sqlite-vec requires separate virtual tables for each dimension. Virtual tables have fixed dimensions and cannot store multiple dimensions.

**Three Locations to Update**:
1. `/workspace/crates/maproom/src/db/sqlite/embeddings.rs` - Storage logic
2. `/workspace/crates/maproom/src/db/sqlite/vector.rs` - Search logic
3. `/workspace/crates/maproom/src/db/columns.rs` - PostgreSQL compatibility

**Note on Duplication**: The get_vec_table_name() function is duplicated in embeddings.rs and vector.rs. This is suboptimal but out of scope to refactor. Apply identical changes to both.

**Backward Compatibility**: All changes must be additive only. Existing 768-dim and 1536-dim embeddings must continue working without modification.

## Dependencies
- **sqlite-vec extension**: Must be statically linked in build (already satisfied)
- **No prerequisite tickets**: This is the foundation ticket for the DIM1024 project

## Risk Assessment
- **Risk**: Migration #10 breaks existing databases or fails on specific SQLite versions
  - **Mitigation**: Test migration idempotency (run twice), verify existing tables untouched, use exact pattern from Migration #7
- **Risk**: Dimension mismatch errors confuse users
  - **Mitigation**: Clear error messages listing all supported dimensions [768, 1024, 1536]
- **Risk**: Missing dimension mapping in one of the three locations causes runtime errors
  - **Mitigation**: Update all three files consistently, verify with unit tests that check all code paths

## Files/Packages Affected
- `/workspace/crates/maproom/src/db/sqlite/migrations.rs`
- `/workspace/crates/maproom/src/db/sqlite/embeddings.rs`
- `/workspace/crates/maproom/src/db/sqlite/vector.rs`
- `/workspace/crates/maproom/src/db/columns.rs`

## Verification Notes
The verify-ticket agent should specifically check:

1. **Migration #10 exists**: Verify it's in the migrations list with correct version number
2. **Table creation SQL**: Confirm `CREATE VIRTUAL TABLE vec_code_1024 USING vec0(embedding float[1024]);`
3. **Constant consistency**: All three SUPPORTED_DIMENSIONS arrays contain [768, 1024, 1536]
4. **Mapping consistency**: Both get_vec_table_name() functions return "vec_code_1024" for dimension 1024
5. **Test execution**: Unit tests were actually RUN (not just created) and show passing output
6. **Regression prevention**: Existing 768-dim and 1536-dim tests still pass
7. **Idempotency**: Migration can be run multiple times without errors (test with in-memory database)
8. **Error messages**: Unsupported dimension errors list all three supported dimensions
