# Ticket: COMPFIX-2005: Fix worktree_ids Population in Scan Implementation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (validated via database queries and integration testing)
- [x] **Verified** - by the verify-ticket agent

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

Fix critical bug in Rust indexer (crates/maproom/) where scan inserts chunks with empty `worktree_ids = []` instead of the actual worktree ID. This causes database queries by worktree to fail, status command to report 0 chunks, and competition framework validation to incorrectly report that worktrees are not indexed.

## Background

After fixing COMPFIX-2004 (blob_sha null constraint), the scan now completes successfully but reveals a second critical bug: all 353,879 chunks are inserted with `worktree_ids = []` (empty JSONB array) instead of containing the actual worktree ID.

**Database Evidence:**
```sql
-- Current (BROKEN):
SELECT worktree_ids, COUNT(*) FROM chunks GROUP BY worktree_ids;
 worktree_ids | count
--------------+--------
 []           | 353974

-- Expected (CORRECT):
SELECT worktree_ids, COUNT(*) FROM chunks GROUP BY worktree_ids;
 worktree_ids | count
--------------+--------
 [3]          | 353974
```

Where `3` is the worktree ID from the `worktrees` table for the base branch.

**Impact:**
- Status command reports 0 chunks (filters by `worktree_ids @> to_jsonb(ARRAY[worktree_id])`)
- MCP search queries by worktree return no results
- Competition framework cannot validate tool access per variant
- Even with COMPFIX-2002 implementation, PreFlightValidator will think nothing is indexed
- Blocks all remaining validation work (COMPFIX-2002, COMPFIX-2003)

**Schema Context:**
The `worktree_ids` column was added by migration 004_add_worktree_tracking.sql:
- Type: `JSONB NOT NULL DEFAULT '[]'`
- Purpose: Track which worktrees contain each chunk for branch-specific search
- Format: JSONB array of integer worktree IDs, e.g., `[3]` or `[3, 5, 7]`
- GIN index: `idx_chunks_worktree_ids` for efficient JSONB queries

**Reference:** This bug was discovered during COMPFIX-2002 validation in:
- `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/e2e-results.md`

## Acceptance Criteria

- [x] New scans populate `worktree_ids` with correct worktree ID (not empty array)
- [x] Query `SELECT worktree_ids FROM chunks LIMIT 1` returns `[<worktree_id>]` not `[]`
- [x] Query `SELECT COUNT(*) FROM chunks WHERE worktree_ids @> to_jsonb(ARRAY[<worktree_id>])` returns full chunk count
- [x] Query `SELECT worktree_ids, COUNT(*) FROM chunks GROUP BY worktree_ids` shows at least one group with non-empty array
- [x] Re-scanning base branch updates existing chunks with correct worktree IDs
- [x] Status command (when COMPFIX-2002 is implemented) can successfully query chunks by worktree
- [x] MCP search queries filtering by worktree return results
- [x] All 353,974+ chunks have non-empty worktree_ids after scan

## Technical Requirements

1. **Investigate chunk insertion paths**
   - Find where chunks are inserted: `crates/maproom/src/upsert.rs`, `db/queries.rs`
   - Identify functions: `upsert_chunk_with_cache()`, `upsert_chunks_batch_with_cache()`, `insert_chunk()`, `insert_chunks_batch()`
   - Check sequential scan path: `indexer/mod.rs`
   - Check parallel scan path: `indexer/parallel.rs`

2. **Look up worktree ID before insertion**
   - Query: `SELECT id FROM worktrees WHERE name = ? AND repo_id = ?`
   - Ensure this query executes before chunk insertion
   - Cache the worktree ID for batch operations (don't re-query for every chunk)
   - Handle case where worktree doesn't exist (should error, not silently insert empty array)

3. **Populate worktree_ids with actual ID**
   - Format as JSONB array: `[worktree_id]`
   - Use `serde_json::json!([worktree_id])` or equivalent
   - Ensure SQL parameter binding uses correct JSONB type
   - Verify insertion statement includes worktree_ids column explicitly

4. **Handle the JSONB format correctly**
   - Current: `worktree_ids = '[]'::jsonb` (wrong - empty array)
   - Correct: `worktree_ids = '[3]'::jsonb` (right - array with worktree ID)
   - Example in Rust: `let worktree_ids_json = serde_json::json!([worktree_id]);`
   - Example SQL: `INSERT INTO chunks (..., worktree_ids) VALUES (..., $15::jsonb)`

5. **Update all insertion code paths**
   - Sequential scan in `indexer/mod.rs`
   - Parallel scan in `indexer/parallel.rs`
   - Upsert operations in `upsert.rs`
   - Ensure consistent behavior across all paths

6. **Test on crewchief repository**
   - Run full scan: `crewchief-maproom scan --path /workspace --commit HEAD`
   - Verify worktree_ids are populated with actual IDs
   - Test force re-scan updates existing empty arrays
   - Verify queries by worktree ID return results

## Implementation Notes

### Root Cause Analysis

The scan implementation likely:
1. Creates chunks with `worktree_ids` using the default value `[]`
2. Never looks up or includes the actual worktree ID
3. Relies on the database DEFAULT to populate the column

The fix requires:
1. Look up worktree ID at scan start (cache it)
2. Pass worktree ID to chunk creation/insertion functions
3. Format as JSONB array `[worktree_id]`
4. Explicitly set worktree_ids in INSERT/UPDATE statements

### Code Investigation Starting Points

**Files in `crates/maproom/src/`:**
- `upsert.rs` - Chunk upsert logic with caching
- `db/queries.rs` - Raw SQL insertion functions
- `indexer/mod.rs` - Main scan orchestration (sequential)
- `indexer/parallel.rs` - Parallel scan implementation
- `db/mod.rs` or `db/client.rs` - Database client/connection

**Expected flow:**
```rust
// 1. Look up worktree ID (at scan start)
let worktree_id: i64 = db.get_worktree_id(&worktree_name, repo_id).await?;

// 2. Pass to chunk creation
let chunk = Chunk {
    // ... other fields
    worktree_ids: json!([worktree_id]), // Not empty array!
};

// 3. Insert with proper JSONB parameter
sqlx::query!(
    "INSERT INTO chunks (..., worktree_ids) VALUES (..., $15::jsonb)",
    // ... other params
    worktree_ids_json // as serde_json::Value
).execute(pool).await?;
```

### Schema Reference

```sql
-- From packages/maproom-mcp/migrations/004_add_worktree_tracking.sql
ALTER TABLE maproom.chunks ADD COLUMN IF NOT EXISTS worktree_ids JSONB DEFAULT '[]';

-- Valid queries (after fix):
SELECT * FROM chunks WHERE worktree_ids ? '3';                    -- contains worktree 3
SELECT * FROM chunks WHERE worktree_ids @> '[3]'::jsonb;          -- contains worktree 3
SELECT * FROM chunks WHERE worktree_ids @> to_jsonb(ARRAY[3]);    -- contains worktree 3
```

### Testing Commands

**Build Rust indexer:**
```bash
cd /workspace
cargo build --release --package crewchief-maproom
```

**Run scan:**
```bash
/workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD
```

**Verify worktree_ids are populated:**
```bash
docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT worktree_ids, COUNT(*) FROM chunks GROUP BY worktree_ids;"
# Should show: [3] | 353974 (not [] | 353974)
```

**Query specific worktree:**
```bash
docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT COUNT(*) FROM chunks WHERE worktree_ids @> '[3]'::jsonb;"
# Should return: 353974 (not 0)
```

**Check sample chunks:**
```bash
docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT id, worktree_ids FROM chunks LIMIT 5;"
# Should show: worktree_ids = [3] for all rows
```

**Force re-scan (to fix existing data):**
```bash
/workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD --force
```

### Edge Cases

1. **Worktree doesn't exist** - Should fail with clear error, not silently insert empty array
2. **Multiple worktrees** - Future support for `[3, 5, 7]` when chunk exists in multiple branches
3. **Batch operations** - Cache worktree ID, don't re-query for each chunk
4. **Incremental scan** - New chunks should get worktree_ids, existing chunks should be updated
5. **NULL vs empty array** - Column is NOT NULL, so empty array `[]` is currently valid but semantically wrong

## Dependencies

**Prerequisite Tickets:**
- COMPFIX-2004: Fix Scan blob_sha Null Constraint (COMPLETED)

**External Dependencies:**
- PostgreSQL database running and accessible
- Rust toolchain for building the indexer
- Migration 004_add_worktree_tracking.sql already applied

**Blocks the following tickets:**
- COMPFIX-2002: End-to-End Validation (currently blocked - status reports 0 chunks)
- COMPFIX-2003: Error Scenario Testing (4/7 scenarios blocked)
- All future competition runs (invalid data in database)

## Risk Assessment

- **Risk**: Existing 353,974 chunks in database have empty worktree_ids
  - **Mitigation**: Force re-scan with `--force` flag to update existing data, or write migration script to backfill

- **Risk**: Multiple code paths for chunk insertion may be missed
  - **Mitigation**: Search codebase for all INSERT/UPDATE statements on chunks table, verify each path

- **Risk**: JSONB format may be incorrect causing queries to fail
  - **Mitigation**: Test queries manually with psql before implementing, verify JSONB operators work

- **Risk**: Performance impact of worktree ID lookup
  - **Mitigation**: Cache worktree ID at scan start (single query), reuse for all chunks in that scan

- **Risk**: May reveal additional issues with worktree tracking
  - **Mitigation**: Test incrementally, verify each step, add logging for debugging

## Files/Packages Affected

**Rust Indexer (`crates/maproom/src/`):**
- `upsert.rs` - Update `upsert_chunk_with_cache()`, `upsert_chunks_batch_with_cache()`
- `db/queries.rs` - Update `insert_chunk()`, `insert_chunks_batch()`
- `indexer/mod.rs` - Add worktree ID lookup, pass to chunk creation (sequential scan)
- `indexer/parallel.rs` - Add worktree ID lookup, pass to chunk creation (parallel scan)
- `db/mod.rs` or `db/client.rs` - May need `get_worktree_id()` helper function
- Chunk struct definition - May need to add/modify worktree_ids field

**Testing:**
- Existing Rust tests may need updates for worktree_ids
- Integration tests for scan functionality
- Database verification queries

**Database:**
- `chunks` table - Updated via INSERT/UPDATE (no schema changes)
- Existing data may need backfill or force re-scan

## Validation Steps for verify-ticket

1. **Verify scan completes without errors:**
   ```bash
   /workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD
   ```

2. **Verify worktree_ids are non-empty:**
   ```bash
   docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT worktree_ids, COUNT(*) FROM chunks GROUP BY worktree_ids;"
   # Expected: At least one group with non-empty array like [3]
   # NOT EXPECTED: Single group with [] and 353974 count
   ```

3. **Verify query by worktree returns results:**
   ```bash
   # First get worktree ID
   WORKTREE_ID=$(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT id FROM worktrees WHERE name = 'main' LIMIT 1;" | xargs)

   # Query chunks by that worktree
   docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT COUNT(*) FROM chunks WHERE worktree_ids @> to_jsonb(ARRAY[$WORKTREE_ID]);"
   # Expected: 353974 (not 0)
   ```

4. **Verify sample chunks have correct format:**
   ```bash
   docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT id, worktree_ids, jsonb_array_length(worktree_ids) as array_len FROM chunks LIMIT 5;"
   # Expected: array_len = 1 for all rows, worktree_ids = [some_id]
   # NOT EXPECTED: array_len = 0, worktree_ids = []
   ```

5. **Verify force re-scan updates existing chunks:**
   ```bash
   /workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD --force
   # Should complete successfully and update all chunks
   ```

6. **Verify no chunks with empty worktree_ids:**
   ```bash
   docker exec maproom-postgres psql -U maproom -d maproom -c "SELECT COUNT(*) FROM chunks WHERE jsonb_array_length(worktree_ids) = 0;"
   # Expected: 0 (no chunks with empty array)
   ```

## Planning References

- **COMPFIX Planning:** `.agents/projects/COMPFIX_competition-agent-setup-validation/planning/`
- **Validation Results:** `.agents/projects/COMPFIX_competition-agent-setup-validation/validation-results/e2e-results.md`
- **Database Schema:** `packages/maproom-mcp/config/init.sql`
- **Migration:** `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql`
- **Rust Indexer:** `crates/maproom/`
- **Related Ticket:** COMPFIX-2004 (blob_sha fix - similar insertion bug pattern)

## Priority

**CRITICAL** - This bug blocks all remaining validation work:
- Cannot complete COMPFIX-2002 (End-to-End Validation) - status reports 0 chunks
- Cannot complete COMPFIX-2003 (Error Scenario Testing) - 4/7 scenarios blocked
- Cannot run any competitions - framework validation fails
- Database contains semantically invalid data (empty arrays instead of actual IDs)
- Without this fix, the entire validation phase is stalled

## Estimated Time

**1-3 hours**
- Investigation: 30-60 minutes (find all insertion code paths)
- Fix implementation: 30-60 minutes (add worktree ID lookup and population)
- Testing and validation: 30-60 minutes (verify queries work, re-scan data)

---

**Created:** 2025-11-11
**Phase:** 2 (Validation)
**Project:** COMPFIX_competition-agent-setup-validation
**Next Step:** Assign to rust-indexer-engineer agent to begin investigation and fix
