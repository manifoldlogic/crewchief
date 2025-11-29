# CLAUDE.md - Maproom Database Migrations

**CRITICAL**: When working with database migrations, you MUST complete BOTH steps or the migration will not run.

## Migration Process (Two Required Steps)

### Step 1: Create SQL Migration File

Create the migration file in this directory:
```
crates/maproom/migrations/NNNN_description.sql
```

**Naming Convention**:
- `NNNN` = Zero-padded sequential number (0001, 0002, ..., 0018, 0019, 0020)
- `description` = Brief snake_case description (add_blob_sha, create_code_embeddings)
- Example: `0021_add_chunk_metadata.sql`

**SQL File Requirements**:
- Use `IF NOT EXISTS` for all CREATE statements
- Use `IF EXISTS` for all DROP statements
- Include header comment with ticket ID and purpose
- Test idempotency (can run multiple times safely)
- Avoid explicit COMMIT statements (breaks transaction safety)
- Avoid `CREATE INDEX CONCURRENTLY` unless you set concurrent flag to true

### Step 2: Add to Rust Migration Runner (MANDATORY)

**Location**: `crates/maproom/src/db/queries.rs` (around lines 28-140)

**Find the migrations vector**:
```rust
let all_migrations: Vec<(i32, &str, &str, bool)> = vec![
    (1, "0001_init.sql", include_str!("./../../migrations/0001_init.sql"), false),
    // ... existing migrations ...
    (N, "NNNN_last_migration.sql", include_str!("./../../migrations/NNNN_last_migration.sql"), false),
];
```

**Add your migration as the NEXT sequential entry**:
```rust
(N+1, "NNNN_your_migration.sql", include_str!("./../../migrations/NNNN_your_migration.sql"), false),
```

**Migration Tuple Format**:
- **Field 1 (i32)**: Version number (sequential, no gaps)
- **Field 2 (&str)**: Filename (must match SQL file exactly)
- **Field 3 (&str)**: SQL content via `include_str!("./../../migrations/NNNN_name.sql")`
- **Field 4 (bool)**: Concurrent flag (see below)

**Concurrent Flag**:
- `false` = Run in transaction (DEFAULT - use this unless you know you need true)
  - Provides rollback on failure
  - Required for most DDL operations
  - Cannot use with `CREATE INDEX CONCURRENTLY`
- `true` = Run outside transaction
  - Required for `CREATE INDEX CONCURRENTLY`
  - Required for operations that cannot run in transaction block
  - No rollback on failure

### Step 3: Verify Integration

```bash
# Verify compilation succeeds (validates include_str! paths)
cd crates/maproom
cargo build

# Ensure no linting warnings
cargo clippy

# Verify migration is in the array
grep "NNNN_your_migration.sql" src/db/queries.rs
```

## Common Pitfalls

### 1. Creating SQL File But Not Adding to queries.rs ❌
**Problem**: Migration exists in filesystem but never runs
**Example**: Migration 0017 (`fix_index_size_limits.sql`) existed but wasn't in migrations array
**Fix**: ALWAYS add to queries.rs immediately after creating SQL file

### 2. Wrong Path in include_str! ❌
**Incorrect**: `include_str!("../../migrations/NNNN_name.sql")`
**Correct**: `include_str!("./../../migrations/NNNN_name.sql")`

### 3. Skipping Version Numbers ❌
**Problem**: Creates gaps in migration sequence
**Fix**: Always use next sequential number (no gaps allowed)

### 4. Using COMMIT in Migration SQL ❌
**Problem**: Cannot run inside transaction (concurrent=false fails)
**Fix**: Remove explicit COMMIT statements, rely on transaction boundaries

### 5. Using CREATE INDEX CONCURRENTLY with concurrent=false ❌
**Problem**: CONCURRENTLY requires no active transaction
**Fix**: Either remove CONCURRENTLY or set concurrent=true

## Migration Checklist

Before marking a migration ticket complete:

- [ ] SQL file created in `crates/maproom/migrations/NNNN_description.sql`
- [ ] Migration added to `crates/maproom/src/db/queries.rs` migrations array
- [ ] Version number is sequential (no gaps)
- [ ] Filename in tuple matches SQL file exactly
- [ ] include_str! path uses `./../../migrations/` format
- [ ] Concurrent flag set correctly (false for transaction-safe, true only if needed)
- [ ] `cargo build` succeeds (validates include_str! path)
- [ ] `cargo clippy` passes with no warnings
- [ ] SQL uses IF NOT EXISTS / IF EXISTS for idempotency
- [ ] Header comment added to SQL file with ticket ID

## Why This Matters

**The Rust binary is the single source of truth for migrations.**

- MCP TypeScript server uses migrations managed by Rust
- SQL files in this directory are embedded at compile time
- If a migration isn't in the queries.rs array, it NEVER runs
- This causes schema drift between code expectations and actual database state
- Result: "table does not exist" errors, crashes, data corruption

**Always complete both steps or the migration will be orphaned.**

## Example: Adding Migration 0021

### 1. Create SQL File

File: `crates/maproom/migrations/0021_add_chunk_metadata.sql`
```sql
-- TICKET-1234: Add metadata column to chunks table
-- Purpose: Store additional chunk metadata for analysis
-- Warning: Backfill may take time on large databases

CREATE TABLE IF NOT EXISTS maproom.chunk_metadata (
  chunk_id UUID PRIMARY KEY REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_chunk_metadata_gin
ON maproom.chunk_metadata USING GIN(metadata);
```

### 2. Add to queries.rs

File: `crates/maproom/src/db/queries.rs`
```rust
let all_migrations: Vec<(i32, &str, &str, bool)> = vec![
    // ... existing migrations 1-20 ...
    (20, "0020_add_worktree_tracking.sql", include_str!("./../../migrations/0020_add_worktree_tracking.sql"), false),
    // NEW MIGRATION:
    (21, "0021_add_chunk_metadata.sql", include_str!("./../../migrations/0021_add_chunk_metadata.sql"), false),
];
```

### 3. Verify

```bash
cd crates/maproom
cargo build  # Should succeed
cargo clippy # Should pass
```

## Getting Help

See:
- `crates/maproom/CLAUDE.md` - Rust indexer development guide
- `docs/architecture/DATABASE_ARCHITECTURE.md` - Schema documentation
- `.crewchief/projects/SCHMAFIX_*/planning/architecture.md` - Migration integration examples
