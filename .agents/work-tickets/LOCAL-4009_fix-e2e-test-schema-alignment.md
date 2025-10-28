# Ticket: LOCAL-4009: Fix E2E Test Schema Alignment Issues

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- test-runner
- verify-ticket
- commit-ticket

## Summary
Fix the 4 failing E2E tests in `e2e_workflow_simple.rs` by aligning test expectations with the actual database schema. Tests were written against assumed schema but actual schema uses different column names, data types, and table relationships.

## Background
LOCAL-4004 successfully created E2E test infrastructure with 3/7 tests passing (stack health, embedding integration). However, 4 tests are failing due to schema mismatches between test expectations and the actual database schema implemented in the Rust migrations.

The test infrastructure is solid and execution is fast (1.53s), but tests need to be updated to use:
- Correct column names (`preview` not `content`, `code_embedding`/`text_embedding` not `embedding`)
- Correct table relationships (JOIN `files` via `file_id` for `relpath`)
- Correct data types (`i64/bigint` not `i32`)

This is a straightforward fix that will bring test coverage from 3/7 to 7/7 passing, validating the indexed data quality and search functionality before release.

## Acceptance Criteria
- [ ] All 7/7 E2E tests passing (currently 3/7)
- [ ] Test `test_02_indexed_data_validation` passes (uses correct JOIN and column names)
- [ ] Test `test_03_fts_search_functionality` passes (uses correct data types and columns)
- [ ] Test `test_04_embedding_quality` passes (uses `code_embedding`/`text_embedding` columns)
- [ ] Test `test_05_data_persistence` passes (uses correct schema throughout)
- [ ] No schema-related test failures in output
- [ ] Test execution time remains under 5 seconds
- [ ] Clear diagnostic output maintained for all tests

## Technical Requirements

### Schema Corrections Needed

**1. Embedding Column Names**
- **Current test expectation**: `chunks.embedding`
- **Actual schema**: `chunks.code_embedding` and `chunks.text_embedding`
- **Fix**: Query both columns, validate both have 768-dimensional embeddings
- **Affected tests**: `test_04_embedding_quality`

**2. File Path Column**
- **Current test expectation**: `chunks.rel_path` (direct column)
- **Actual schema**: `files.relpath` (requires JOIN via `chunks.file_id`)
- **Fix**: Add JOIN to files table: `JOIN files ON chunks.file_id = files.id`
- **Affected tests**: `test_02_indexed_data_validation`, `test_03_fts_search_functionality`

**3. Content Column Name**
- **Current test expectation**: `chunks.content`
- **Actual schema**: `chunks.preview`
- **Fix**: Change all references from `content` to `preview`
- **Affected tests**: `test_02_indexed_data_validation`, `test_03_fts_search_functionality`

**4. ID Data Types**
- **Current test expectation**: `i32` for all IDs
- **Actual schema**: `bigint` (maps to `i64` in Rust)
- **Fix**: Change all ID types from `i32` to `i64`
- **Affected tests**: All tests that query IDs

### Query Pattern Example

**Current (incorrect):**
```rust
let chunks: Vec<(i32, String, String)> = sqlx::query_as(
    "SELECT id, rel_path, content FROM chunks LIMIT 5"
)
.fetch_all(&pool)
.await?;
```

**Corrected:**
```rust
let chunks: Vec<(i64, String, String)> = sqlx::query_as(
    "SELECT c.id, f.relpath, c.preview
     FROM chunks c
     JOIN files f ON c.file_id = f.id
     LIMIT 5"
)
.fetch_all(&pool)
.await?;
```

### Embedding Validation Example

**Current (incorrect):**
```rust
let embeddings: Vec<(Vec<f32>,)> = sqlx::query_as(
    "SELECT embedding FROM chunks WHERE embedding IS NOT NULL LIMIT 10"
)
.fetch_all(&pool)
.await?;
```

**Corrected (check both columns):**
```rust
let code_embeddings: Vec<(Vec<f32>,)> = sqlx::query_as(
    "SELECT code_embedding FROM chunks WHERE code_embedding IS NOT NULL LIMIT 10"
)
.fetch_all(&pool)
.await?;

let text_embeddings: Vec<(Vec<f32>,)> = sqlx::query_as(
    "SELECT text_embedding FROM chunks WHERE text_embedding IS NOT NULL LIMIT 10"
)
.fetch_all(&pool)
.await?;

// Validate both types
for (embedding,) in code_embeddings {
    assert_eq!(embedding.len(), 768, "Code embedding dimension mismatch");
}
for (embedding,) in text_embeddings {
    assert_eq!(embedding.len(), 768, "Text embedding dimension mismatch");
}
```

## Implementation Notes

### Tests to Fix

1. **test_02_indexed_data_validation**
   - Add JOIN to files table for `relpath`
   - Change `content` to `preview`
   - Change ID types to `i64`

2. **test_03_fts_search_functionality**
   - Add JOIN to files table for `relpath` in results
   - Change `content` to `preview`
   - Change ID types to `i64`

3. **test_04_embedding_quality**
   - Query `code_embedding` and `text_embedding` separately
   - Validate both embedding types
   - Change ID types to `i64`

4. **test_05_data_persistence**
   - Apply all schema corrections from above tests
   - Ensure validation logic uses correct column names

### Testing Strategy

1. Run tests one at a time to verify each fix:
   ```bash
   cargo test --test e2e_workflow_simple test_02_indexed_data_validation -- --nocapture
   cargo test --test e2e_workflow_simple test_03_fts_search_functionality -- --nocapture
   cargo test --test e2e_workflow_simple test_04_embedding_quality -- --nocapture
   cargo test --test e2e_workflow_simple test_05_data_persistence -- --nocapture
   ```

2. Run full suite to verify all 7/7 pass:
   ```bash
   cargo test --test e2e_workflow_simple -- --nocapture --test-threads=1
   ```

3. Verify test execution time remains fast (< 5 seconds)

### Reference Schema (from migrations)

```sql
-- files table
CREATE TABLE files (
    id BIGSERIAL PRIMARY KEY,
    relpath TEXT NOT NULL,
    ...
);

-- chunks table
CREATE TABLE chunks (
    id BIGSERIAL PRIMARY KEY,
    file_id BIGINT NOT NULL REFERENCES files(id),
    preview TEXT,
    code_embedding vector(768),
    text_embedding vector(768),
    ...
);
```

## Dependencies
- **LOCAL-4004**: E2E indexing workflow tests (parent ticket, partially complete)
- **LOCAL-1009**: Fix database schema mismatch (schema definitions reference)
- Existing Docker stack running at `~/.maproom-mcp` with indexed data

## Risk Assessment

- **Risk**: Schema changes might have cascading effects on other tests
  - **Mitigation**: Tests are isolated and serial - each test independently validates specific aspects, no shared state beyond database

- **Risk**: Embedding column logic becomes more complex with two columns
  - **Mitigation**: Test both columns independently, maintain clear separation between code and text embeddings in validation logic

- **Risk**: JOIN operations might slow down tests
  - **Mitigation**: Only querying LIMIT 5-10 rows for validation, negligible performance impact (tests currently run in 1.53s)

- **Risk**: Future schema changes could break tests again
  - **Mitigation**: These tests validate actual production schema - future schema changes should intentionally update tests as part of migration work

## Files/Packages Affected
- `/workspace/crates/maproom/tests/e2e_workflow_simple.rs` - Fix all 4 failing tests with schema corrections
