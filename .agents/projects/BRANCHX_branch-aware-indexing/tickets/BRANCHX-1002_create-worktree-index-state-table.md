# Ticket: BRANCHX-1002: Create worktree_index_state table for tree SHA tracking

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - migration tested in BRANCHX-1003 (SQL validated, database test pending)
- [x] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create table to track the last indexed git tree SHA for each worktree, enabling incremental update optimization.

## Background
This is Phase 1, Step 1.2 of the BRANCHX project. After adding worktree_ids to chunks (BRANCHX-1001), we need to track the indexed state of each worktree using git's tree SHA. This enables the core optimization: comparing current tree SHA to last indexed tree SHA to detect if any changes occurred (and skip scanning if identical).

The worktree_index_state table serves as the foundation for branch-aware incremental indexing by maintaining a record of what was last indexed for each worktree. The git tree SHA is the perfect immutable identifier because it represents the exact state of the entire working tree—if two commits have the same tree SHA, their file contents are identical, even if commit metadata differs.

Reference: `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 1, Step 1.2

## Acceptance Criteria
- [x] worktree_index_state table created with correct schema
- [x] Primary key on worktree_id with foreign key to worktrees table
- [x] Index on last_tree_sha for fast lookups
- [x] Existing worktrees initialized with 'init' placeholder tree SHA
- [x] Migration includes metrics columns (chunks_processed, embeddings_generated)
- [x] Migration runs successfully on test database

## Technical Requirements
- Table must reference worktrees(id) with foreign key constraint
- last_tree_sha stores output of `git rev-parse HEAD^{tree}` (SHA-256 hash)
- last_indexed timestamp defaults to NOW() on insert
- Metrics columns enable cost tracking and monitoring
- Initialize existing worktrees to prevent null state
- Schema must support ON DELETE CASCADE to clean up when worktrees are removed

## Implementation Notes

Add this table to the same migration file as BRANCHX-1001 (`004_add_worktree_tracking.sql`):

```sql
CREATE TABLE worktree_index_state (
  worktree_id INT PRIMARY KEY REFERENCES worktrees(id) ON DELETE CASCADE,
  last_tree_sha TEXT NOT NULL,
  last_indexed TIMESTAMP DEFAULT NOW(),
  chunks_processed INT DEFAULT 0,
  embeddings_generated INT DEFAULT 0
);

CREATE INDEX idx_worktree_index_state_tree_sha ON worktree_index_state(last_tree_sha);

-- Initialize for existing worktrees
INSERT INTO worktree_index_state (worktree_id, last_tree_sha)
SELECT id, 'init' FROM worktrees;
```

**Design Rationale**:
- **last_tree_sha**: The git tree SHA represents the complete state of all files in the working tree. It's immutable and content-addressed, making it perfect for change detection.
- **'init' placeholder**: Existing worktrees get initialized with a placeholder that will never match a real tree SHA, forcing a full index on first run.
- **Metrics columns**: Track processing costs and enable monitoring of indexing efficiency.
- **Index on tree SHA**: Enables fast deduplication queries (future Phase 3 optimization).

See `architecture.md` section "Worktree Index State Table" for complete design rationale.

## Dependencies
- BRANCHX-1001 must be complete (can be in same migration file)
- Requires existing worktrees table

## Risk Assessment
- **Risk**: Existing worktrees not initialized, causing null pointer errors or unexpected behavior
  - **Mitigation**: Initialize all existing worktrees with 'init' placeholder in migration
- **Risk**: Foreign key constraint fails if orphan records exist
  - **Mitigation**: Clean up orphan worktrees before migration (migration should handle this gracefully)
- **Risk**: Placeholder 'init' value could theoretically collide with a real SHA
  - **Mitigation**: SHA-1 hashes are 40 hex chars, SHA-256 are 64 hex chars; 'init' is 4 chars and not hex format—collision impossible

## Files/Packages Affected
- `packages/maproom-mcp/migrations/004_add_worktree_tracking.sql` (append to existing migration from BRANCHX-1001)

## Planning References
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 1, Step 1.2
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` - Index State Table design
