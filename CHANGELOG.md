# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - BRANCHX: Branch-Aware Indexing

**Core Features**:
- **Worktree tracking**: Chunks now track which worktrees/branches contain them via JSONB array (`worktree_ids`)
- **Incremental updates**: Only scan files that changed since last index (5-10x faster for typical branch switches)
- **Tree SHA optimization**: Instant detection of unchanged repositories (<100ms vs minutes)
- **Content deduplication**: Same code across branches shares storage and embeddings (via `blob_sha`)
- **Branch-specific search**: MCP search tool accepts `worktree` parameter to filter results

**CLI Updates**:
- `maproom scan` uses incremental mode by default (tree SHA comparison + git diff-tree)
- `maproom scan --force` bypasses tree SHA optimization for full repository scan
- Scan stats include cache hit rate and files processed

**Performance**:
- No changes: <100ms (tree SHA match, skip scan)
- Branch switch (20% changed): 1-2 min (vs 5-10 min full scan)
- Embedding cache hit rate: 80% for typical branches

**See also**: [Branch-Aware Indexing Architecture](/docs/architecture/branch-aware-indexing.md)

### Changed

**Database schema** (Migration 004 required):
- Added `worktree_ids JSONB` column to `chunks` table
- Added `worktree_index_state` table for tree SHA tracking
- Added GIN index on `worktree_ids` for efficient JSONB queries

**Breaking changes**:
- Existing installations must run migration 004 before upgrading
- Chunks table structure changed (backward compatible for queries)

### Migration

**For existing Maproom installations**:

1. Backup your database:
```bash
docker exec maproom-postgres pg_dump -U maproom maproom > backup.sql
```

2. Apply migration 004:
```bash
psql -h localhost -p 5433 -U maproom -d maproom \
  -f packages/maproom-mcp/migrations/004_add_worktree_tracking.sql
```

3. Re-index your worktrees (recommended):
```bash
npx @crewchief/maproom-mcp scan /path/to/repo --force
```

**Migration 004 changes**:
- Adds `worktree_ids` column with backfill from existing `files.worktree_id`
- Creates GIN index for JSONB queries
- Creates `worktree_index_state` table for tree SHA tracking
- Initializes state with placeholder 'init' tree SHA

**Rollback** (if needed):
```sql
DROP TABLE IF EXISTS maproom.worktree_index_state;
DROP INDEX IF EXISTS maproom.idx_chunks_worktree_ids;
ALTER TABLE maproom.chunks DROP COLUMN IF EXISTS worktree_ids;
```

### Fixed
- **watch**: Fixed file change detection misclassifying modified files as new files
  - **Root cause**: Path format mismatch between file watcher (absolute paths) and database (relative paths)
  - **Impact**: Watch command now correctly re-indexes modified files with updated timestamps
  - **Security**: Added file size limits (10MB) to prevent DoS attacks
  - **Security**: Added path traversal protection in normalization utility
  - **Related**: See `.agents/projects/WATCHFIX_watch-change-detection-fix/` for detailed analysis
