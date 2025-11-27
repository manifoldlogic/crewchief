# Architecture: SQLite-Only Migration

## 1. Design Overview

This project **removes PostgreSQL** and makes **SQLite the only database backend**. This dramatically simplifies the codebase by eliminating:
- The `VectorStore` trait abstraction
- Backend factory pattern
- Feature flags for SQLite
- Dual implementation maintenance

### Before (Complex)

```
┌─────────────┐      ┌──────────────┐      ┌─────────────────┐
│   main.rs   │──────│  VectorStore │──────│  PostgresStore  │
│ (commands)  │      │    trait     │      │  or SqliteStore │
└─────────────┘      └──────────────┘      └─────────────────┘
        │                   │                      │
        │ get_store()       │ Arc<dyn>             │
        ▼                   ▼                      ▼
┌─────────────┐      ┌──────────────┐      ┌─────────────────┐
│  factory.rs │      │ BackendType  │      │  db/postgres/   │
│             │      │   checks     │      │  db/sqlite/     │
└─────────────┘      └──────────────┘      └─────────────────┘
```

### After (Simple)

```
┌─────────────┐      ┌──────────────┐      ┌─────────────────┐
│   main.rs   │──────│  SqliteStore │──────│   SQLite DB     │
│ (commands)  │      │   (direct)   │      │  (~/.maproom/)  │
└─────────────┘      └──────────────┘      └─────────────────┘
```

## 2. Key Decisions

### Decision 1: Remove VectorStore Trait

**Approach**: **Delete** the `VectorStore` trait definition entirely from `db/mod.rs`, use `SqliteStore` directly.

**What Gets Deleted:**
- `pub trait VectorStore: Send + Sync { ... }` - The entire trait definition (~150 lines)
- `BackendType` enum
- All `#[async_trait]` impl blocks for the trait
- Factory pattern and `get_store()` function

```rust
// Before: Trait object indirection
pub async fn scan_worktree(store: Arc<dyn VectorStore>, ...) -> Result<ScanStats>

// After: Concrete type
pub async fn scan_worktree(store: &SqliteStore, ...) -> Result<ScanStats>
```

**Benefits**:
- No runtime dispatch overhead
- Simpler error messages
- Easier debugging
- No `Send + Sync` complexity

### Decision 2: Remove Feature Flags

**Approach**: SQLite is always enabled, no conditional compilation.

```toml
# Before (Cargo.toml)
[features]
sqlite = ["rusqlite", "sqlite-vec"]

# After
# No sqlite feature - always included
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
```

**Benefits**:
- Simpler build process
- One binary, one code path
- No feature matrix testing

### Decision 3: Direct Connection Function

**Approach**: Simple `connect()` function returns `SqliteStore`.

```rust
// db/mod.rs
pub async fn connect() -> anyhow::Result<SqliteStore> {
    let url = std::env::var("MAPROOM_DATABASE_URL")
        .unwrap_or_else(|_| {
            let home = dirs::home_dir().expect("No home directory");
            format!("sqlite://{}", home.join(".maproom/maproom.db").display())
        });
    SqliteStore::new(&url).await
}
```

**Benefits**:
- No factory pattern complexity
- Clear default path
- Environment variable override still works

## 3. Files to Delete

### Complete Removal

| File/Directory | Reason |
|---------------|--------|
| `db/postgres/` | PostgreSQL implementation |
| `db/pool.rs` | PostgreSQL connection pooling |
| `db/queries.rs` | PostgreSQL-specific queries (28 functions) |
| `db/factory.rs` | Backend switching logic |
| `db/materialized_views.rs` | PostgreSQL materialized views |
| `db/connection.rs` | PostgreSQL connection (if separate) |

### Partial Removal (from db/mod.rs)

- `VectorStore` trait definition
- `BackendType` enum
- PostgreSQL-related imports
- Feature flag conditionals

## 4. Module Refactoring

### 4.1 indexer/mod.rs

**Current**: Uses `&Client` (tokio_postgres)

**Change to**: `&SqliteStore`

```rust
// Before
use tokio_postgres::Client;
pub async fn scan_worktree(client: &Client, ...) -> Result<ScanStats> {
    let repo_id = db::get_or_create_repo(client, repo, root).await?;
    db::upsert_file(client, ...).await?;
}

// After
use crate::db::SqliteStore;
pub async fn scan_worktree(store: &SqliteStore, ...) -> Result<ScanStats> {
    let repo_id = store.get_or_create_repo(repo, root).await?;
    store.upsert_file(&file_record).await?;
}
```

**Functions to update** (9 PostgreSQL references):
- `scan_worktree`
- `scan_worktree_parallel` (merge into scan_worktree)
- `upsert_files`
- `watch_worktree`
- Helper functions

### 4.2 embedding/pipeline.rs

**Current**: Uses `&Client` (tokio_postgres) with raw SQL

**Change to**: `&SqliteStore` with method calls

```rust
// Before (15 PostgreSQL references)
use tokio_postgres::Client;
impl EmbeddingPipeline {
    pub async fn run(&self, client: &Client) -> Result<PipelineStats> {
        let count_row = client.query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NULL", &[]
        ).await?;
    }
}

// After
use crate::db::SqliteStore;
impl EmbeddingPipeline {
    pub async fn run(&self, store: &SqliteStore) -> Result<PipelineStats> {
        let count = store.get_chunks_needing_embeddings_count().await?;
    }
}
```

### 4.3 search/ modules

**Files with PostgreSQL references**:
- `search/pipeline.rs` (2)
- `search/fts.rs` (2)
- `search/vector.rs` (6)
- `search/graph.rs` (3)
- `search/signals.rs` (4)
- `search/executors.rs` (2)

**Change**: Replace `&Client` with `&SqliteStore` throughout.

### 4.4 context/ modules

**Files with PostgreSQL references**:
- `context/relationships.rs` (8)
- `context/graph.rs` (6)
- `context/assembler.rs` (7)
- `context/cache.rs` (3)
- Various detectors and strategies

**Change**: Replace `&Client` with `&SqliteStore` throughout.

### 4.5 main.rs

**Current**: Backend switching, SQLite blockers

**Changes**:
1. Remove `BackendType` enum
2. Remove `get_store_with_type()` function
3. Remove all `if backend_type == BackendType::SQLite` blocks
4. Replace `db::connect()` (returns Client) with new `db::connect()` (returns SqliteStore)
5. Remove `--parallel` flag or make it a no-op

```rust
// Before
Commands::Scan { parallel, ... } => {
    let (_, backend_type) = get_store_with_type().await?;
    if backend_type == BackendType::SQLite {
        anyhow::bail!("...");
    }
    if parallel {
        indexer::scan_worktree_parallel(&pool, ...).await?;
    } else {
        indexer::scan_worktree(&client, ...).await?;
    }
}

// After
Commands::Scan { ... } => {
    let store = db::connect().await?;
    indexer::scan_worktree(&store, ...).await?;
}
```

## 5. Cargo.toml Changes

### Remove Dependencies

```toml
# Remove these
tokio-postgres = "..."
pgvector = "..."
deadpool-postgres = "..."
```

### Keep/Add Dependencies

```toml
# Already present, remove feature flag
rusqlite = { version = "0.31", features = ["bundled", "backup"] }

# sqlite-vec is statically linked in db/sqlite/mod.rs
```

### Remove Features Section

```toml
# Delete this section
[features]
default = []
sqlite = ["rusqlite", "sqlite-vec"]
```

## 6. Testing Strategy

### Update Test Helpers

```rust
// Before
#[tokio::test]
#[cfg(feature = "sqlite")]
async fn test_something_sqlite() {
    let store = create_test_sqlite_store().await;
}

#[tokio::test]
async fn test_something_postgres() {
    let client = create_test_postgres_client().await;
}

// After
#[tokio::test]
async fn test_something() {
    let store = create_test_store().await;  // Always SQLite
}
```

### Delete PostgreSQL Test Files

Any tests specifically for PostgreSQL should be deleted.

## 7. Migration Path

### Phase 1: Core Database Simplification
1. Delete PostgreSQL files
2. Simplify `db/mod.rs`
3. Update `db/cleanup.rs` and `db/index_state.rs`

### Phase 2: Indexer Migration
1. Update `indexer/mod.rs`
2. Merge parallel scan into single function
3. Update helpers

### Phase 3: Embedding Pipeline
1. Update `embedding/pipeline.rs`
2. Update `main.rs` embedding commands

### Phase 4: Search & Context
1. Update search modules
2. Update context modules

### Phase 5: Main.rs & Testing
1. Clean up main.rs
2. Update Cargo.toml
3. Fix tests

## 8. File Change Summary

| File | Action |
|------|--------|
| `db/postgres/` | DELETE |
| `db/pool.rs` | DELETE |
| `db/queries.rs` | DELETE |
| `db/factory.rs` | DELETE |
| `db/materialized_views.rs` | DELETE |
| `db/mod.rs` | SIMPLIFY (remove trait, add connect()) |
| `db/cleanup.rs` | REFACTOR (use SqliteStore) |
| `db/index_state.rs` | REFACTOR (use SqliteStore) |
| `indexer/mod.rs` | REFACTOR (use SqliteStore) |
| `indexer/parallel.rs` | DELETE or MERGE |
| `embedding/pipeline.rs` | REFACTOR (use SqliteStore) |
| `search/*.rs` | REFACTOR (use SqliteStore) |
| `context/*.rs` | REFACTOR (use SqliteStore) |
| `main.rs` | REFACTOR (remove backend switching) |
| `Cargo.toml` | SIMPLIFY (remove pg deps, feature flags) |
