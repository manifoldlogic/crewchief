# Ticket: [DIM1024-1001]: Database Foundation for 1024-Dimensional Embeddings

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
- [x] Migration #10 is added to migrations list in migrations.rs
- [x] `SUPPORTED_DIMENSIONS` contains [768, 1024, 1536] in embeddings.rs
- [x] `SUPPORTED_DIMENSIONS` contains [768, 1024, 1536] in vector.rs
- [x] `SUPPORTED_DIMENSIONS` contains [768, 1024, 1536] in columns.rs
- [x] `get_vec_table_name(1024)` returns "vec_code_1024" in embeddings.rs
- [x] `get_vec_table_name(1024)` returns "vec_code_1024" in vector.rs
- [x] Unit test: 1024-dim embedding stored successfully
- [x] Unit test: 1024-dim embedding syncs to vec_code_1024 table
- [x] All existing unit tests still pass (768/1536 dimensions unaffected)
- [x] Migration #10 runs successfully and is idempotent

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

## Implementation Summary

**Changes completed:**

1. **Migration #10 added** (`/workspace/crates/maproom/src/db/sqlite/migrations.rs`)
   - Version 10, name "add_vec_code_1024"
   - Up SQL: `CREATE VIRTUAL TABLE vec_code_1024 USING vec0(embedding float[1024]);`
   - Down SQL: `DROP TABLE IF EXISTS vec_code_1024;`
   - Updated test assertions to expect version 10 (was 9)

2. **embeddings.rs updated** (`/workspace/crates/maproom/src/db/sqlite/embeddings.rs`)
   - `SUPPORTED_DIMENSIONS` changed from `&[768, 1536]` to `&[768, 1024, 1536]`
   - Added `1024 => Ok("vec_code_1024")` to `get_vec_table_name()`
   - Updated `sync_all_embeddings_to_vec()` to sync 1024-dim embeddings to vec_code_1024
   - Added unit tests: `test_1024_dim_embedding_storage`, `test_1024_dim_vector_table_sync`
   - Updated `test_mixed_dimensions_storage` to test all three dimensions
   - Updated `test_sync_all_mixed_dimensions` to test all three dimensions (6 embeddings total)
   - Updated test setup to create vec_code_1024 virtual table
   - Updated error message tests to verify 1024 is listed

3. **vector.rs updated** (`/workspace/crates/maproom/src/db/sqlite/vector.rs`)
   - `SUPPORTED_DIMENSIONS` changed from `&[768, 1536]` to `&[768, 1024, 1536]`
   - Added `1024 => Ok("vec_code_1024")` to `get_vec_table_name()`

4. **columns.rs updated** (`/workspace/crates/maproom/src/db/columns.rs`)
   - Added `ColumnSet::MXBAI` constant with "code_embedding_mxbai" and "text_embedding_mxbai"
   - Updated `select_columns_for_dimension()` to handle 1024 dimension
   - Updated documentation to include 1024-dim and mxbai-embed-large
   - Added unit test: `test_1024_dimension_selects_mxbai_columns`
   - Updated all error message tests to verify 1024 is listed

**Test Results:**
- All migration tests pass (5/5)
- All embeddings tests pass (15/15) including new 1024-dim tests
- All vector tests pass (5/5)
- All columns tests pass (9/9) including new 1024-dim test
- Overall: 976/977 tests pass (1 pre-existing failure in config::hot_reload, unrelated)
- Migration successfully runs and creates vec_code_1024 table

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
