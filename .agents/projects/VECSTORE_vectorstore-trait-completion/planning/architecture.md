# VECSTORE Architecture: VectorStore Trait Completion

## Solution Overview

Complete the `VectorStore` trait abstraction by adding all missing database operations as trait methods. Both `PostgresStore` and `SqliteStore` will implement these methods, enabling the CLI, daemon, and indexer to use `Arc<dyn VectorStore>` instead of direct PostgreSQL connections.

## Architecture Decisions

### ADR-1: Expand Existing Trait vs. Multiple Traits

**Decision**: Expand the existing `VectorStore` trait with additional methods.

**Rationale**:
- Single trait is simpler than multiple (SearchStore, ContextStore, etc.)
- All operations relate to vector/semantic code storage
- Existing codebase already uses `Arc<dyn VectorStore>`
- Both backends need all operations—no selective implementation needed

**Alternatives Rejected**:
- Multiple specialized traits: Added complexity without clear benefit
- Generic repository pattern: Over-engineering for 2 backends

### ADR-2: Method Grouping Within Trait

**Decision**: Organize trait methods by logical group with comments, not separate traits.

**Structure**:
```rust
pub trait VectorStore: Send + Sync {
    // --- Repository & Worktree ---
    // --- Indexing ---
    // --- Embeddings ---
    // --- Search (FTS, Vector, Hybrid) ---
    // --- Context Assembly ---
    // --- Repository Queries ---
    // --- Index State ---
    // --- Cleanup ---
    // --- Migrations ---
}
```

### ADR-3: Return Types

**Decision**: Use domain structs instead of raw database types.

**New Types**:
```rust
// Already defined
pub struct SearchHit { ... }
pub struct FileRecord { ... }
pub struct ChunkRecord { ... }  // Used for INSERTION (write path)

// To add - Read Path Types
// ChunkRecord is for writing (insertion), ChunkFull/ChunkSummary are for reading

/// Full chunk data for display/context - read-only view of a chunk
/// Contains all fields from ChunkRecord plus computed fields
pub struct ChunkFull {
    pub id: i64,              // Database ID (not in ChunkRecord)
    pub file_id: i64,
    pub blob_sha: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
    pub preview: String,
    pub content: String,      // Full chunk content (may not be in ChunkRecord)
    pub file_path: String,    // Denormalized from file table
    pub worktree_id: i64,
}

/// Lightweight chunk reference for lists/navigation
/// Minimal fields for displaying chunk references without full content
pub struct ChunkSummary {
    pub id: i64,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub file_path: String,
}

/// Context around a chunk - surrounding and related chunks
pub struct ChunkContext {
    pub chunk: ChunkFull,
    pub file_path: String,
    pub surrounding_chunks: Vec<ChunkSummary>,  // Chunks before/after by line number
    pub related_chunks: Vec<ChunkSummary>,      // Chunks related by edges (callers, callees)
}

/// Repository metadata
pub struct RepoInfo {
    pub id: i64,
    pub name: String,
    pub root_path: String,
}

/// Worktree metadata
pub struct WorktreeInfo {
    pub id: i64,
    pub repo_id: i64,
    pub name: String,
    pub abs_path: String,
}

/// Index state for a worktree
pub struct IndexState {
    pub worktree_id: i64,
    pub tree_sha: String,
    pub last_indexed: DateTime<Utc>,
    pub files_indexed: i64,
    pub chunks_indexed: i64,
}

/// Report from cleanup operations
pub struct CleanupReport {
    pub worktree_id: i64,
    pub chunks_deleted: u64,
    pub files_deleted: u64,
    pub embeddings_deleted: u64,
}
```

**Type Relationships**:
- `ChunkRecord` → Write path (insertion into database)
- `ChunkFull` → Read path (full chunk retrieval with all data)
- `ChunkSummary` → Read path (lightweight references in lists)
- `SearchHit` → Search results (includes score, not same as ChunkFull)

### ADR-4: Error Handling

**Decision**: Continue using `anyhow::Result<T>` for all trait methods.

**Rationale**:
- Consistent with existing codebase
- Allows rich error context via `.context()`
- Both backends can wrap their specific errors

### ADR-5: Async All Methods

**Decision**: All trait methods are `async`.

**Rationale**:
- Consistent API surface
- PostgreSQL is inherently async (tokio-postgres)
- SQLite uses `spawn_blocking` to be async-compatible
- Callers don't need to know which backend they're using

### ADR-6: Context Assembly Scope

**Decision**: Implement simplified context methods in trait; keep sophisticated context assembly as higher-level code.

**Context**: The existing `context/assembler.rs` module has 400+ lines of sophisticated context assembly logic including:
- Strategy pattern for different context types
- Relationship traversal via graph.rs
- Smart expansion based on token budgets
- Multiple assembly modes (surrounding, semantic, hybrid)

**Decision**:
- `get_chunk_by_id()` - Simple single-chunk lookup (trait method)
- `get_file_chunks()` - All chunks for a file (trait method)
- `get_chunk_context(chunk_id, surrounding)` - Simplified: returns N chunks before/after by line number (trait method)
- Full context assembly (ContextAssembler, strategies) - Remains as higher-level code that uses VectorStore

**Rationale**:
- Keeps trait focused on data access, not business logic
- Context assembly strategies need flexibility that traits don't provide well
- The sophisticated logic can use trait methods internally
- Simpler trait is easier to implement correctly in both backends

**Alternatives Rejected**:
- Moving all context logic to trait: Too complex, business logic in data layer
- Creating separate ContextStore trait: Over-engineering for current needs

### ADR-7: Embedding Dimension Handling

**Decision**: Both backends support 768-dim (Ollama) and 1536-dim (OpenAI) embeddings with dimension-specific storage.

**Context**:
- SQLite is currently hardcoded for 1536-dim (OpenAI) - **MUST BE FIXED**
- PostgreSQL already supports 768 (Ollama) and 1536 (OpenAI)
- Ollama is the zero-config default embedding provider
- EMBPERF project optimizes Ollama performance (produces 768-dim embeddings)

**Decision**:
- Keep `dimension` parameter in embedding methods
- **SQLite: Add 768-dim support** using separate tables (like PostgreSQL's column approach)
- PostgreSQL: Continue using column-based dimension routing
- Both backends validate dimension at upsert time

**SQLite 768-dim Implementation**:
```sql
-- Existing table (1536-dim, OpenAI)
CREATE VIRTUAL TABLE IF NOT EXISTS vec_code_embeddings USING vec0(
    embedding float[1536]
);

-- NEW table (768-dim, Ollama)
CREATE VIRTUAL TABLE IF NOT EXISTS vec_code_embeddings_768 USING vec0(
    embedding float[768]
);

-- Routing table to track which dimension each chunk uses
-- (or use embedding_dim column in code_embeddings)
```

**Rationale**:
- **Ollama priority**: Zero-config experience requires SQLite + Ollama to work together
- **EMBPERF dependency**: Parallel Ollama optimization produces 768-dim embeddings
- sqlite-vec supports dynamic dimensions via separate tables
- Maintains backward compatibility with existing 1536-dim data

### ADR-8: Transaction Semantics

**Decision**: Individual trait methods are NOT transactional; callers manage transactions where needed.

**Context**:
- `delete_worktree_data()` deletes from multiple tables
- PostgreSQL cleanup uses explicit transactions
- SQLite uses auto-commit by default

**Decision**:
- Individual methods complete atomically within their scope
- Batch methods (like `delete_worktree_data`) handle their own transaction
- No cross-method transaction API in trait (too complex)

**Rationale**:
- Simpler trait API
- Each backend manages transactions appropriately
- `delete_worktree_data` is the only multi-table operation; it handles its own transaction

### ADR-9: SQLite Multi-Dimension Embedding Schema

**Decision**: Use separate sqlite-vec virtual tables for each embedding dimension.

**Context**:
- sqlite-vec requires fixed dimensions per virtual table (`float[N]`)
- PostgreSQL uses separate columns (`code_embedding` vs `code_embedding_ollama`)
- Ollama produces 768-dim, OpenAI produces 1536-dim
- Users may switch providers or use both

**Schema Design**:
```sql
-- Metadata table (existing, add embedding_dim tracking)
CREATE TABLE IF NOT EXISTS code_embeddings (
    id INTEGER PRIMARY KEY,
    blob_sha TEXT NOT NULL UNIQUE,
    embedding_dim INTEGER NOT NULL DEFAULT 1536,  -- Track which dimension
    model_version TEXT NOT NULL DEFAULT 'unknown',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 1536-dim vector table (existing)
CREATE VIRTUAL TABLE IF NOT EXISTS vec_code_embeddings USING vec0(
    embedding float[1536]
);

-- 768-dim vector table (NEW)
CREATE VIRTUAL TABLE IF NOT EXISTS vec_code_embeddings_768 USING vec0(
    embedding float[768]
);

-- Optional: text embeddings follow same pattern
CREATE VIRTUAL TABLE IF NOT EXISTS vec_text_embeddings USING vec0(
    embedding float[1536]
);

CREATE VIRTUAL TABLE IF NOT EXISTS vec_text_embeddings_768 USING vec0(
    embedding float[768]
);
```

**Query Routing**:
```rust
// In sqlite/embeddings.rs
fn get_vec_table_name(dimension: usize) -> &'static str {
    match dimension {
        768 => "vec_code_embeddings_768",
        1536 => "vec_code_embeddings",
        _ => panic!("Unsupported dimension: {}", dimension),
    }
}

// In sqlite/vector.rs - search routes to correct table based on query embedding dimension
pub fn search_vector(conn: &Connection, embedding: &[f32], k: i64) -> Result<Vec<SearchHit>> {
    let table = get_vec_table_name(embedding.len());
    // Query the appropriate table
}
```

**Migration Strategy**:
1. Add new 768-dim tables (non-breaking)
2. Update upsert functions to route by dimension
3. Update search functions to query correct table
4. Existing 1536-dim data unchanged

**Alternatives Rejected**:
- Single table with dynamic dimensions: sqlite-vec doesn't support this
- Convert all embeddings to one dimension: Loses model-specific information
- Store as JSON: Terrible search performance

## Component Design

### VectorStore Trait (Expanded)

```rust
// crates/maproom/src/db/mod.rs

#[async_trait]
pub trait VectorStore: Send + Sync {
    // === Repository & Worktree ===
    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64>;
    async fn get_or_create_worktree(&self, repo_id: i64, name: &str, abs_path: &str) -> anyhow::Result<i64>;
    async fn get_or_create_commit(&self, repo_id: i64, sha: &str, committed_at: Option<DateTime<Utc>>) -> anyhow::Result<i64>;
    async fn get_repo_by_name(&self, name: &str) -> anyhow::Result<Option<RepoInfo>>;
    async fn get_worktree_by_name(&self, repo_id: i64, name: &str) -> anyhow::Result<Option<WorktreeInfo>>;
    async fn list_repos(&self) -> anyhow::Result<Vec<RepoInfo>>;
    async fn list_worktrees(&self, repo_id: i64) -> anyhow::Result<Vec<WorktreeInfo>>;

    // === Indexing ===
    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64>;
    async fn insert_chunk(&self, chunk: &ChunkRecord) -> anyhow::Result<i64>;
    async fn insert_chunks_batch(&self, chunks: &[ChunkRecord]) -> anyhow::Result<Vec<i64>>;
    async fn insert_chunk_edge(&self, src_chunk_id: i64, dst_chunk_id: i64, edge_type: &str) -> anyhow::Result<()>;
    async fn delete_chunks_by_file(&self, file_id: i64) -> anyhow::Result<u64>;
    async fn get_chunks_by_blob_sha(&self, blob_sha: &str) -> anyhow::Result<Vec<ChunkSummary>>;

    // === Embeddings ===
    async fn upsert_embeddings(&self, chunk_id: i64, code_embedding: Option<&[f32]>, text_embedding: Option<&[f32]>, dimension: usize) -> anyhow::Result<()>;
    async fn batch_upsert_embeddings(&self, embeddings: &[(i64, Option<Vec<f32>>, Option<Vec<f32>>)], dimension: usize) -> anyhow::Result<()>;

    // === Search ===
    async fn search_chunks_fts(&self, repo: &str, worktree: Option<&str>, query: &str, k: i64, debug: bool) -> anyhow::Result<Vec<SearchHit>>;
    async fn search_chunks_vector(&self, repo: &str, worktree: Option<&str>, embedding: &[f32], k: i64, debug: bool) -> anyhow::Result<Vec<SearchHit>>;
    async fn search_chunks_hybrid(&self, repo: &str, worktree: Option<&str>, query: &str, embedding: &[f32], k: i64, debug: bool) -> anyhow::Result<Vec<SearchHit>>;

    // === Context Assembly ===
    async fn get_chunk_by_id(&self, chunk_id: i64) -> anyhow::Result<Option<ChunkFull>>;
    async fn get_file_chunks(&self, file_id: i64) -> anyhow::Result<Vec<ChunkSummary>>;
    async fn get_chunk_context(&self, chunk_id: i64, surrounding: usize) -> anyhow::Result<Option<ChunkContext>>;

    // === Lookup ===
    async fn find_chunk_by_symbol(&self, repo_id: i64, worktree_id: Option<i64>, symbol_name: &str, relpath: Option<&str>) -> anyhow::Result<Option<i64>>;

    // === Index State ===
    async fn get_last_indexed_tree(&self, worktree_id: i64) -> anyhow::Result<Option<String>>;
    async fn update_index_state(&self, worktree_id: i64, tree_sha: &str, files_indexed: i64, chunks_indexed: i64) -> anyhow::Result<()>;

    // === Cleanup ===
    async fn detect_stale_worktrees(&self, repo_id: i64) -> anyhow::Result<Vec<StaleWorktree>>;
    async fn delete_worktree_data(&self, worktree_id: i64) -> anyhow::Result<CleanupReport>;

    // === Migrations ===
    async fn migrate(&self) -> anyhow::Result<()>;
    async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>>;
}
```

### PostgresStore Implementation Pattern

```rust
// crates/maproom/src/db/postgres/mod.rs

impl PostgresStore {
    pub async fn connect() -> anyhow::Result<Self> {
        let pool = crate::db::pool::create_pool().await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl VectorStore for PostgresStore {
    // Pattern: get connection, delegate to queries.rs
    async fn search_chunks_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        k: i64,
        debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let client = self.pool.get().await.context("Failed to get connection")?;
        super::queries::search_chunks_vector(&client, repo, worktree, embedding, k, debug).await
    }
    // ... similar for other methods
}
```

### PostgreSQL Query Functions: Existing vs Required

**IMPORTANT**: The following audit identifies which queries.rs functions exist vs need to be written.

#### EXISTING in queries.rs (wrap directly)
- ✅ `search_chunks_fts()` - Full-text search
- ✅ `upsert_embeddings()` - Embedding storage
- ✅ `get_or_create_repo()` - Repository creation
- ✅ `get_or_create_worktree()` - Worktree creation
- ✅ `get_or_create_commit()` - Commit tracking
- ✅ `upsert_file()` - File upsert
- ✅ `insert_chunk()` - Chunk insertion
- ✅ `insert_chunks_batch()` - Batch chunk insertion
- ✅ `find_chunk_by_symbol()` - Symbol lookup

#### EXISTING in other modules (need integration)
- ✅ `index_state.rs::get_last_indexed_tree()` - Index state query
- ✅ `index_state.rs::update_index_state()` - Index state update
- ✅ `cleanup.rs::StaleWorktreeDetector` - Stale detection (class, not function)

#### MISSING - Must be written for VECSTORE
- ❌ `search_chunks_vector()` - Vector similarity search (pgvector query)
- ❌ `search_chunks_hybrid()` - Hybrid RRF search (FTS + pgvector fusion)
- ❌ `get_chunk_by_id()` - Single chunk lookup by ID
- ❌ `get_file_chunks()` - All chunks for a file
- ❌ `get_repo_by_name()` - Repository lookup by name
- ❌ `list_repos()` - List all repositories
- ❌ `get_worktree_by_name()` - Worktree lookup by name
- ❌ `list_worktrees()` - List worktrees for repo
- ❌ `get_chunks_by_blob_sha()` - Chunks with given blob SHA
- ❌ `delete_chunks_by_file()` - Delete chunks for file
- ❌ `delete_worktree_data()` - Delete all worktree data

**Implementation Note**: Each missing function requires:
1. Write SQL query using PostgreSQL-specific features (pgvector, tsvector)
2. Add parameterized query function to queries.rs
3. Wire into PostgresStore trait implementation
4. Unit test the query

### SqliteStore Implementation Pattern

```rust
// crates/maproom/src/db/sqlite/mod.rs

impl SqliteStore {
    pub async fn connect(path: &str) -> anyhow::Result<Self> { ... }

    // Helper for sync operations
    async fn run<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&mut Connection) -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    { ... }

    // Check if sqlite-vec extension is available
    pub fn has_vec_extension(&self) -> bool { ... }
}

#[async_trait]
impl VectorStore for SqliteStore {
    // Pattern: use run() helper, delegate to module functions
    async fn search_chunks_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        k: i64,
        debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let repo = repo.to_string();
        let worktree = worktree.map(String::from);
        let embedding = embedding.to_vec();
        let has_vec = self.has_vec_extension();

        self.run(move |conn| {
            if !has_vec {
                anyhow::bail!("Vector search requires sqlite-vec extension");
            }
            vector::search_vector(conn, &repo, worktree.as_deref(), &embedding, k, debug)
        }).await
    }
    // ... similar for other methods
}
```

### sqlite-vec Graceful Degradation

**Dependency**: sqlite-vec is statically linked via the `sqlite` feature flag. When available, it enables vector similarity search. When not available, vector search returns an error but other operations work.

**Behavior Matrix**:

| Operation | sqlite-vec Available | sqlite-vec NOT Available |
|-----------|---------------------|-------------------------|
| FTS search | ✅ Works | ✅ Works |
| Vector search | ✅ Works | ❌ Returns error |
| Hybrid search | ✅ Works | ⚠️ Falls back to FTS only |
| Chunk insert | ✅ Works | ✅ Works |
| Embedding upsert | ✅ Stores embedding | ⚠️ Stores but can't search |

**Implementation Requirements**:
1. `SqliteStore::has_vec_extension()` - Check at connection time
2. Vector search methods check `has_vec` before attempting
3. Hybrid search falls back to FTS-only when no sqlite-vec
4. Clear error messages explain the limitation

**Testing Requirements**:
1. Test vector search WITH sqlite-vec (normal case)
2. Test vector search WITHOUT sqlite-vec (graceful error)
3. Test hybrid search fallback behavior

## File Structure

```
crates/maproom/src/db/
├── mod.rs              # VectorStore trait + domain types (MODIFIED)
├── factory.rs          # get_store() factory (EXISTING)
├── connection.rs       # URL parsing (EXISTING)
├── pool.rs             # PostgreSQL pool (EXISTING)
├── queries.rs          # PostgreSQL query functions (ADD new functions)
├── columns.rs          # Embedding column selection (EXISTING)
├── index_state.rs      # Index state queries (INTEGRATE into trait)
├── cleanup.rs          # Cleanup operations (INTEGRATE into trait)
├── materialized_views.rs # PostgreSQL views (EXISTING)
├── postgres/
│   └── mod.rs          # PostgresStore (ADD new methods)
└── sqlite/
    ├── mod.rs          # SqliteStore (ADD new methods)
    ├── schema.rs       # DDL (EXISTING)
    ├── migrations.rs   # Migration runner (EXISTING)
    ├── embeddings.rs   # Embedding storage (EXISTING)
    ├── vector.rs       # Vector search (EXISTING)
    ├── fts.rs          # FTS search (EXISTING)
    ├── hybrid.rs       # Hybrid search (EXISTING)
    └── graph.rs        # Graph traversal (EXISTING)
```

## Migration Strategy

### Phase 1: Add Methods to Trait (Non-Breaking)

1. Add new method signatures to `VectorStore` trait
2. Add default implementations that panic (temporary)
3. Implement methods in `PostgresStore` (wrapping queries.rs)
4. Implement methods in `SqliteStore` (wrapping sqlite/* modules)
5. Remove default panic implementations

### Phase 2: Consumer Migration (CLI/Daemon/Indexer)

**After trait is complete**, separate project (MAPROOMCLI) will:
1. Update main.rs to use `get_store()` instead of `db::connect()`
2. Update daemon to accept `Arc<dyn VectorStore>`
3. Update indexer to use trait methods instead of pool.get()

## Technology Choices

### Already Decided (No Changes)
- **PostgreSQL**: tokio-postgres + deadpool + pgvector
- **SQLite**: rusqlite + r2d2 + sqlite-vec
- **Async**: tokio runtime + async_trait
- **Error Handling**: anyhow + thiserror

### Constraints
- **Rust 1.70+**: Required for async trait support
- **Feature Flag**: `sqlite` feature controls SQLite backend compilation
- **No Breaking Changes**: Existing VectorStore methods unchanged

## Performance Considerations

### Connection Pooling
- PostgreSQL: deadpool-postgres (10 connections default)
- SQLite: r2d2 (10 connections default)

### Query Patterns
- Both backends use parameterized queries
- Batch operations for chunk insertion
- Indexed columns for search (GIN/FTS5, HNSW/ivfflat)

### Memory
- Embeddings are Vec<f32> (1536 × 4 = 6KB per chunk for OpenAI)
- Batch operations should limit batch size (~100 chunks)

## Long-Term Maintainability

### Adding New Backends
1. Implement `VectorStore` trait
2. Add feature flag
3. Update `factory.rs:get_store()` to detect new backend

### Adding New Operations
1. Add method to `VectorStore` trait
2. Implement in `PostgresStore`
3. Implement in `SqliteStore`
4. No consumer changes needed (they use trait)

### Testing Strategy
- Unit tests per backend in `db/postgres/tests/` and `db/sqlite/tests/`
- Integration tests using `get_store()` with both backends
- Feature-gated tests for SQLite-specific behavior
