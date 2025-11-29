# Ticket: COMPFIX-2004: Fix Scan blob_sha Null Constraint Violation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - integration test (scan execution) passed successfully
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

Fix critical bug in Rust indexer (crates/maproom/) where scan attempts to insert chunks with `blob_sha = null`, violating the database NOT NULL constraint. This bug completely blocks all E2E validation testing and most error scenario testing for the competition framework validation project.

## Background

During execution of COMPFIX-2002 (End-to-End Validation) and COMPFIX-2003 (Error Scenario Testing), the pre-flight validation correctly detected that the base branch was not indexed and instructed the user to run:

```bash
crewchief-maproom scan --path /workspace --commit HEAD
```

However, when following this instruction, the scan itself fails with a database constraint violation:

```
Error: scan failed for main@HEAD

Caused by:
    0: db error: ERROR: null value in column "blob_sha" of relation "chunks" violates not-null constraint
    DETAIL: Failing row contains (170423, 4356, null, module, null, null, 1, 18, ...)
```

This creates a critical blocking issue:
- Cannot scan base branch → Cannot run any competitions
- Cannot index worktrees → Cannot validate agent tool access
- Cannot test end-to-end workflow → Cannot complete validation tickets
- Blocks 4/7 error scenario tests (all scenarios that require worktree scanning)

The `blob_sha` column in the chunks table has a NOT NULL constraint (see `packages/maproom-mcp/config/init.sql`), but the Rust indexer is attempting to insert chunks with null values for this column.

**Reference:** This bug was discovered during COMPFIX-2002 validation and documented in:
- `.crewchief/projects/COMPFIX_competition-agent-setup-validation/validation-results/e2e-results.md`
- `.crewchief/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenarios.md`

## Acceptance Criteria

- [x] Scan completes successfully on crewchief repository (`/workspace`)
- [x] All inserted chunks have valid (non-null) blob_sha values
- [x] Database NOT NULL constraint is satisfied (no constraint violations)
- [x] Can successfully index base branch (main) at HEAD commit
- [x] Query `SELECT COUNT(*) FROM chunks WHERE blob_sha IS NULL` returns 0
- [x] Chunk count is greater than 0 after successful scan
- [x] Re-running the scan is idempotent (no errors on subsequent scans)
- [x] Can execute: `/workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD` without errors

## Technical Requirements

1. **Investigate blob_sha calculation logic**
   - Find where blob_sha should be computed in the indexer code
   - Determine why it's being set to null instead of a valid SHA
   - Review chunk creation and insertion code paths

2. **Fix null assignment**
   - Ensure blob_sha is computed from file content for each chunk
   - Verify the computation matches the expected format (likely SHA-256 or similar)
   - Handle edge cases (empty files, binary files, etc.)

3. **Add validation before database insert**
   - Validate that blob_sha is non-null before attempting to insert chunk
   - Add error logging if blob_sha is unexpectedly null
   - Fail fast with clear error message if validation fails

4. **Test on crewchief repository**
   - Run scan on `/workspace` (the crewchief codebase itself)
   - Verify scan completes without database errors
   - Verify all chunks have valid blob_sha values in database
   - Test with `--force` flag for full re-scan

5. **Verify database integrity**
   - Query chunks table to confirm all blob_sha values are non-null
   - Verify chunk count matches expected file/code structure
   - Confirm no orphaned or malformed data

## Implementation Notes

### Expected Behavior

The `blob_sha` should be a hash of the file content for the chunk. This is used for:
- Change detection (has this content changed?)
- Deduplication (same content in multiple files)
- Incremental indexing (skip unchanged chunks)

### Investigation Starting Points

**Files to check in `crates/maproom/src/`:**
- Chunk creation logic (likely in a module handling parsing/chunking)
- Database insertion code (where chunks are written to PostgreSQL)
- Hash calculation utilities (where blob_sha should be computed)
- Scan orchestration (main scan flow)

**Database schema reference:**
```sql
-- packages/maproom-mcp/config/init.sql
CREATE TABLE chunks (
  id BIGSERIAL PRIMARY KEY,
  document_id BIGINT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  blob_sha TEXT NOT NULL,  -- ← This constraint is being violated
  chunk_type TEXT NOT NULL,
  -- ... other columns
);
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

**Verify database:**
```bash
# Connect to database
docker exec -it maproom-postgres psql -U maproom -d maproom

# Check for null blob_sha values
SELECT COUNT(*) FROM chunks WHERE blob_sha IS NULL;

# Should return 0 after fix
```

**Full re-scan:**
```bash
/workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD --force
```

### Edge Cases to Consider

1. **Empty files** - Should still have a blob_sha (hash of empty content)
2. **Binary files** - May be filtered out by tree-sitter, but if indexed need valid hash
3. **Large files** - Ensure hash computation doesn't fail on large files
4. **Git-ignored files** - Should not be scanned, but if they are, need valid hash
5. **Incremental vs full scan** - Both modes should produce valid blob_sha values

## Dependencies

**Prerequisite Tickets:**
- None (this is a critical bug fix)

**External Dependencies:**
- PostgreSQL database running and accessible
- Rust toolchain for building the indexer
- crewchief repository at `/workspace` for testing

**Blocks the following tickets:**
- COMPFIX-2002: End-to-End Validation (currently blocked)
- COMPFIX-2003: Error Scenario Testing (4/7 scenarios blocked)

## Risk Assessment

- **Risk**: Fix may reveal additional scan bugs not yet discovered
  - **Mitigation**: Test incrementally, verify each step, add logging to identify issues early

- **Risk**: blob_sha calculation may be computationally expensive for large repos
  - **Mitigation**: Profile performance, ensure hashing is efficient, consider caching if needed

- **Risk**: Database may have existing null blob_sha values from previous failed scans
  - **Mitigation**: Clean up test data before validation, consider migration script if needed

- **Risk**: Fix may change blob_sha format, invalidating existing indexed data
  - **Mitigation**: Document expected format, verify consistency with existing valid chunks (if any)

## Files/Packages Affected

**Rust Indexer (`crates/maproom/src/`):**
- Chunk creation/parsing modules
- Database insertion code
- Hash calculation utilities
- Scan orchestration code
- Error handling for chunk validation

**Testing:**
- Existing Rust tests may need updates
- Integration tests for scan functionality
- Database integrity verification queries

**Database:**
- `chunks` table (via inserts, not schema changes)
- Existing data may need cleanup if test runs left invalid rows

## Validation Steps for verify-ticket

1. **Verify scan succeeds:**
   ```bash
   /workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD
   # Should complete without database errors
   ```

2. **Verify all blob_sha values are non-null:**
   ```bash
   docker exec -it maproom-postgres psql -U maproom -d maproom -c \
     "SELECT COUNT(*) FROM chunks WHERE blob_sha IS NULL;"
   # Should return: count = 0
   ```

3. **Verify chunks were created:**
   ```bash
   docker exec -it maproom-postgres psql -U maproom -d maproom -c \
     "SELECT COUNT(*) FROM chunks;"
   # Should return: count > 0 (thousands expected for crewchief repo)
   ```

4. **Verify idempotent re-scan:**
   ```bash
   /workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD
   # Should complete without errors (incremental scan, no changes)
   ```

5. **Verify force re-scan:**
   ```bash
   /workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD --force
   # Should complete without errors (full re-scan)
   ```

6. **Verify sample blob_sha values are valid:**
   ```bash
   docker exec -it maproom-postgres psql -U maproom -d maproom -c \
     "SELECT blob_sha, LENGTH(blob_sha) FROM chunks LIMIT 5;"
   # Should show non-null, consistent-length hash values
   ```

## Planning References

- **COMPFIX Planning:** `.crewchief/projects/COMPFIX_competition-agent-setup-validation/planning/`
- **Validation Results:** `.crewchief/projects/COMPFIX_competition-agent-setup-validation/validation-results/e2e-results.md`
- **Error Scenarios:** `.crewchief/projects/COMPFIX_competition-agent-setup-validation/validation-results/error-scenarios.md`
- **Database Schema:** `packages/maproom-mcp/config/init.sql`
- **Rust Indexer:** `crates/maproom/`

## Priority

**CRITICAL** - This bug blocks all remaining validation work:
- Cannot complete COMPFIX-2002 (End-to-End Validation)
- Cannot complete COMPFIX-2003 (Error Scenario Testing) - 4/7 scenarios blocked
- Cannot test the competition framework at all
- Without this fix, the entire validation phase is stalled

## Estimated Time

**1-3 hours**
- Investigation: 30-60 minutes
- Fix implementation: 30-60 minutes
- Testing and validation: 30-60 minutes

---

**Created:** 2025-11-10
**Phase:** 2 (Validation)
**Project:** COMPFIX_competition-agent-setup-validation
**Next Step:** Assign to rust-indexer-engineer agent to begin investigation and fix
