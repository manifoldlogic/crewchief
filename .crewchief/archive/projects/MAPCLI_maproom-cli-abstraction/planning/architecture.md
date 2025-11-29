# Architecture: MAPCLI - Maproom CLI Abstraction

## Overview

This document describes the architectural approach for updating the `crewchief-maproom` CLI and daemon to use the `VectorStore` trait abstraction, enabling SQLite backend support without modifying core indexer logic.

## Current Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        main.rs (CLI)                            │
├─────────────────────────────────────────────────────────────────┤
│  Commands:                                                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌────────────┐ │
│  │    scan     │ │   search    │ │   status    │ │   serve    │ │
│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬─────┘ │
│         │               │               │               │       │
│         ▼               ▼               ▼               ▼       │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              db::connect() → tokio_postgres::Client       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    PostgreSQL Only                        │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

**Problems**:
- Tight coupling to `tokio_postgres::Client`
- No abstraction layer for database operations
- Cannot use SQLite backend

## Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        main.rs (CLI)                            │
├─────────────────────────────────────────────────────────────────┤
│  Commands:                                                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌────────────┐ │
│  │    scan     │ │   search    │ │   status    │ │   serve    │ │
│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬─────┘ │
│         │               │               │               │       │
│         ▼               ▼               ▼               ▼       │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │           get_store() → Arc<dyn VectorStore>              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│              ┌───────────────┴───────────────┐                  │
│              ▼                               ▼                  │
│  ┌───────────────────────┐     ┌───────────────────────┐       │
│  │     PostgresStore     │     │      SqliteStore      │       │
│  │    (via PgPool)       │     │   (via rusqlite)      │       │
│  └───────────────────────┘     └───────────────────────┘       │
└─────────────────────────────────────────────────────────────────┘
```

## Design Decisions

### 1. Factory Pattern for Backend Selection

**Decision**: Use `get_store()` factory from `db/factory.rs`

**Rationale**:
- Already implemented in VECSTORE
- URL-based backend detection is intuitive
- Environment variable configuration is standard

**Implementation**:
```rust
// Before
let client = db::connect().await?;

// After
let store = db::factory::get_store().await?;
```

### 2. Arc<dyn VectorStore> as Primary Interface

**Decision**: Pass `Arc<dyn VectorStore>` to all command handlers

**Rationale**:
- Trait objects provide runtime polymorphism
- Arc enables sharing across async tasks
- Consistent with Rust patterns for database access

**Trade-offs**:
- Dynamic dispatch overhead (negligible for I/O-bound work)
- Cannot use backend-specific features without downcasting

### 3. DaemonState Redesign

**Decision**: Replace `PgPool` with `Arc<dyn VectorStore>`

**Current**:
```rust
struct DaemonState {
    pool: PgPool,
    embedding_service: EmbeddingService,
}
```

**Proposed**:
```rust
struct DaemonState {
    store: Arc<dyn VectorStore>,
    embedding_service: EmbeddingService,
}
```

**Rationale**:
- Daemon operations map directly to VectorStore methods
- Search, status, and other RPC calls use trait methods
- Embedding service remains independent of storage

### 4. Deferred Indexer Strategy (MVP Decision)

**Decision**: Defer scan/upsert/watch commands to Phase 2 - focus MVP on daemon/search/status/cleanup

**Problem**: Indexer functions have explicit PostgreSQL type parameters:
```rust
pub async fn scan_worktree(client: &Client, ...)        // tokio_postgres::Client
pub async fn upsert_files(client: &Client, ...)         // tokio_postgres::Client
pub async fn watch_worktree(_client: &Client, ...)      // tokio_postgres::Client
pub async fn scan_worktree_parallel(pool: &PgPool, ...) // deadpool_postgres::Pool
```

Creating SQLite equivalents (`scan_worktree_sqlite`) would require duplicating ~1000 lines of indexer logic.

**Options Evaluated**:
1. **Option A**: Create duplicate SQLite indexer functions - significant work, cleaner separation
2. **Option B**: Defer scan/upsert/watch to Phase 2 - smaller MVP, achievable now ✅
3. **Option C**: Refactor indexer to accept `&dyn VectorStore` - biggest change, cleanest result

**Chosen**: **Option B** - Defer indexer commands to Phase 2

**Rationale**:
- MVP delivers zero-config semantic search with pre-indexed data
- Users can populate SQLite via database import/migration
- Phase 2 can properly abstract the indexer when there's more time
- Avoids half-baked indexer duplication

**MVP Commands**:
| Command | SQLite Support | Implementation |
|---------|----------------|----------------|
| `search` | ✅ Phase 1 | Uses `store.search_chunks_fts()` |
| `vector-search` | ✅ Phase 1 | Uses `store.search_chunks_vector()` |
| `status` | ✅ Phase 1 | Uses refactored status module |
| `db cleanup-stale` | ✅ Phase 1 | Uses `store.detect_stale_worktrees()` |
| `serve` (daemon) | ✅ Phase 1 | Uses VectorStore trait |
| `scan` | ⏸️ Phase 2 | Requires indexer abstraction |
| `upsert` | ⏸️ Phase 2 | Requires indexer abstraction |
| `watch` | ⏸️ Phase 2 | Requires indexer abstraction |

**Phase 2 Work** (separate project):
- Abstract indexer to accept `&dyn VectorStore`
- Create `IndexerTrait` with backend-specific implementations
- Enable scan/upsert/watch for SQLite

### 5. Parallel Scan Handling

**Decision**: Disable parallel mode for SQLite

**Rationale**:
- SQLite has single-writer limitation
- `PgPool` concept doesn't apply to SQLite
- Sequential scan is correct, just slower

**Implementation**:
```rust
if parallel && store.backend_type() == BackendType::SQLite {
    eprintln!("Warning: --parallel flag ignored for SQLite backend");
    // Fall through to sequential scan
}
```

### 6. Migration Command Handling

**Decision**: Skip migrations for SQLite, run for PostgreSQL

**Rationale**:
- SQLite auto-migrates on `connect()`
- PostgreSQL requires explicit migration
- No behavior change for existing users

**Implementation**:
```rust
Commands::Db { command: DbCommand::Migrate } => {
    match get_store().await?.backend_type() {
        BackendType::PostgreSQL => {
            let client = db::connect().await?;
            db::migrate(&client).await?;
        }
        BackendType::SQLite => {
            println!("SQLite database auto-migrates on connection");
        }
    }
}
```

## Component Changes

### 0. BackendType Enum (PREREQUISITE - First Deliverable)

Before any other work, add `BackendType` to the VectorStore trait:

**File**: `db/mod.rs`
```rust
/// Backend type for runtime detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    PostgreSQL,
    SQLite,
}

#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Returns the backend type for runtime feature detection
    fn backend_type(&self) -> BackendType;

    // ... existing methods
}
```

**Implementation in stores**:
```rust
// PostgresStore
fn backend_type(&self) -> BackendType {
    BackendType::PostgreSQL
}

// SqliteStore
fn backend_type(&self) -> BackendType {
    BackendType::SQLite
}
```

This must be completed first as all other tickets depend on `backend_type()`.

### main.rs Modifications

1. **Add backend type detection helper**:
```rust
async fn get_store_with_type() -> anyhow::Result<(Arc<dyn VectorStore>, BackendType)> {
    let store = db::factory::get_store().await?;
    let backend_type = store.backend_type();
    Ok((store, backend_type))
}
```

2. **Update command handlers** (for MVP-scoped commands):
- Replace `db::connect()` with `get_store()` for search/status/cleanup
- Pass `store` to refactored functions
- Add backend-specific logic where needed
- Keep `db::connect()` for scan/upsert/watch (PostgreSQL-only in MVP)

3. **Graceful PostgreSQL-only handling**:
```rust
Commands::Scan { ... } => {
    let store = get_store().await?;
    if store.backend_type() == BackendType::SQLite {
        anyhow::bail!("scan command requires PostgreSQL backend (SQLite support coming in Phase 2)");
    }
    // Proceed with PostgreSQL scan
    let client = db::connect().await?;
    indexer::scan_worktree(&client, ...).await?;
}
```

### status.rs Refactoring (CRITICAL)

**Problem**: `status.rs` creates its own PostgreSQL connection, bypassing all abstraction:

```rust
// Current (lines 28-34)
let database_url = env::var("MAPROOM_DATABASE_URL")?;
let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;
```

It also uses PostgreSQL-specific JSONB operators:
```sql
COUNT(DISTINCT c.id) FILTER (WHERE c.worktree_ids @> jsonb_build_array(w.id))
```

**Solution**: Refactor `get_status()` to accept `Arc<dyn VectorStore>` and use existing trait methods.

**New Implementation**:
```rust
pub async fn get_status(
    store: Arc<dyn VectorStore>,
    repo_filter: Option<&str>,
    worktree_filter: Option<&str>,
) -> Result<StatusResponse> {
    // Use trait methods instead of direct queries
    let repos = store.list_repos().await?;

    let mut repo_statuses = Vec::new();
    for repo in repos {
        if let Some(filter) = repo_filter {
            if repo.name != filter { continue; }
        }

        let worktrees = store.list_worktrees(&repo.name).await?;
        let mut worktree_statuses = Vec::new();

        for worktree in worktrees {
            if let Some(filter) = worktree_filter {
                if worktree.name != filter { continue; }
            }
            // Build status from worktree info
            worktree_statuses.push(WorktreeStatus {
                name: worktree.name,
                root_path: worktree.root_path,
                // Note: chunk counts may need a new trait method or iteration
            });
        }

        repo_statuses.push(RepoStatus {
            name: repo.name,
            worktrees: worktree_statuses,
        });
    }

    Ok(StatusResponse { repos: repo_statuses })
}
```

**Chunk Count Consideration**: The current status queries count chunks per worktree with JSONB operators. Options:
1. Add `count_chunks_by_worktree()` to VectorStore trait
2. Accept approximate counts from `list_worktrees()` metadata
3. Skip chunk counts in SQLite status (simpler output)

For MVP, option 3 (skip detailed counts for SQLite) is acceptable.

### daemon/mod.rs Modifications

1. **Update DaemonState**:
```rust
struct DaemonState {
    store: Arc<dyn VectorStore>,
    embedding_service: EmbeddingService,
}
```

2. **Update run()**:
```rust
pub async fn run() -> Result<()> {
    let store = db::factory::get_store().await?;
    let embedding_service = EmbeddingService::from_env().await?;
    let state = Arc::new(DaemonState { store, embedding_service });
    // ... rest unchanged
}
```

3. **Update execute_search()** - Replace raw queries with trait methods:

**Current Problem** (lines 108-150):
```rust
// Raw query for repo lookup
let repo_row = client.query_one(
    "SELECT id FROM maproom.repos WHERE name = $1",
    &[&params.repo],
)?;

// Raw query for chunk details
let chunk_row = client.query_opt(
    r#"SELECT c.start_line, c.end_line, ... FROM maproom.chunks c ..."#,
)?;
```

**New Implementation**:
```rust
async fn execute_search(state: Arc<DaemonState>, params: SearchParams) -> Result<Value> {
    // Use trait methods instead of raw queries
    let hits = match params.mode.as_deref() {
        Some("fts") | None => {
            state.store.search_chunks_fts(
                &params.repo,
                params.worktree.as_deref(),
                &params.query,
                params.limit.unwrap_or(10) as i64,
                false
            ).await?
        }
        Some("vector") => {
            state.store.search_chunks_vector(
                &params.repo,
                params.worktree.as_deref(),
                &params.query,
                params.limit.unwrap_or(10) as i64,
            ).await?
        }
        Some("hybrid") => {
            state.store.search_chunks_hybrid(
                &params.repo,
                params.worktree.as_deref(),
                &params.query,
                params.limit.unwrap_or(10) as i64,
            ).await?
        }
        _ => anyhow::bail!("Unknown search mode"),
    };

    // SearchHit already contains chunk metadata from trait methods
    format_search_response(hits)
}
```

**Chunk Detail Fetching**: The `SearchHit` struct returned by trait methods already includes:
- `chunk_id`, `file_path`, `start_line`, `end_line`
- `symbol_name`, `kind`, `content`
- `score`

No additional queries needed for formatting search results.

### db/factory.rs Enhancements

1. **Add BackendType enum**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    PostgreSQL,
    SQLite,
}
```

2. **Add trait method for backend detection** (if not already in VectorStore):
```rust
fn backend_type(&self) -> BackendType;
```

## File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `db/mod.rs` | MODIFY | Add BackendType enum and backend_type() to VectorStore trait |
| `db/postgres/mod.rs` | MODIFY | Implement backend_type() returning PostgreSQL |
| `db/sqlite/mod.rs` | MODIFY | Implement backend_type() returning SQLite |
| `main.rs` | MODIFY | Replace db::connect() with get_store() for MVP commands |
| `daemon/mod.rs` | MODIFY | Replace PgPool with Arc<dyn VectorStore>, use trait search methods |
| `status.rs` | REWRITE | Accept Arc<dyn VectorStore>, remove direct connection, use trait methods |
| `daemon/types.rs` | NONE | No changes needed |

## Deferred Items (Phase 2)

| File | Change Type | Description |
|------|-------------|-------------|
| `indexer/mod.rs` | MAJOR REFACTOR | Abstract to accept `&dyn VectorStore` |
| `main.rs` (scan/upsert/watch) | DEFERRED | Enable SQLite for indexing commands |
| `generate-embeddings` | DEFERRED | Abstract embedding pipeline |

## Error Handling

### Backend-Specific Errors

```rust
// Graceful degradation for missing features
match store.search_chunks_vector(...).await {
    Ok(hits) => hits,
    Err(e) if e.to_string().contains("sqlite-vec") => {
        eprintln!("Vector search unavailable (sqlite-vec not installed)");
        eprintln!("Falling back to FTS search");
        store.search_chunks_fts(...).await?
    }
    Err(e) => return Err(e),
}
```

### Missing Configuration

```rust
// If no database URL and no SQLite file exists
let store = match get_store().await {
    Ok(s) => s,
    Err(e) => {
        eprintln!("No database configured. Options:");
        eprintln!("  1. Set MAPROOM_DATABASE_URL environment variable");
        eprintln!("  2. Run 'maproom init' to create SQLite database");
        return Err(e);
    }
};
```

## Performance Considerations

1. **Connection Reuse**: Store is created once per command, reused throughout
2. **No Pool for SQLite**: Single connection is sufficient (no pool overhead)
3. **Async/Spawn Blocking**: SQLite operations use `spawn_blocking` as implemented in VECSTORE
4. **Parallel Disabled**: Sequential scans for SQLite prevent lock contention

## Testing Strategy

See `quality-strategy.md` for full testing approach. Key points:

1. **Unit Tests**: Mock VectorStore for command handler testing
2. **Integration Tests**: Test full command flow with SQLite backend
3. **E2E Tests**: `cargo run -- scan` with real SQLite database

## Migration Path

### For Existing Users (PostgreSQL)
- No changes required
- Existing `MAPROOM_DATABASE_URL` continues to work
- All commands behave identically

### For New Users (SQLite)
- No configuration needed
- First `scan` creates `~/.maproom/maproom.db`
- Full functionality without Docker

### For Developers
1. Build with `--features sqlite` for SQLite support
2. Test both backends before release
3. CI runs tests against both backends
