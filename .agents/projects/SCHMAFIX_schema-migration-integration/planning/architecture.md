# Architecture: Schema Migration Integration

## MVP-Focused Solution Design

This is NOT a greenfield architecture - it's integrating existing migration SQL files into the existing Rust migration runtime. The goal is **pragmatic integration**, not enterprise perfection.

## Core Architecture Decision

**Single Source of Truth**: Rust migration runner in `crates/maproom/src/db/queries.rs`

All database migrations must be:
1. Embedded in the Rust binary via `include_str!` macro
2. Applied automatically on startup by the migration runner
3. Tracked in the `schema_migrations` table

### Why Rust Owns Migrations

1. **Runtime Independence** - Rust binary works standalone without Node.js
2. **Compile-Time Validation** - Migration SQL is checked at compile time
3. **Deployment Simplicity** - Single binary contains all schema logic
4. **Already Implemented** - Migration framework exists, just add files

## Migration Integration Strategy

### Approach: Copy MCP Migrations to Rust

**Source**: `packages/maproom-mcp/migrations/*.sql`
**Destination**: `crates/maproom/migrations/*.sql`
**Numbering**: Continue from 0017 (latest Rust migration)

**New Migrations**:
```
0018_add_blob_sha.sql            (from MCP 001)
0019_create_code_embeddings.sql  (from MCP 002)
0020_add_worktree_tracking.sql   (from MCP 004)
0021_complete_branchx_schema.sql (from MCP 005)
```

### Why Not Share Migration Files?

Considered alternatives:
1. ❌ **Symlinks** - Breaks cross-platform compatibility
2. ❌ **Build-time copy** - Adds complexity to build process
3. ❌ **Shared directory** - Confusing ownership
4. ✅ **Duplicate and adapt** - Clear, simple, works

**Trade-off**: Duplication vs. clarity. We choose clarity.

## Migration Content Strategy

### Adapt MCP Migrations for Rust

MCP migrations have MCP-specific comments and context. We'll:
1. **Remove MCP references** - Clean up comments
2. **Add Rust context** - Update headers with SCHMAFIX ticket references
3. **Preserve SQL logic** - Keep the actual DDL statements identical
4. **Test independently** - Ensure migrations work in Rust context

### Example Adaptation

**MCP Migration** (`packages/maproom-mcp/migrations/001_add_blob_sha.sql`):
```sql
-- Migration 001: Add blob_sha column to chunks table
-- Purpose: Enable content-addressed chunk storage for embedding deduplication
-- Related: BLOBSHA-1002
```

**Rust Migration** (`crates/maproom/migrations/0018_add_blob_sha.sql`):
```sql
-- Migration 0018: Add blob_sha column to chunks table
-- Purpose: Enable content-addressed chunk storage for embedding deduplication
-- Related: SCHMAFIX-1001 (integrating BLOBSHA-1002)
-- Original: packages/maproom-mcp/migrations/001_add_blob_sha.sql
```

## Integration Points

### 1. Rust Migration Runner

**File**: `crates/maproom/src/db/queries.rs`
**Function**: `run_migrations()`

**Changes Required**:
```rust
// Add new migrations to the array
let migrations: Vec<(i32, &str, &str, bool)> = vec![
    // ... existing 0000-0016 ...
    (
        17,
        "0017_fix_index_size_limits.sql",
        include_str!("./../../migrations/0017_fix_index_size_limits.sql"),
        true,
    ),
    // NEW MIGRATIONS
    (
        18,
        "0018_add_blob_sha.sql",
        include_str!("./../../migrations/0018_add_blob_sha.sql"),
        false,  // Not concurrent
    ),
    (
        19,
        "0019_create_code_embeddings.sql",
        include_str!("./../../migrations/0019_create_code_embeddings.sql"),
        false,
    ),
    (
        20,
        "0020_add_worktree_tracking.sql",
        include_str!("./../../migrations/0020_add_worktree_tracking.sql"),
        false,
    ),
    (
        21,
        "0021_complete_branchx_schema.sql",
        include_str!("./../../migrations/0021_complete_branchx_schema.sql"),
        false,
    ),
];
```

### 2. Migration Table Tracking

**Existing Table**: `schema_migrations`
```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    executed_at TIMESTAMP DEFAULT NOW()
);
```

**How It Works**:
1. Rust checks `schema_migrations` table
2. If migration version doesn't exist, applies it
3. Records version and timestamp
4. Skips already-applied migrations

**Idempotent**: Safe to run multiple times

### 3. MCP Code Compatibility

**Current Problem** (`packages/maproom-mcp/src/index.ts:511`):
```typescript
const { rows: embeddingCheck } = await client.query(
  'SELECT COUNT(*) as count FROM maproom.code_embeddings LIMIT 1'
)
```

**After Migration 0019**: Table exists, query succeeds

**No MCP Code Changes Needed**: Migration fixes the underlying issue

## Technology Choices

### PostgreSQL DDL

**Migration Language**: Pure SQL (PostgreSQL dialect)
**Why**: Database-native, portable, well-understood

**Features Used**:
- `IF NOT EXISTS` - Idempotent table/column creation
- `ALTER TABLE ADD COLUMN` - Safe column additions
- `CREATE INDEX CONCURRENTLY` - Non-blocking indexes (where needed)
- `COMMENT ON` - Self-documenting schema

### Rust include_str! Macro

**Why**: Compile-time embedding of migration files
**Benefits**:
- Single binary deployment
- No external file dependencies
- Compile-time validation of file existence
- Zero runtime file I/O

**Trade-off**: Changes require recompile (acceptable for migrations)

## Performance Considerations

### Migration Execution Time

**Estimated Durations** (for typical database with 10k-100k chunks):

| Migration | Operation | Estimated Time | Blocking? |
|-----------|-----------|----------------|-----------|
| 0018 | Add `blob_sha` column | <1 second | No (ALTER TABLE ADD COLUMN is fast) |
| 0018 | Backfill blob_sha | 10-60 seconds | Yes (UPDATE all rows) |
| 0019 | Create `code_embeddings` | <1 second | No (empty table) |
| 0019 | Create HNSW index | <1 second | No (no data yet) |
| 0020 | Add `worktree_ids` | Already exists | Skip |
| 0020 | Create `worktree_index_state` | Already exists | Skip |
| 0021 | Final schema tweaks | <5 seconds | Depends on content |

**Total Estimated Time**: 15-70 seconds (one-time, on first run after upgrade)

### Concurrent Index Creation

Some migrations use `CREATE INDEX CONCURRENTLY` which:
- Allows reads/writes during index creation
- Requires special handling (can't run in transaction)
- Marked with `true` in migration array (4th parameter)

**Affected Migrations**:
- 0004_optimize_vector_indices.sql
- 0008_context_query_optimizations.sql
- 0010_add_blake3_hash.sql
- 0012_optimize_indices.sql
- 0015_add_ollama_columns.sql

**Our New Migrations**: All run in transactions (simpler, safe for additive changes)

## Long-Term Maintainability

### Migration Hygiene

**Rules**:
1. **Never modify existing migrations** - Only add new ones
2. **Always increment version** - Sequential numbering
3. **Test before merge** - Run against test database
4. **Document purpose** - Clear headers in migration SQL

### Future Deprecation

**MCP Migration Directory** (`packages/maproom-mcp/migrations/`):
- Keep for historical reference
- Add README explaining Rust owns migrations now
- Consider removing in future major version

**Why Keep Short-Term**:
- Historical context for why migrations exist
- Original ticket references (BLOBSHA-*, BRANCHX-*)
- Comparison for validation

## Testing Strategy

### Migration Testing

**1. Fresh Database Test**:
```bash
# Drop and recreate database
dropdb maproom_test
createdb maproom_test

# Run migrations
cargo run --bin crewchief-maproom -- db

# Verify schema
psql maproom_test -c "\d maproom.chunks"
```

**2. Incremental Migration Test**:
```bash
# Start with database at version 0017
# Run new binary with 0018-0021
# Verify only new migrations applied
```

**3. Idempotency Test**:
```bash
# Run migrations twice
# Verify no errors, no duplicates
```

### Integration Testing

**MCP E2E Test**:
```typescript
// packages/maproom-mcp/tests/integration.test.ts
it('vector search works after migrations', async () => {
  // Attempt vector search (used to fail)
  const result = await search({
    mode: 'vector',
    query: 'authentication'
  })

  // Should not throw "table code_embeddings does not exist"
  expect(result).toBeDefined()
})
```

## Edge Cases and Error Handling

### Case 1: Partial Migration Failure

**Scenario**: Migration 0018 succeeds, 0019 fails mid-execution

**Handling**:
- Each migration runs in a transaction (except CONCURRENT)
- Failed migration rolls back
- `schema_migrations` not updated for failed migration
- Next run retries from failed migration

**Recovery**: Fix migration SQL, restart process

### Case 2: Database Already Has Some Schema

**Scenario**: Manual SQL created `worktree_ids` column

**Handling**:
- Migration SQL uses `IF NOT EXISTS`
- PostgreSQL skips existing objects
- Migration completes successfully
- Idempotent execution

**Example**:
```sql
ALTER TABLE maproom.chunks
  ADD COLUMN IF NOT EXISTS worktree_ids JSONB;
  -- Skips if column already exists
```

### Case 3: Old Rust Binary vs New Schema

**Scenario**: User downgrades to old Rust binary after running new migrations

**Handling**:
- Old binary ignores new columns (PostgreSQL allows extra columns)
- Queries work (SELECT * still works)
- Features gracefully degrade (no blob_sha, no deduplication)

**Not a Breaking Change**: Additive schema is backward-compatible

## Success Metrics

**How We Know It Works**:

1. ✅ `schema_migrations` table shows versions 18-21
2. ✅ `\d maproom.chunks` shows `blob_sha` column
3. ✅ `\dt maproom.code_embeddings` shows table exists
4. ✅ MCP vector search doesn't crash
5. ✅ Integration tests pass
6. ✅ No manual SQL required

## Constraints Validated

✅ **No Breaking Changes** - Additive only (new columns/tables)
✅ **Backward Compatibility** - Old binaries tolerate new schema
✅ **Zero Downtime** - Migrations run quickly, no service interruption
✅ **Data Preservation** - No DROP or DELETE statements
✅ **Test Coverage** - Migration tests in place

## Explicitly Not Implementing

This architecture does NOT include:
- BLOBSHA feature logic (blob SHA computation during indexing)
- BRANCHX feature logic (incremental updates, tree SHA comparison)
- BRWATCH feature logic (file watching)
- Migration rollback scripts
- Database connection pooling improvements
- Query performance optimization beyond what migrations provide

**Scope**: Schema only. Features later.
