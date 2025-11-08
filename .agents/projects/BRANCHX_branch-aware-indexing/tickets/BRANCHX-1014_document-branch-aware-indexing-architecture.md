# Ticket: BRANCHX-1014: Document branch-aware indexing architecture and usage

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - documentation reviewed and validated
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive documentation covering architecture, usage, migration guide, and CHANGELOG entry for the branch-aware indexing system.

## Background
This is Phase 5 (Documentation) of BRANCHX. After implementing and testing the complete system (Phases 1-4), we need to document the architecture, usage patterns, and migration guide for future developers and users. This documentation ensures the system can be understood, maintained, and used effectively.

Reference: `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 5 (lines 339-349)

## Acceptance Criteria
- [x] `docs/architecture/branch-aware-indexing.md` created with complete architecture documentation
- [x] `packages/maproom-mcp/README.md` updated with new schema details (worktree_ids, worktree_index_state)
- [x] `CHANGELOG.md` updated with BRANCHX features and breaking changes
- [x] Migration guide for existing installations included
- [x] Usage examples for CLI and MCP provided
- [x] All documentation reviewed and internally consistent

## Technical Requirements

### Architecture Documentation (`docs/architecture/branch-aware-indexing.md`)
- Explain problem statement and solution approach
- Document JSONB worktree tracking design
- Explain tree SHA optimization mechanism
- Detail incremental update algorithm
- Include schema diagrams (worktree_ids, worktree_index_state tables)
- Document query patterns and performance characteristics
- Provide migration guide

### README Updates (`packages/maproom-mcp/README.md`)
- Document worktree_ids JSONB column on chunks table
- Document worktree_index_state table structure
- Explain GIN index usage for JSONB queries
- Link to detailed architecture documentation

### CHANGELOG Updates (`CHANGELOG.md`)
- Document new features under "Added" section
- List breaking changes under "Changed" section
- Include migration instructions
- Follow existing CHANGELOG format

### Usage Examples
- CLI scan with incremental updates (default behavior)
- CLI scan with --force flag for full rescan
- MCP search with worktree filtering
- Performance comparison before/after

## Implementation Notes

### File 1: `docs/architecture/branch-aware-indexing.md`

Create comprehensive architecture document with structure:

```markdown
# Branch-Aware Indexing Architecture

## Overview
Problem: No branch tracking, must rescan on every update
Solution: Worktree tracking + tree SHA optimization

## Schema Design
- worktree_ids JSONB column (chunks table)
- worktree_index_state table (tree SHA tracking)
- GIN index for JSONB queries

## Incremental Update Algorithm
1. Get current tree SHA
2. Compare to last indexed SHA
3. If unchanged, skip (instant)
4. If changed, git diff-tree
5. Process only changed files

## Query Patterns
- Single worktree: WHERE worktree_ids ? '2'
- Multiple worktrees: WHERE worktree_ids ?| ARRAY['2', '5']

## Performance
- Tree SHA check: <100ms
- Incremental update (20% changed): 5-10x faster
- Branch switch (cached): <1s

## Migration Guide
Run migration 004:
psql -d maproom -f packages/maproom-mcp/migrations/004_add_worktree_tracking.sql
```

### File 2: `packages/maproom-mcp/README.md` (update)

Add schema documentation section or update existing:

```markdown
## Database Schema

### chunks table
- worktree_ids JSONB - Array of worktree IDs containing this chunk
- Uses GIN index for efficient JSONB queries

### worktree_index_state table
- worktree_id INT - Reference to worktrees table
- last_tree_sha TEXT - Git tree SHA of last indexed state
- last_indexed TIMESTAMP
- chunks_processed INT
- embeddings_generated INT
```

### File 3: `CHANGELOG.md` (update)

Add under `[Unreleased]` section:

```markdown
## [Unreleased]

### Added - BRANCHX: Branch-Aware Indexing
- Worktree tracking: chunks now track which worktrees/branches contain them
- Incremental updates: only scan changed files (5-10x faster)
- Tree SHA optimization: skip scanning if repository unchanged (<100ms)
- MCP search filtering: query specific worktree/branch
- CLI: `maproom scan` uses incremental by default, `--force` for full scan

### Changed
- Database schema: Added worktree_ids JSONB column to chunks table
- Added worktree_index_state table for tree SHA tracking
- Migration 004 required for existing installations

### Migration
Run migration 004:
```bash
psql -d maproom -f packages/maproom-mcp/migrations/004_add_worktree_tracking.sql
```
```

### File 4: Usage Examples (in README or architecture doc)

```markdown
## Usage Examples

### CLI: Incremental Scan (Default)
```bash
maproom scan --repo ~/myproject --worktree main
# First run: Full scan (5 minutes)
# Second run: <1 second (tree SHA match, skipped)

# Modify files
git commit -am "Changes"

maproom scan --repo ~/myproject --worktree main
# Incremental: Only scans changed files (20 seconds)
```

### CLI: Force Full Scan
```bash
maproom scan --repo ~/myproject --worktree main --force
```

### MCP: Search Specific Branch
```typescript
await search({
  query: 'authentication flow',
  worktree: 'main',  // Only search main branch
});
```
```

### Documentation Guidelines
- Link to planning documents for architectural rationale:
  - `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md`
  - `.agents/projects/BRANCHX_branch-aware-indexing/planning/analysis.md`
- Use clear, concise language
- Include concrete examples
- Ensure consistency with existing documentation style
- Test all code examples for accuracy

## Dependencies
- All Phase 1-4 tickets complete (BRANCHX-1001 through BRANCHX-1013)
- BRANCHX-1013 E2E tests pass
- Migration 004 implemented and tested

## Risk Assessment
- **Risk**: Documentation becomes stale as code evolves
  - **Mitigation**: Include links to planning docs, version with release, add note about doc maintenance
- **Risk**: Migration guide incomplete, users encounter issues
  - **Mitigation**: Test migration on clean database, document edge cases, provide troubleshooting section
- **Risk**: Usage examples don't match actual implementation
  - **Mitigation**: Verify all examples against current code, include version numbers

## Files/Packages Affected
- `docs/architecture/branch-aware-indexing.md` (new)
- `packages/maproom-mcp/README.md` (update)
- `CHANGELOG.md` (update)
- `docs/usage/incremental-updates.md` (optional, new)

## Planning References
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 5 (lines 339-349)
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` - Full architectural details
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/analysis.md` - Problem analysis and rationale
