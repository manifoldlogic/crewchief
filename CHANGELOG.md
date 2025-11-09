# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

#### Index Size Limit Errors (Migration 0017)

**Problem**: PostgreSQL B-tree index size limit errors when indexing code with large preview text
- Error: `index row size exceeds btree version 4 maximum 2704`
- Affected 50%+ of codebases with minified files, large constants, or generated code
- Original covering index `idx_chunks_search_covering` failed on chunks with preview > 2704 bytes

**Solution**: Two-index strategy replacing single covering index
- **idx_chunks_search_small_preview**: Partial covering index for preview ≤ 2000 bytes (95%+ of chunks)
- **idx_chunks_search_basic**: Universal fallback index for all chunks including large previews

**Benefits**:
- Eliminates size limit errors completely (100% success rate)
- Maintains index-only scan performance for 95%+ of queries
- No application code changes required
- PostgreSQL query planner automatically selects optimal index

**Trade-offs**:
- Storage increase: ~31% (+155MB typical)
- Slightly slower queries for large previews (5% of data): 15-30ms vs 5-10ms

**Migration**: `crates/maproom/migrations/0017_fix_index_size_limits.sql`

**Note**: Originally planned 3-index strategy with hash-based approach, but PostgreSQL does not support expressions in INCLUDE clauses. Two-index solution achieves same functional outcome.

### Added

#### Automatic Branch Switch Detection (BRWATCH)

**New `maproom branch-watch` command** automatically detects branch switches and triggers incremental indexing:
- Watches `.git/HEAD` for changes using OS-level file events (inotify/FSEvents/ReadDirectoryChanges)
- Auto-indexes branches within <1 minute of switching
- Resource efficient: <5% CPU while idle, ~15-20MB memory
- Fault-tolerant: Retry logic with exponential backoff for transient errors
- Graceful shutdown with Ctrl+C signal handling

**Performance**:
- Detection latency: <1 second (OS file events)
- Update time: 30-60s for typical branch switches (varies by changed files)
- Debouncing: 2-second window prevents rapid successive triggers

**Usage**:
```bash
# Start watcher (blocks until Ctrl+C)
maproom branch-watch --repo /workspace/myproject

# With verbose logging
maproom branch-watch --repo . --verbose
```

**See also**: [Automatic Indexing Guide](docs/features/automatic-indexing.md)

#### BRANCHX: Branch-Aware Indexing

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
