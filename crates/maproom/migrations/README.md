# Maproom Database Migrations

This directory contains SQL migration files for the Maproom database schema.

## Migration Files

Migrations are numbered sequentially and executed in order:

1. **0001_init.sql** - Initial schema with repos, worktrees, commits, files, chunks, edges
   - Creates pgvector, pg_trgm, and unaccent extensions
   - Defines symbol_kind and edge_type enums
   - Creates base tables and indices
   - Establishes vector columns (code_embedding, text_embedding) with dimension 1536
   - Creates ivfflat indices with lists=200

2. **0002_markdown_support.sql** - Markdown document support
   - Adds heading_1 through heading_6 to symbol_kind enum
   - Enables semantic search on markdown documentation

3. **0003_yaml_toml_support.sql** - YAML/TOML configuration support
   - Adds yaml_key, toml_key, json_key to symbol_kind enum
   - Enables search across configuration files

4. **0004_optimize_vector_indices.sql** - Vector search optimization (HYBRID_SEARCH-1002)
   - Creates partial indices for recency_score and churn_score
   - Creates composite index for repo_id + worktree_id filtering
   - Configures ivfflat.probes=10 for optimal accuracy/speed balance
   - Updates statistics with ANALYZE
   - Documents performance baselines and verification queries

5. **0014_add_enhanced_symbol_kinds.sql** - Enhanced symbol kinds for markdown and multi-language support (MD_ENHANCE-5001)
   - Adds markdown structural elements: list, table, link, image, image_link
   - Adds Rust language symbols: use, import, imports, trait, impl, struct, enum, macro, async_method, async_func, static, constant, variable, method
   - Adds Go language symbols: package, require, go_version
   - Enables comprehensive indexing of markdown documents and Rust/Go codebases
   - All additions use IF NOT EXISTS for idempotent migrations

## Running Migrations

Migrations are automatically executed when the Maproom service starts via `db::migrate()`.

### Manual Migration

To run migrations manually using psql:

```bash
# Connect to database
psql $MAPROOM_DATABASE_URL

# Run specific migration
\i crates/maproom/migrations/0001_init.sql
\i crates/maproom/migrations/0002_markdown_support.sql
\i crates/maproom/migrations/0003_yaml_toml_support.sql
\i crates/maproom/migrations/0004_optimize_vector_indices.sql
```

### Verifying Migrations

After running migrations, verify the schema:

```sql
-- List all tables in maproom schema
\dt maproom.*

-- List all indices
\di maproom.*

-- Check pgvector extension
\dx vector

-- Verify vector columns
\d maproom.chunks

-- Check current ivfflat.probes setting
SHOW ivfflat.probes;
```

## Migration Guidelines

When creating new migrations:

1. **Naming**: Use format `NNNN_description.sql` (e.g., `0005_add_user_preferences.sql`)

2. **Safety**:
   - Use `IF NOT EXISTS` for CREATE statements
   - Use `CREATE INDEX CONCURRENTLY` for production (outside transactions)
   - Test both forward and rollback paths
   - Avoid blocking operations on large tables

3. **Enums**:
   - Only ADD values to enums (never remove)
   - Use DO block for safe enum value addition:
     ```sql
     DO $$ BEGIN
       ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'new_value';
     EXCEPTION WHEN duplicate_object THEN NULL; END $$;
     ```

4. **Documentation**:
   - Include comments explaining purpose
   - Document performance impact
   - Provide verification queries
   - Reference related tickets/issues

5. **Update Migration List**:
   - Add migration to `src/db.rs` migrate() function
   - Update this README with description

## Performance Considerations

### Index Creation

Creating indices on large tables can be slow and blocking. For production:

```sql
-- Instead of:
CREATE INDEX idx_name ON table(column);

-- Use:
CREATE INDEX CONCURRENTLY idx_name ON table(column);
```

**Note**: `CREATE INDEX CONCURRENTLY` cannot run inside a transaction block.

### Statistics Updates

Run `ANALYZE` after:
- Creating new indices
- Bulk data imports
- Schema changes
- Major data modifications

```sql
ANALYZE maproom.chunks;
ANALYZE maproom.files;
```

### ivfflat Index Sizing

The `lists` parameter should be approximately `sqrt(row_count)`:

| Table Size | Recommended lists |
|------------|-------------------|
| 40k rows | 200 (current) |
| 100k rows | 316 |
| 500k rows | 707 |
| 1M rows | 1000 |

To update lists parameter, the index must be recreated:

```sql
DROP INDEX CONCURRENTLY maproom.idx_chunks_code_vec;
CREATE INDEX CONCURRENTLY idx_chunks_code_vec
  ON maproom.chunks USING ivfflat (code_embedding vector_cosine_ops)
  WITH (lists = 707);
```

## Troubleshooting

### Migration Fails

If a migration fails partway through:

1. **Check error message**: Identify the specific SQL statement that failed
2. **Verify prerequisites**: Ensure extensions are installed
3. **Check permissions**: Verify database user has required privileges
4. **Inspect schema state**: Determine what was already created

### Rolling Back

Migrations do not include automatic rollback. To roll back:

1. Manually drop created objects (tables, indices, etc.)
2. Restore from backup if available
3. Create a new migration with DROP statements

### Index Creation Timeout

If index creation times out:

1. Increase `maintenance_work_mem`: `SET maintenance_work_mem = '1GB';`
2. Use `CREATE INDEX CONCURRENTLY` (slower but non-blocking)
3. Reduce `lists` parameter temporarily
4. Run during low-traffic periods

## References

- [PostgreSQL Documentation](https://www.postgresql.org/docs/current/)
- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [Vector Search Configuration Guide](/workspace/crates/maproom/docs/VECTOR_SEARCH_CONFIGURATION.md)
- [HYBRID_SEARCH Architecture](/workspace/.agents/archive/projects/HYBRID_SEARCH_hybrid-retrieval-system/planning/HYBRID_SEARCH_ARCHITECTURE.md)
