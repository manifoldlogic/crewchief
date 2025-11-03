# Migration Runner Investigation Report
## MAPROOM_MIGRATIONS-1001

**Date:** 2025-11-03
**Investigator:** Claude (general-purpose agent)
**Status:** COMPLETE

---

## Executive Summary

The migration runner fails with "CREATE INDEX CONCURRENTLY cannot run inside a transaction block" errors due to **how `batch_execute()` sends multiple SQL statements in a single message to PostgreSQL**. Even though the simple query protocol doesn't create implicit transactions, PostgreSQL groups statements sent together and may execute them in a pseudo-transaction context.

**Root Causes Identified:**
1. **No Migration Tracking** - No `schema_migrations` table means migrations can't be idempotent
2. **batch_execute() Batching** - Sending multiple statements together causes PostgreSQL to group them
3. **Missing Schema in maproom-postgres** - The maproom-postgres database has NO schema/tables at all
4. **Migration Re-runs** - Without tracking, migrations try to create existing indexes

**Critical Finding:** The maproom-postgres database is EMPTY (0 tables). This is why manual SQL was needed.

---

## Investigation Findings

### 1. Tokio-Postgres Transaction Behavior

**Testing Results:**
- ✅ Single-line CONCURRENT CREATE INDEX works via psql
- ❌ Multi-statement SQL with batch_execute() fails with transaction error
- ✅ simple_query() uses "simple query protocol" (no implicit transactions)
- ❌ **BUT**: PostgreSQL groups statements sent in the same message

**Key Discovery:**
Both `batch_execute()` and `simple_query()` use the PostgreSQL "simple query protocol", which does NOT create implicit transactions at the protocol level. However:

```
When multiple statements are sent in a single message:
1. PostgreSQL receives them as a batch
2. PostgreSQL may execute them in a pseudo-transaction context
3. CREATE INDEX CONCURRENTLY requires being OUTSIDE any transaction context
4. Even IF NOT EXISTS doesn't help inside transactions
```

**Evidence:**
```bash
# This works (single statement):
$ psql -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_test ON chunks(id);"
CREATE INDEX

# This fails (part of batch_execute multi-statement):
Error: CREATE INDEX CONCURRENTLY cannot run inside a transaction block
```

---

### 2. CONCURRENT Index Migration Analysis

**Migrations with CONCURRENT indexes:**
- `0004_optimize_vector_indices.sql` - 1 CONCURRENT index
- `0008_context_query_optimizations.sql` - 5 CONCURRENT indexes
- `0010_add_blake3_hash.sql` - 1 CONCURRENT index
- `0012_optimize_indices.sql` - 17 CONCURRENT indexes
- `0015_add_ollama_columns.sql` - 3 CONCURRENT indexes

**Total:** 27 CONCURRENT index creations across 5 migrations

**Common Pattern:**
All use `CREATE INDEX CONCURRENTLY IF NOT EXISTS`, which is correct, but they're being executed via `simple_query()` which still groups statements when the migration file contains multiple statements.

**False Alarm:**
- BEGIN/COMMIT statements found in migrations are ONLY in:
  - PL/pgSQL function bodies ($$BEGIN...END$$) - Safe
  - Commented-out rollback sections - Safe
- NO actual SQL transaction blocks in active migration code

---

### 3. Migration Tracking Analysis

**Critical Finding:** NO migration tracking table exists.

**Checked:**
- ✅ No `schema_migrations` table
- ✅ No `migrations` table
- ✅ No `_migrations` table
- ✅ No version tracking of any kind

**Impact:**
1. **No Idempotency** - Can't safely re-run `db migrate`
2. **No Skip Logic** - Every migration attempts to run every time
3. **Index Conflicts** - CONCURRENT IF NOT EXISTS fails in transaction context even if index exists
4. **No Rollback Tracking** - Can't determine current schema version

**Current Migration Runner Logic:**
```rust
pub async fn migrate(client: &Client) -> anyhow::Result<()> {
    // Execute ALL migrations every time (no tracking)
    let migrations = vec![...];
    for sql in migrations {
        client.batch_execute(sql).await?;  // Run all statements
    }
    // Execute CONCURRENT migrations
    execute_with_concurrent_indexes(client, migration_0004).await?;
    execute_with_concurrent_indexes(client, migration_0008).await?;
    // ...
}
```

**Problem:** Every call to `db migrate` tries to run ALL migrations, causing conflicts.

---

### 4. Schema Drift Analysis

**Database States:**

| Database | Schema | Tables | Status |
|----------|--------|--------|--------|
| `maproom-postgres` (maproom.maproom) | ❌ Does not exist | 0 | EMPTY |
| `devcontainer-postgres` (crewchief.maproom) | ✅ Exists | 8 | Partial |
| `devcontainer-postgres` (crewchief.public) | ✅ Exists | 3 | Partial |

**Critical Discovery:** The `maproom-postgres` database has **NO SCHEMA OR TABLES AT ALL**.

This explains:
- Why manual SQL was needed for migration 0016
- Why the CLI setup command fails
- Why migrations can't be applied

**Devcontainer Database State:**
- Has `maproom` schema with 8 tables
- Has `updated_at` column (manually applied)
- Has some indexes but not all CONCURRENT ones

**Schema Drift:**
- Manual application of migration 0016 to devcontainer postgres ✅
- Manual application of migration 0016 to maproom-postgres ❌ (database empty)
- Unknown which other migrations were manually applied

---

### 5. Migration Runner Code Analysis

**File:** `/workspace/crates/maproom/src/db/queries.rs`

**Current Approach:**
```rust
// Migration 0004: Moved to execute_with_concurrent_indexes (recent fix)
execute_with_concurrent_indexes(
    client,
    include_str!("./../../migrations/0004_optimize_vector_indices.sql"),
).await?;
```

**execute_with_concurrent_indexes() implementation:**
```rust
async fn execute_with_concurrent_indexes(client: &Client, sql: &str) -> anyhow::Result<()> {
    client.simple_query(sql).await?;  // Sends entire migration as one message
    Ok(())
}
```

**The Problem:**
Even though `simple_query()` uses the simple query protocol (no implicit transactions), it sends the **entire migration file content** as a single message. Migration 0008, for example, contains:
- 5 CREATE INDEX CONCURRENTLY statements
- Multiple COMMENT statements
- ANALYZE statements
- Other DDL

PostgreSQL receives all these statements together and may execute them in a grouped context that prevents CONCURRENT operations.

---

## Root Cause Summary

### Primary Issue: Statement Batching
**What's happening:**
1. Migration runner calls `simple_query(entire_migration_file)`
2. PostgreSQL receives 100+ lines of SQL in one message
3. PostgreSQL groups execution even though protocol doesn't require transactions
4. CREATE INDEX CONCURRENTLY fails because it's not truly isolated

**Why manual SQL works:**
```bash
# This works:
$ psql -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_test ON chunks(id);"

# This is essentially what we're doing (fails):
$ psql << EOF
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_1 ON table(col1);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_2 ON table(col2);
COMMENT ON INDEX idx_1 IS 'description';
ANALYZE table;
EOF
```

### Secondary Issue: No Migration Tracking
Without a migrations table:
- Can't skip already-applied migrations
- Can't achieve idempotency
- Can't safely re-run migrations
- Can't track schema version

### Tertiary Issue: Empty maproom-postgres Database
The production-like database (`maproom-postgres`) has no schema at all, making it impossible to run the MCP service against it.

---

## Recommendations

### Option 1: Execute Each Statement Separately (Recommended)
**Approach:** Parse migration files and execute CREATE INDEX CONCURRENTLY statements individually

**Pros:**
- True isolation for CONCURRENT operations
- Works with current migration files
- No need to rewrite migrations

**Cons:**
- Requires SQL parsing logic
- More complex migration runner

**Implementation:**
```rust
async fn execute_with_concurrent_indexes(client: &Client, sql: &str) -> anyhow::Result<()> {
    // Parse SQL into individual statements
    let statements = parse_sql_statements(sql);

    for stmt in statements {
        if stmt.contains("CREATE INDEX CONCURRENTLY") {
            // Execute CONCURRENT indexes individually
            client.simple_query(&stmt).await?;
        } else {
            // Batch other statements together
            client.batch_execute(&stmt).await?;
        }
    }
    Ok(())
}
```

### Option 2: Use Separate Migration Files for CONCURRENT Indexes
**Approach:** Split migrations so each CONCURRENT index is in its own file

**Pros:**
- Simple, no parsing needed
- Clear separation of concerns

**Cons:**
- Requires rewriting migration structure
- More migration files to manage
- Breaks existing migration numbering

### Option 3: Add Migration Tracking + Idempotency (Must Have)
**Approach:** Implement `schema_migrations` table to track applied migrations

**Pros:**
- Enables idempotency
- Industry standard pattern
- Prevents re-running migrations

**Cons:**
- Additional complexity
- Needs careful design

**Implementation:**
```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
    version VARCHAR(255) PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

```rust
pub async fn migrate(client: &Client) -> anyhow::Result<()> {
    // Create tracking table
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version VARCHAR(255) PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ).await?;

    // Check which migrations are applied
    let applied: HashSet<String> = client
        .query("SELECT version FROM schema_migrations", &[])
        .await?
        .iter()
        .map(|row| row.get(0))
        .collect();

    // Run only unapplied migrations
    for (version, sql) in all_migrations {
        if !applied.contains(version) {
            execute_migration(client, sql).await?;
            client.execute(
                "INSERT INTO schema_migrations (version) VALUES ($1)",
                &[&version]
            ).await?;
        }
    }
}
```

### Option 4: Initialize maproom-postgres Database
**Approach:** Run migrations on the empty maproom-postgres database

**Required:** Must be done regardless of other options

**Steps:**
1. Ensure maproom schema exists
2. Apply all migrations in order
3. Verify schema matches devcontainer database

---

## Recommended Implementation Plan

### Phase 1: Add Migration Tracking (Required)
- Create `schema_migrations` table
- Add version checking logic
- Enable idempotent migration runs

### Phase 2: Fix CONCURRENT Index Execution (Required)
- Implement statement-level execution for CONCURRENT indexes
- Parse SQL or separate CONCURRENT statements
- Test with all 5 affected migrations

### Phase 3: Initialize Databases (Required)
- Apply all migrations to maproom-postgres database
- Verify schema consistency across databases
- Document expected schema state

### Phase 4: Add Safety Features (Recommended)
- Add rollback capability
- Add migration validation
- Add schema checksum verification

---

## Technical Specifications for Implementation Ticket

### Files to Modify:
1. `/workspace/crates/maproom/src/db/queries.rs`
   - Add `schema_migrations` table creation
   - Add migration tracking logic
   - Modify `execute_with_concurrent_indexes()` to handle individual statements

2. `/workspace/crates/maproom/src/db/mod.rs` (if needed)
   - Add SQL parsing utilities
   - Add migration version management

### Required Functions:
```rust
// Check if migration already applied
async fn is_migration_applied(client: &Client, version: &str) -> Result<bool>;

// Record migration as applied
async fn record_migration(client: &Client, version: &str) -> Result<()>;

// Parse SQL into individual statements
fn parse_sql_statements(sql: &str) -> Vec<String>;

// Execute single statement (handles CONCURRENT specially)
async fn execute_statement(client: &Client, sql: &str) -> Result<()>;
```

### Migration Versions:
```
0001_init
0002_markdown_support
0003_yaml_toml_support
0004_optimize_vector_indices
0005_create_materialized_views
0006_optimize_gin_index
0007_ab_testing_schema
0008_context_query_optimizations
0009_create_context_cache
0010_add_blake3_hash
0011_python_symbol_kinds
0012_optimize_indices
0013_query_tuning
0014_add_enhanced_symbol_kinds
0015_add_ollama_columns
0016_add_updated_at_to_chunks
```

### Testing Requirements:
1. Fresh database migration (all 16)
2. Partial database migration (resume from middle)
3. Idempotent re-run (no errors)
4. CONCURRENT index creation (verify all 27 indexes)
5. Schema consistency check (both databases match)

---

## Conclusion

The migration runner issues stem from three interconnected problems:

1. **Statement Batching:** `simple_query()` sends entire migration files as batches, causing PostgreSQL to group execution and block CONCURRENT operations

2. **No Tracking:** Without `schema_migrations`, migrations can't be idempotent and always try to re-run

3. **Empty Database:** The maproom-postgres database has no schema, requiring initial migration application

**Solution:** Implement migration tracking + execute CONCURRENT statements individually + initialize both databases.

**Priority:** HIGH - This blocks clean database setup and automated deployment.

**Estimated Effort:** 3-4 hours for full implementation and testing.

---

## Next Steps

1. ✅ Create this investigation report
2. ⏳ Review findings with team/user
3. ⏳ Create implementation ticket (MAPROOM_MIGRATIONS-2001)
4. ⏳ Implement recommended solution
5. ⏳ Test with both databases
6. ⏳ Document migration workflow
