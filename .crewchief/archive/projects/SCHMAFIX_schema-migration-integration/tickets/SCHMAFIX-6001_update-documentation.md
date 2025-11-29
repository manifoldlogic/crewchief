# Ticket: SCHMAFIX-6001: Update Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update project documentation to reflect that Rust owns all migrations, document the new schema elements (blob_sha, code_embeddings, worktree tracking), and add comments to migration SQL files explaining their purpose.

## Background
Before SCHMAFIX, migration SQL files existed in two places (MCP and Rust) with no clear ownership. Now, Rust is the single source of truth for all migrations. Documentation must be updated to prevent future confusion and to guide contributors on where to add new migrations. We also need to document the new schema elements so developers understand the blob_sha column (for future deduplication) and code_embeddings table (for vector search).

This ticket implements Phase 6 (Documentation) from `.crewchief/projects/SCHMAFIX_schema-migration-integration/planning/plan.md`.

## Acceptance Criteria
- [x] File `packages/maproom-mcp/migrations/README.md` updated to note Rust owns all migrations - CREATED with deprecation notice and migration mapping
- [x] File `crates/maproom/CLAUDE.md` updated to mention migrations 0018-0020 - Added Migrations section with details and guide
- [x] File `docs/architecture/DATABASE_ARCHITECTURE.md` updated with new schema (blob_sha, code_embeddings, worktree tracking) - Added comprehensive Schema section with tables, columns, indexes, and example queries
- [x] Migration SQL files (0018-0020) have clear header comments explaining purpose - ALREADY COMPLETE (added during SCHMAFIX-1001)
- [x] MCP migrations README explains relationship to Rust migrations (historical only) - Migration mapping table and clear deprecation notice added
- [x] All documentation changes reviewed for accuracy - All docs reference actual migration files and current schema state

## Technical Requirements
- Documentation format: Markdown
- Tone: Clear, concise, developer-focused
- Schema documentation: Include table definitions, column purposes, index descriptions
- Migration comments: Ticket ID, purpose, warnings (if any)

## Implementation Notes

### Part 1: Update MCP Migrations README
File: `packages/maproom-mcp/migrations/README.md`

Add section at top:
```markdown
# MCP Migrations (Historical)

⚠️ **DEPRECATED**: As of SCHMAFIX project (2025-11-09), Rust owns all migrations.

These migration files are **historical documentation only**. They were integrated into the Rust migration runner as migrations 0018-0020. Do NOT add new migrations here.

**For new migrations**: Add to `crates/maproom/migrations/` and update `crates/maproom/src/db/queries.rs`.

## Migration Mapping

| MCP Migration | Rust Migration | Purpose |
|---------------|----------------|---------|
| 001_add_blob_sha.sql | 0018_add_blob_sha.sql | Adds blob_sha column for content-addressed storage |
| 002_create_code_embeddings.sql | 0019_create_code_embeddings.sql | Creates deduplicated embeddings table with HNSW index |
| 004_add_worktree_tracking.sql | 0020_add_worktree_tracking.sql | Adds worktree_ids JSONB column and tracking table |
| 005_complete_branchx_schema.sql | 0021_complete_branchx_schema.sql | Completes BRANCHX worktree tracking schema |

## Why Rust Owns Migrations

- Rust binary is standalone (works without Node.js)
- Compile-time validation of SQL syntax via `include_str!` macro
- Single binary deployment
- Existing migration framework with transaction support
```

### Part 2: Update Rust CLAUDE.md
File: `crates/maproom/CLAUDE.md`

Add to "Migrations" section (or create if doesn't exist):
```markdown
## Migrations

Migrations 0000-0017: Original maproom schema
Migrations 0018-0020: BLOBSHA/BRANCHX integration (added SCHMAFIX project)

**Migration 0018** (add_blob_sha): Adds blob_sha TEXT column to chunks for content-addressed storage
**Migration 0019** (create_code_embeddings): Creates deduplicated embeddings table with HNSW index
**Migration 0020** (add_worktree_tracking): Adds worktree_ids JSONB column and worktree_index_state table
**Migration 0021** (complete_branchx_schema): Completes BRANCHX worktree tracking schema

**Adding New Migrations**:
1. Create SQL file in `crates/maproom/migrations/NNNN_description.sql`
2. Update `src/db/queries.rs` migrations array
3. Use `IF NOT EXISTS` for idempotency
4. Set `concurrent = false` for transaction safety
5. Write integration tests in `tests/migration_integration.rs`
```

### Part 3: Update DATABASE_ARCHITECTURE.md
File: `docs/architecture/DATABASE_ARCHITECTURE.md`

Add section documenting new schema elements:
```markdown
### Blob SHA Column (Migration 0018)

The `chunks` table includes a `blob_sha` column for content-addressed storage:

```sql
ALTER TABLE maproom.chunks ADD COLUMN blob_sha TEXT;
```

**Purpose**: Enable deduplication of embeddings based on content hash (git-compatible blob SHA).
**Status**: Column exists but not yet utilized (implementation pending BLOBSHA-IMPL project).
**Future**: Multiple chunks with identical content will reference same embedding in code_embeddings table.

### Code Embeddings Table (Migration 0019)

Deduplicated storage for code embeddings:

```sql
CREATE TABLE maproom.code_embeddings (
  id BIGSERIAL PRIMARY KEY,
  blob_sha TEXT NOT NULL UNIQUE,
  embedding vector(1536),
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_code_embeddings_hnsw ON maproom.code_embeddings
  USING hnsw (embedding vector_cosine_ops);
```

**Purpose**: Store one embedding per unique blob_sha, reducing embedding costs by 70-90%.
**Status**: Table exists but not yet populated (implementation pending BLOBSHA-IMPL project).
**Index**: HNSW index for fast vector similarity search.

### Worktree Tracking (Migrations 0020-0021)

BRANCHX schema for worktree-aware indexing:

```sql
-- worktree_ids JSONB column in chunks table
ALTER TABLE maproom.chunks
  ADD COLUMN worktree_ids JSONB DEFAULT '[]'::jsonb NOT NULL;

CREATE INDEX idx_chunks_worktree_ids ON maproom.chunks
  USING gin (worktree_ids);

-- Tracking table for worktree index state
CREATE TABLE maproom.worktree_index_state (
  worktree_id BIGINT PRIMARY KEY REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
  tree_sha TEXT,
  indexed_at TIMESTAMP DEFAULT NOW()
);
```

**Purpose**: Track which worktrees contain each chunk, enable incremental indexing.
**Status**: Schema complete, incremental update logic pending BRANCHX-IMPL project.
**JSONB Operators**: Use `?` (contains), `?|` (overlaps), `-` (remove) for querying.
```

### Part 4: Add Migration File Comments
For each migration file (0018, 0019, 0020, 0021), add header comment:

**Migration 0018**:
```sql
-- Migration: 0018_add_blob_sha.sql
-- Ticket: SCHMAFIX-1001
-- Purpose: Add blob_sha column to chunks table for content-addressed storage
-- Part of: BLOBSHA_content-addressed-chunk-storage project
-- Note: Column added but not yet populated (requires BLOBSHA-IMPL implementation)
```

**Migration 0019**:
```sql
-- Migration: 0019_create_code_embeddings.sql
-- Ticket: SCHMAFIX-1001
-- Purpose: Create deduplicated embeddings table with HNSW index
-- Part of: BLOBSHA_content-addressed-chunk-storage project
-- Note: Table created but not yet populated (requires BLOBSHA-IMPL implementation)
```

**Migration 0020**:
```sql
-- Migration: 0020_add_worktree_tracking.sql
-- Ticket: SCHMAFIX-2001
-- Purpose: Add worktree_ids JSONB column and tracking table for worktree-aware indexing
-- Part of: BRANCHX_worktree-chunk-tracking project
-- Note: Schema complete, incremental update logic pending BRANCHX-IMPL project
```

**Migration 0021**:
```sql
-- Migration: 0021_complete_branchx_schema.sql
-- Ticket: SCHMAFIX-2001
-- Purpose: Complete BRANCHX worktree tracking schema with indexes and constraints
-- Part of: BRANCHX_worktree-chunk-tracking project
-- Note: Schema complete, incremental update logic pending BRANCHX-IMPL project
```

## Dependencies
- **SCHMAFIX-5001** (RECOMMENDED) - Manual validation should complete first to ensure accuracy

## Risk Assessment
- **Risk**: Documentation inaccurate or out of sync with actual schema
  - **Mitigation**: Reference actual migration SQL files and validate schema against running database
- **Risk**: Future contributors miss documentation and add migrations to wrong location
  - **Mitigation**: Clear warnings in MCP migrations README, prominent documentation in Rust CLAUDE.md

## Files/Packages Affected
- `packages/maproom-mcp/migrations/README.md`
- `crates/maproom/CLAUDE.md`
- `docs/architecture/DATABASE_ARCHITECTURE.md`
- `crates/maproom/migrations/0018_add_blob_sha.sql` (add header comment)
- `crates/maproom/migrations/0019_create_code_embeddings.sql` (add header comment)
- `crates/maproom/migrations/0020_add_worktree_tracking.sql` (add header comment)
- `crates/maproom/migrations/0021_complete_branchx_schema.sql` (add header comment)
