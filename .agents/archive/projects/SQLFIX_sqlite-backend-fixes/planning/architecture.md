# Architecture: SQLite Backend Fixes

## 1. Current Architecture

```
crates/maproom/src/
├── db/
│   ├── mod.rs           # VectorStore trait + shared types (FileRecord, ChunkRecord, SearchHit)
│   ├── factory.rs       # get_store() factory with feature-gated SQLite
│   ├── postgres/
│   │   └── mod.rs       # PostgresStore (working, uses connection pool)
│   ├── sqlite/
│   │   ├── mod.rs       # SqliteStore (BROKEN - compile errors)
│   │   └── schema.rs    # init_schema() (exists but not declared as module)
│   ├── queries.rs       # Legacy Postgres-specific queries
│   ├── pool.rs          # PostgreSQL connection pool (deadpool)
│   └── connection.rs    # Database URL parsing
├── build.rs             # Compiles sqlite-vec.c
└── Cargo.toml           # Feature flags: sqlite, postgres
```

## 2. Design Decisions

### 2.1 Feature Flag Strategy

**Decision**: Mutually exclusive features at compile time
```toml
[features]
default = ["postgres"]
postgres = []
sqlite = ["rusqlite", "r2d2", "r2d2_sqlite"]
```

**Rationale**:
- Prevents binary bloat from including both backends
- Simplifies testing (one backend per test run)
- Matches production use case (users pick one)
- Provides natural rollback mechanism

### 2.2 Module Export Fix

**Problem**: `schema.rs` exists but isn't declared as a module

**Root Cause**: Missing `pub mod schema;` declaration in `sqlite/mod.rs`

**Fix** (in SQLFIX-1001):
```rust
// Add at top of sqlite/mod.rs, before imports
pub mod schema;
```

### 2.3 DateTime Serialization

**Problem**: `DateTime<Utc>` doesn't implement `rusqlite::ToSql`

**Solution**: Enable chrono feature in rusqlite (verified working)
```toml
# Cargo.toml - change from:
rusqlite = { version = "0.29.0", features = ["bundled"], optional = true }

# To:
rusqlite = { version = "0.29.0", features = ["bundled", "chrono"], optional = true }
```

**Why this works**: rusqlite 0.29 supports chrono 0.4.x via the `chrono` feature flag, which implements `ToSql` and `FromSql` for `DateTime<Utc>`.

### 2.4 Connection Initialization

**Current**: Uses `r2d2_sqlite` with `with_init` callback

**Required PRAGMAs** (in `with_init` callback):
```rust
conn.execute_batch(r#"
    PRAGMA journal_mode = WAL;      -- Better concurrency
    PRAGMA synchronous = NORMAL;    -- Balance of safety/speed
    PRAGMA foreign_keys = ON;       -- Referential integrity
    PRAGMA busy_timeout = 5000;     -- 5 second wait on locks (NEW)
"#)?;
```

**Note**: `busy_timeout` prevents immediate SQLITE_BUSY errors during concurrent access.

### 2.5 File Permissions (Security)

**Requirement**: Database file should have 0600 permissions

**Implementation** (in `SqliteStore::connect()`, after pool creation):
```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let db_path = std::path::Path::new(path);
    if db_path.exists() {
        std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600))?;
    }
}
```

### 2.6 Vector Search Strategy

**Phase 1 (This Project)**: FTS-only search
- `search_chunks_fts()` works with SQLite FTS5
- Vector search methods return empty results with log warning

**Phase 2 (Future Project)**: Full vector search
- Implement `vec_distance_cosine()` queries
- Test sqlite-vec with various dimensions
- Add dimension validation

## 3. File Changes Required

### 3.1 `Cargo.toml`
```toml
# Add chrono feature to rusqlite
rusqlite = { version = "0.29.0", features = ["bundled", "chrono"], optional = true }
```

### 3.2 `src/db/sqlite/mod.rs`

**Add module declaration** (top of file):
```rust
pub mod schema;
```

**Add busy_timeout to connection init** (in `with_init` callback):
```rust
conn.execute_batch(r#"
    PRAGMA journal_mode = WAL;
    PRAGMA synchronous = NORMAL;
    PRAGMA foreign_keys = ON;
    PRAGMA busy_timeout = 5000;
"#)?;
```

**Fix find_chunk_by_symbol** (lines 523-577):
- Clone `relpath` before first use: `let relpath_owned = relpath.map(|s| s.to_string());`
- Use `relpath_owned.as_deref()` in subsequent branches
- Consolidate SQL query logic to avoid parameter mismatches

**Add file permissions** (after pool creation in `connect()`):
```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    if std::path::Path::new(path).exists() {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
}
```

### 3.3 `src/db/sqlite/schema.rs`

**Add missing ts_doc_text column** to chunks table:
```sql
CREATE TABLE IF NOT EXISTS chunks (
    ...
    preview TEXT NOT NULL,
    ts_doc_text TEXT,  -- ADD THIS
    recency_score REAL NOT NULL,
    ...
)
```

**Fix FTS5 external content table**:
```sql
-- Change from:
CREATE VIRTUAL TABLE IF NOT EXISTS fts_chunks USING fts5(
    content,
    docstring,
    symbol_name,
    content='chunks',
    content_rowid='id'
);

-- To (standalone FTS table, manually synced):
CREATE VIRTUAL TABLE IF NOT EXISTS fts_chunks USING fts5(
    preview,
    docstring,
    symbol_name
);
```

### 3.4 FTS5 Query Syntax Fix

**Problem**: Current code generates invalid `"term"*` syntax

**Fix** (in `search_chunks_fts`, lines 454-459):
```rust
// Change from:
let fts_query = query
    .split_whitespace()
    .map(|t| format!("\"{}\"*", t.replace("\"", "")))
    .collect::<Vec<_>>()
    .join(" ");

// To (valid FTS5 prefix syntax):
let fts_query = query
    .split_whitespace()
    .map(|t| {
        let clean = t.replace("\"", "");
        format!("{}*", clean)  // Prefix without quotes
    })
    .collect::<Vec<_>>()
    .join(" OR ");  // OR instead of AND for broader matching
```

### 3.5 `src/db/factory.rs`

Already fixed (uncommitted) - feature gates work correctly.

## 4. Data Flow

```
CLI/Extension
     │
     ▼
get_store() ─────────────────┐
     │                       │
     ▼                       ▼
PostgresStore           SqliteStore
(postgres feature)      (sqlite feature)
     │                       │
     ▼                       ▼
tokio-postgres          rusqlite + r2d2
     │                       │
     ▼                       ▼
PostgreSQL              SQLite file
(Docker)                (maproom.db)
```

## 5. API Compatibility

The `VectorStore` trait remains unchanged. Both implementations must satisfy:

```rust
pub trait VectorStore: Send + Sync {
    // Core CRUD - both backends implement
    async fn get_or_create_repo(...) -> Result<i64>;
    async fn get_or_create_worktree(...) -> Result<i64>;
    async fn upsert_file(...) -> Result<i64>;
    async fn insert_chunk(...) -> Result<i64>;

    // Search - both implement FTS
    async fn search_chunks_fts(...) -> Result<Vec<SearchHit>>;

    // Embeddings - SQLite stubs for MVP
    async fn upsert_embeddings(...) -> Result<()>;
    async fn batch_upsert_embeddings(...) -> Result<()>;
}
```

## 6. Error Handling Strategy

**Standard**: Use `anyhow::Context` consistently across both backends

```rust
// Correct pattern
conn.execute(...).context("Failed to insert chunk")?;

// Avoid (loses context)
conn.execute(...)?;
```

## 7. Testing Strategy Integration

Architecture supports testing via:
1. **Unit tests**: Each backend tested independently with `:memory:` SQLite
2. **Feature flags**: `cargo test --features sqlite`
3. **CI matrix**: Both features tested in GitHub Actions

```rust
// tests/sqlite_store.rs
#[cfg(feature = "sqlite")]
mod sqlite_tests {
    #[tokio::test]
    async fn test_crud_operations() {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        // ... test CRUD
    }
}
```
