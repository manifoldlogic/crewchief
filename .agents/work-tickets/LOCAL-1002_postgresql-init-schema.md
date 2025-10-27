# Ticket: LOCAL-1002: Write PostgreSQL init.sql schema

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Create a complete PostgreSQL initialization schema file (init.sql) with pgvector extension that will be automatically loaded when the PostgreSQL container starts for the first time. This schema supports hybrid search combining vector similarity and full-text search using nomic-embed-text embeddings (768 dimensions).

## Background
The LOCAL project aims to provide a fully containerized Maproom MCP service with local LLM embeddings. The PostgreSQL database is a core component that stores code chunks, embeddings, and relationships. This schema must be automatically loaded on first container startup via docker-entrypoint-initdb.d mechanism.

The schema must support:
- **Vector search** using pgvector with 768-dimension embeddings (nomic-embed-text model)
- **Full-text search** using PostgreSQL's native tsvector capabilities
- **Graph relationships** between code chunks (imports, calls, dependencies)
- **Monitoring and statistics** for observability

This ticket is part of Phase 1 (Core Infrastructure) and runs in parallel with LOCAL-1001 (Dockerfile creation).

## Acceptance Criteria
- [ ] init.sql file created in `config/` directory at project root
- [ ] Schema creates successfully in PostgreSQL 16 with pgvector extension
- [ ] All tables (repositories, worktrees, files, chunks, chunk_edges, stats) are defined
- [ ] Vector columns use correct dimension (768) for nomic-embed-text embeddings
- [ ] ivfflat indexes configured for vector similarity search on code_embedding and text_embedding columns
- [ ] GIN index configured for full-text search on fts_tokens tsvector column
- [ ] Foreign key constraints enable proper cascade deletes (deleting a repo deletes all worktrees, files, chunks, edges)
- [ ] Standard indexes on file_id, kind, from_chunk_id, to_chunk_id for graph traversal performance
- [ ] Schema can be loaded successfully via docker-entrypoint-initdb.d mechanism

## Technical Requirements
- **Database**: PostgreSQL 16 with pgvector extension
- **Extension**: `CREATE EXTENSION IF NOT EXISTS vector;`
- **Schema namespace**: `maproom` schema for all tables
- **Vector dimensions**: 768 (not 1536) - sized for nomic-embed-text model
- **Index types**:
  - ivfflat for vector similarity (with lists parameter)
  - GIN for full-text search (tsvector)
  - B-tree for standard lookups (foreign keys, enum columns)
- **Cascade deletes**: All foreign keys must have `ON DELETE CASCADE`
- **Timestamps**: Use `TIMESTAMPTZ` for all timestamp columns
- **Unique constraints**: Enforce uniqueness where appropriate (repo names, worktree names per repo, file paths per worktree, edges per type)

## Implementation Notes

### Tables Overview

1. **repositories**: Top-level container for a git repository
   - `id`, `name` (unique), `created_at`

2. **worktrees**: Git worktree within a repository
   - `id`, `repo_id` (FK to repositories), `name`, `path`, `created_at`
   - Unique constraint on `(repo_id, name)`

3. **files**: Individual files in a worktree
   - `id`, `worktree_id` (FK to worktrees), `relpath`, `file_type`, `size_bytes`, `last_modified`, `git_hash`
   - Unique constraint on `(worktree_id, relpath)`

4. **chunks**: Parsed code chunks with embeddings
   - `id` (BIGSERIAL), `file_id` (FK to files), `symbol_name`, `kind`, `start_line`, `end_line`
   - `signature`, `docstring`, `preview`
   - `code_embedding vector(768)`, `text_embedding vector(768)` - Vector columns
   - `fts_tokens tsvector` - Full-text search column
   - `created_at`, `updated_at`

5. **chunk_edges**: Relationships between chunks (imports, calls, etc.)
   - `id`, `from_chunk_id` (FK to chunks), `to_chunk_id` (FK to chunks), `edge_type`, `created_at`
   - Unique constraint on `(from_chunk_id, to_chunk_id, edge_type)`

6. **stats**: Monitoring metrics
   - `id`, `metric_name`, `metric_value`, `recorded_at`
   - Index on `(metric_name, recorded_at DESC)`

### Index Configuration

**Vector indexes** (ivfflat):
```sql
CREATE INDEX idx_chunks_code_embedding ON maproom.chunks
    USING ivfflat (code_embedding vector_cosine_ops)
    WITH (lists = 100);

CREATE INDEX idx_chunks_text_embedding ON maproom.chunks
    USING ivfflat (text_embedding vector_cosine_ops)
    WITH (lists = 100);
```

**Full-text index** (GIN):
```sql
CREATE INDEX idx_chunks_fts ON maproom.chunks USING GIN (fts_tokens);
```

**Standard indexes**:
- `idx_chunks_file_id` on `chunks(file_id)` for file-level queries
- `idx_chunks_kind` on `chunks(kind)` for filtering by symbol type
- `idx_edges_from` on `chunk_edges(from_chunk_id)` for outbound relationships
- `idx_edges_to` on `chunk_edges(to_chunk_id)` for inbound relationships
- `idx_edges_type` on `chunk_edges(edge_type)` for filtering by relationship type
- `idx_stats_name_time` on `stats(metric_name, recorded_at DESC)` for time-series queries

### Reference Implementation
The complete schema structure is documented in LOCAL_ARCHITECTURE.md lines 449-542. Use this as the authoritative reference for table structure and index configuration.

### Critical Considerations
- **768 dimensions**: Ensure vector columns use `vector(768)` not `vector(1536)`. This is for nomic-embed-text, not OpenAI.
- **ivfflat lists parameter**: Set to 100 for initial deployment. This can be tuned later based on dataset size.
- **Cascade deletes**: Critical for data consistency. When a repository is deleted, all child records (worktrees, files, chunks, edges) should be automatically removed.
- **Schema namespace**: Use `maproom` schema to avoid polluting the public schema.
- **Idempotency**: Use `IF NOT EXISTS` for extension and schema creation to allow re-running the script.

## Dependencies
- **None** - This ticket runs in parallel with LOCAL-1001 (Dockerfile creation)
- **Coordination**: Works with docker-engineer to ensure init.sql is properly mounted in docker-compose.yml (LOCAL-1003)

## Risk Assessment
- **Risk**: Vector dimension mismatch (using 1536 instead of 768)
  - **Impact**: High - Would break embedding storage and retrieval
  - **Mitigation**: Explicitly document 768 dimensions, add validation in integration tests (LOCAL-4004)

- **Risk**: Index creation fails on container startup
  - **Impact**: Medium - Would degrade search performance but not block functionality
  - **Mitigation**: Test schema creation manually before docker integration, verify indexes exist in integration tests

- **Risk**: Cascade delete removes too much data unexpectedly
  - **Impact**: Medium - Could cause data loss if not understood
  - **Mitigation**: Document cascade behavior, add integration tests that verify cascade behavior

- **Risk**: ivfflat index parameters not optimal for dataset size
  - **Impact**: Low - Search performance may be suboptimal initially
  - **Mitigation**: Start with conservative value (lists=100), document tuning in Phase 4 (LOCAL-4008)

## Files/Packages Affected
- `config/init.sql` (NEW) - PostgreSQL initialization schema
- Future: `docker-compose.yml` will mount this file into postgres container
- Future: Integration tests (LOCAL-4004) will validate schema creation
