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

## Why Rust Owns Migrations

- Rust binary is standalone (works without Node.js)
- Compile-time validation of SQL syntax via `include_str!` macro
- Single binary deployment
- Existing migration framework with transaction support

## Historical Context

These migrations were originally created as part of the BLOBSHA and BRANCHX projects to enable:
- **Content-addressed storage**: Deduplicate embeddings using blob SHA hashes
- **Worktree tracking**: Track which worktrees contain each code chunk

The SCHMAFIX project (November 2025) integrated these migrations into the Rust migration runner to fix a critical bug where MCP TypeScript code referenced tables that didn't exist in the database schema.

## Related Documentation

- Rust migrations: `crates/maproom/migrations/`
- Migration runner: `crates/maproom/src/db/queries.rs`
- Schema documentation: `docs/architecture/DATABASE_ARCHITECTURE.md`
- Migration guide: `crates/maproom/migrations/CLAUDE.md`
