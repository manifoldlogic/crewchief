# Ticket: BRANCHX-1001: Add worktree_ids JSONB column to chunks table

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass (deferred to BRANCHX-1003)
- [x] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add JSONB column to the chunks table to track which worktrees contain each chunk, with GIN index for efficient queries. This is the foundational schema change enabling branch-aware indexing.

## Background
This is the first ticket in the BRANCHX project (Branch-Aware Indexing). After completing BLOBSHA, we have content-addressed storage with deduplication via blob_sha, but no mechanism to track which branches/worktrees contain which code chunks. This ticket adds the foundational schema change that enables worktree tracking.

The worktree_ids column will store an array of worktree IDs (as JSONB) for each chunk, allowing us to:
- Track which branches contain specific code
- Filter search results by worktree
- Support incremental updates per worktree
- Enable efficient branch switching workflows

Reference: `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 1, Step 1.1

## Acceptance Criteria
- [ ] Migration file `004_add_worktree_tracking.sql` created in `packages/maproom-mcp/migrations/`
- [ ] worktree_ids column added as JSONB NOT NULL DEFAULT '[]'
- [ ] Existing chunks backfilled with current worktree IDs from files table
- [ ] GIN index `idx_chunks_worktree_ids` created for efficient JSONB queries
- [ ] Migration runs successfully on test database without errors
- [ ] All existing chunks have non-empty worktree_ids after migration

## Technical Requirements
- Use JSONB type (not integer array) for PostgreSQL-native JSONB operators
- Default value must be empty array '[]' for backward compatibility with new rows
- GIN index must support JSONB contains (`?`) and overlaps (`?|`) operators
- Backfill query must correctly identify worktree for existing chunks via files table JOIN
- Handle null worktree_id gracefully during backfill (skip orphan chunks)
- Column must be NOT NULL after backfill completes
- Migration must be idempotent (safe to run multiple times)

## Implementation Notes

### Migration File Structure

Create `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql`:

```sql
-- Phase 1: Add column with default
ALTER TABLE chunks ADD COLUMN worktree_ids JSONB DEFAULT '[]';

-- Phase 2: Backfill existing chunks with their worktree ID
UPDATE chunks c
SET worktree_ids = jsonb_build_array(
  (SELECT w.id
   FROM worktrees w
   JOIN files f ON f.worktree_id = w.id
   WHERE f.id = c.file_id)
)
WHERE c.file_id IS NOT NULL;

-- Phase 3: Make NOT NULL after backfill
ALTER TABLE chunks ALTER COLUMN worktree_ids SET NOT NULL;

-- Phase 4: Create GIN index for efficient JSONB queries
CREATE INDEX idx_chunks_worktree_ids ON chunks USING gin(worktree_ids);
```

### Design Rationale

**Why JSONB over alternatives:**
- Integer array (`INT[]`): Less PostgreSQL operator support
- Junction table: Requires JOIN for every query, complex upsert
- Bitmask (`BIGINT`): Limited to 64 worktrees
- JSONB array: Unlimited worktrees, GIN index, PostgreSQL operators, readable

**Query patterns enabled:**
```sql
-- Find chunks in specific worktree
WHERE worktree_ids ? '2'

-- Find chunks in multiple worktrees (OR)
WHERE worktree_ids ?| ARRAY['2', '5']

-- Find chunks in all worktrees (AND)
WHERE worktree_ids ?& ARRAY['2', '5']
```

### Backfill Strategy

The backfill joins through the files table to find each chunk's worktree:
- chunks.file_id -> files.id
- files.worktree_id -> worktrees.id

For chunks without a valid file_id (orphans), the WHERE clause filters them out, leaving their worktree_ids as '[]'. These will be cleaned up by future maintenance tasks.

See `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` section "Database Schema Design" for full design rationale and query patterns.

## Dependencies
- Requires BLOBSHA project complete (content-addressed storage with blob_sha)
- Requires existing worktrees table (schema from earlier migrations)
- Requires existing chunks table with file_id foreign key to files table

## Risk Assessment

- **Risk**: Backfill may fail if orphan chunks exist (file_id references deleted file)
  - **Mitigation**: Add WHERE clause to filter chunks with valid file_id, leaving orphans with empty array

- **Risk**: Migration slow on large chunks table (100K+ rows)
  - **Mitigation**: Test migration on copy of production data first, add timing logs

- **Risk**: Index creation may lock table during concurrent operations
  - **Mitigation**: Run migration during low-traffic period, use CREATE INDEX CONCURRENTLY if needed

- **Risk**: NOT NULL constraint may fail if backfill incomplete
  - **Mitigation**: Verify all chunks have worktree_ids populated before setting NOT NULL, use DEFAULT '[]' as safety net

## Files/Packages Affected
- `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql` (new file)

## Planning References
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 1, Step 1.1
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` - Database Schema Design section
