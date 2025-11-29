# Architecture: Full SQLite Implementation

## Design Principles

1. **SQLite-native**: Optimize for SQLite, not abstraction compatibility
2. **Single-file simplicity**: All data in one `.maproom.db` file
3. **Zero external dependencies**: No Docker, no servers, no network
4. **Build on existing work**: Extend current sqlite/mod.rs (645 lines), don't replace

## Existing Implementation

The SQLFIX project established a working SQLite backend in `crates/maproom/src/db/sqlite/`:

### What Exists and Must Be Preserved

| File | Contents | Status |
|------|----------|--------|
| `mod.rs` (645 lines) | SqliteStore, VectorStore impl, all CRUD | Working - extend, don't replace |
| `schema.rs` (130 lines) | Table creation via `init_schema()` | Working - needs migration wrapper |

### Key Patterns to Preserve

1. **Async via spawn_blocking**: All database operations use `spawn_blocking` to avoid blocking the Tokio runtime
   ```rust
   async fn run<F, T>(&self, f: F) -> anyhow::Result<T>
   where
       F: FnOnce(&mut Connection) -> anyhow::Result<T> + Send + 'static,
       T: Send + 'static,
   {
       let pool = self.pool.clone();
       spawn_blocking(move || {
           let mut conn = pool.get()?;
           f(&mut conn)
       }).await?
   }
   ```

2. **sqlite-vec extension loading**: Already implemented via `sqlite3_auto_extension` before pool creation
   ```rust
   // crates/maproom/src/db/sqlite/mod.rs lines 40-42
   unsafe {
       rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
   }
   ```

3. **Connection pooling**: r2d2_sqlite with WAL mode and proper PRAGMAs

4. **FTS5 sync**: Manual INSERT into fts_chunks (not triggers) - consistent with PostgreSQL approach

### What This Project Adds

| New Module | Purpose |
|------------|---------|
| `migrations.rs` | Schema versioning and migration runner |
| `embeddings.rs` | Deduplicated embedding storage |
| `vector.rs` | Vector similarity search |
| `hybrid.rs` | RRF fusion search |
| `graph.rs` | Recursive CTE traversals |

## Reusable Utilities

These existing utilities should be imported, NOT reimplemented:

### From `src/search/fts.rs`
```rust
/// Normalize query for exact match detection
/// Handles: camelCase, XMLParser, HTTPSHandler, kebab-case
pub fn normalize_for_exact_match(query: &str) -> String
```
**Usage**: Import in semantic ranking for exact match boost detection

### From `src/db/columns.rs`
```rust
/// Select database columns based on embedding dimension
pub fn select_columns_for_dimension(dimension: usize) -> Result<ColumnSet>
```
**Note**: For MVP, we use 1536-dim only. This utility is PostgreSQL-specific (uses column names). SQLite will store dimension in `code_embeddings.embedding_dim` column.

### From `src/search/executor_types.rs`
```rust
pub struct RankedResult { pub chunk_id: i64, pub score: f64 }
pub struct RankedResults { pub results: Vec<RankedResult>, pub source: SearchSource }
```
**Note**: Consider reusing if compatible, but SQLite-specific structs acceptable for simplicity.

## Module Structure

```
crates/maproom/src/db/sqlite/
├── mod.rs              # SqliteStore struct (EXISTING - extend)
├── schema.rs           # Table creation (EXISTING - extend)
├── migrations.rs       # Migration runner (NEW)
├── embeddings.rs       # Embedding storage/dedup (NEW)
├── vector.rs           # Vector search with sqlite-vec (NEW)
├── hybrid.rs           # RRF fusion search (NEW)
└── graph.rs            # Graph traversal queries (NEW)
```

## Migration System

### Critical Requirement

The existing `init_schema()` only creates tables if not exists. Users upgrading from SQLFIX need a proper migration path.

### Migration Versioning

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Migration Runner (`migrations.rs`)

```rust
pub struct MigrationRunner<'a> {
    conn: &'a Connection,
}

impl<'a> MigrationRunner<'a> {
    /// Get current schema version (0 if no migrations table)
    pub fn current_version(&self) -> Result<i32>;

    /// Apply all pending migrations in order
    pub fn migrate(&self) -> Result<()>;

    /// List of migrations (version, name, SQL)
    fn migrations() -> Vec<Migration>;
}

struct Migration {
    version: i32,
    name: &'static str,
    up: &'static str,
    down: &'static str,  // For rollback
}
```

### Migration Sequence

| Version | Name | Description |
|---------|------|-------------|
| 1 | `init_schema` | Wrap existing schema creation |
| 2 | `add_chunk_worktrees` | Add junction table |
| 3 | `add_code_embeddings` | Add deduplicated embeddings table |
| 4 | `add_vec_code` | Add vector index table |
| 5 | `drop_worktree_ids` | Remove deprecated JSON column |
| 6 | `drop_vec_chunks` | Remove deprecated vec_chunks table |

> **Note**: No data migration is required. There are no existing SQLite databases with data to migrate. Fresh indexing populates all tables from scratch.

### Migration Safety

- Each migration is idempotent (re-running is safe)
- Migrations run in a transaction
- Failed migration rolls back cleanly
- Version is recorded only after successful apply

## Schema Design

### Core Tables (Existing - from SQLFIX)

```sql
-- Repository metadata (EXISTING)
CREATE TABLE repos (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  root_path TEXT NOT NULL
);

-- Worktree tracking (EXISTING)
CREATE TABLE worktrees (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  abs_path TEXT NOT NULL,
  UNIQUE(repo_id, name)
);

-- Commit tracking (EXISTING)
CREATE TABLE commits (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
  sha TEXT NOT NULL,
  committed_at DATETIME,
  UNIQUE(repo_id, sha)
);

-- File tracking (EXISTING)
CREATE TABLE files (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
  worktree_id INTEGER NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,
  commit_id INTEGER NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
  relpath TEXT NOT NULL,
  language TEXT,
  content_hash TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  last_modified DATETIME,
  UNIQUE(commit_id, relpath, content_hash)
);

-- Code chunks (EXISTING - worktree_ids column will be dropped)
CREATE TABLE chunks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  blob_sha TEXT NOT NULL,
  symbol_name TEXT,
  kind TEXT NOT NULL,
  signature TEXT,
  docstring TEXT,
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  preview TEXT NOT NULL,
  ts_doc_text TEXT,
  recency_score REAL NOT NULL,
  churn_score REAL NOT NULL,
  metadata JSON,
  worktree_ids JSON NOT NULL,  -- DEPRECATED: will be dropped (Migration 5)
  UNIQUE(file_id, start_line, end_line)
);

-- Chunk edges (EXISTING)
CREATE TABLE chunk_edges (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  src_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
  dst_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
  type TEXT NOT NULL,
  UNIQUE(src_chunk_id, dst_chunk_id, type)
);
```

### New Tables (Added by Migrations)

```sql
-- Chunk-worktree junction (proper relational design)
-- Migration 2: add_chunk_worktrees
CREATE TABLE chunk_worktrees (
  chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
  worktree_id INTEGER NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,
  PRIMARY KEY (chunk_id, worktree_id)
);

-- Deduplicated embeddings by content hash
-- Migration 3: add_code_embeddings
CREATE TABLE code_embeddings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  blob_sha TEXT NOT NULL UNIQUE,
  embedding BLOB,                -- float32 array as little-endian bytes
  embedding_dim INTEGER NOT NULL DEFAULT 1536,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Vector index for similarity search (1536-dim only for MVP)
-- Migration 4: add_vec_code
CREATE VIRTUAL TABLE vec_code USING vec0(
  embedding float[1536]
);
```

### Deprecated Table: vec_chunks

The existing `vec_chunks` table (created by SQLFIX) is **DEPRECATED** and replaced by the new `code_embeddings` + `vec_code` architecture.

```sql
-- DEPRECATED: Will be dropped by Migration 7
-- This was the original SQLFIX design (chunk_id keyed, no deduplication)
CREATE VIRTUAL TABLE vec_chunks USING vec0(
    chunk_id INTEGER PRIMARY KEY,
    code_embedding float[1536],
    text_embedding float[1536]
);
```

**Why Replace vec_chunks:**

| Aspect | vec_chunks (Old) | code_embeddings + vec_code (New) |
|--------|------------------|----------------------------------|
| Key | chunk_id | blob_sha (content hash) |
| Deduplication | None - same content = N embeddings | Yes - same content = 1 embedding |
| Storage | 70-90% waste on shared content | Minimal waste |
| Text embedding | Stored separately | Combined into single embedding |

**Migration Path (Migration 6: drop_vec_chunks):**

```sql
-- Migration 6: drop_vec_chunks
-- No data migration needed - fresh indexing populates code_embeddings
DROP TABLE IF EXISTS vec_chunks;
```

**Code Migration:**

The existing `VectorStore::upsert_embeddings(chunk_id, ...)` method in `mod.rs` uses `vec_chunks`. This method will be deprecated in favor of new SQLite-specific methods:

```rust
// DEPRECATED (mod.rs:343) - uses vec_chunks
async fn upsert_embeddings(&self, chunk_id: i64, ...) -> Result<()>

// NEW (embeddings.rs) - uses code_embeddings + vec_code
impl SqliteStore {
    pub async fn upsert_embedding(&self, blob_sha: &str, embedding: &[f32], model: &str) -> Result<i64>
}
```

**Note:** The new `upsert_embedding()` method is SQLite-specific and NOT part of the `VectorStore` trait. This is intentional per the "SQLite-native" design principle - we optimize for SQLite, not abstraction compatibility.

### FTS5 Table (Existing)

```sql
-- FTS5 with external content table (EXISTING)
CREATE VIRTUAL TABLE fts_chunks USING fts5(
  content,
  docstring,
  symbol_name,
  content='chunks',
  content_rowid='id'
);
```

### FTS5 Sync Strategy

**Decision**: Manual INSERT (not triggers)

Rationale:
- Consistent with PostgreSQL implementation
- More control over when indexing happens
- Easier to debug and test
- Triggers add complexity for minimal benefit

On chunk insert/update, application code must:
```rust
// After inserting chunk with id
conn.execute(
    "INSERT INTO fts_chunks(rowid, content, docstring, symbol_name)
     VALUES (?1, ?2, ?3, ?4)",
    params![chunk_id, preview, docstring, symbol_name],
)?;
```

### Indexes

```sql
-- Performance indexes
CREATE INDEX idx_files_repo_worktree ON files(repo_id, worktree_id);
CREATE INDEX idx_chunks_file ON chunks(file_id);
CREATE INDEX idx_chunks_blob_sha ON chunks(blob_sha);
CREATE INDEX idx_chunk_worktrees_worktree ON chunk_worktrees(worktree_id);
CREATE INDEX idx_chunk_edges_target ON chunk_edges(dst_chunk_id, type);
CREATE INDEX idx_embeddings_blob ON code_embeddings(blob_sha);
```

## Extension Verification

### Runtime Check

On first vector operation, verify sqlite-vec is properly loaded:

```rust
fn verify_vec_extension(conn: &Connection) -> Result<()> {
    // This will fail if sqlite-vec not loaded
    let result: Result<String, _> = conn.query_row(
        "SELECT vec_version()",
        [],
        |row| row.get(0),
    );

    match result {
        Ok(version) => {
            tracing::debug!("sqlite-vec version: {}", version);
            Ok(())
        }
        Err(_) => Err(SqliteError::VecExtensionMissing.into()),
    }
}
```

### Graceful Degradation

If sqlite-vec is missing:
1. Log warning: "Vector search disabled - sqlite-vec extension not loaded"
2. Vector search returns empty results (not error)
3. Hybrid search falls back to FTS-only mode
4. Embedding storage still works (stored but not indexed)

## Component Architecture

### Embeddings Module (`embeddings.rs`)

```rust
impl SqliteStore {
    /// Store embedding, deduplicating by blob_sha
    pub async fn upsert_embedding(
        &self,
        blob_sha: &str,
        embedding: &[f32],
        model_version: &str,
    ) -> Result<i64>;

    /// Batch upsert with deduplication
    pub async fn upsert_embeddings_batch(
        &self,
        embeddings: &[EmbeddingRecord],
    ) -> Result<()>;

    /// Check if embedding exists for blob_sha
    pub async fn has_embedding(&self, blob_sha: &str) -> Result<bool>;
}
```

### Vector Conversion

Convert between Rust Vec<f32> and SQLite BLOB:

```rust
/// Convert f32 slice to little-endian bytes for SQLite storage
fn vec_to_blob(vec: &[f32]) -> Vec<u8> {
    vec.iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// Convert bytes back to f32 slice
fn blob_to_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}

/// Format for sqlite-vec query: vec_f32('[0.1, 0.2, ...]')
fn vec_to_sqlite_literal(vec: &[f32]) -> String {
    let values: Vec<String> = vec.iter().map(|f| f.to_string()).collect();
    format!("vec_f32('[{}]')", values.join(", "))
}
```

### Vector Search Module (`vector.rs`)

```rust
impl SqliteStore {
    pub async fn search_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorResult>>;
}
```

**SQL Pattern (JOIN via blob_sha)**:
```sql
SELECT c.id, v.distance
FROM chunks c
JOIN code_embeddings e ON c.blob_sha = e.blob_sha
JOIN vec_code v ON v.rowid = e.id
JOIN files f ON f.id = c.file_id
WHERE v.embedding MATCH vec_f32(?)
  AND f.repo_id = ?
  AND (? IS NULL OR f.worktree_id = ?)
ORDER BY v.distance ASC
LIMIT ?
```

### FTS5 Rank Normalization

FTS5 rank is negative (more negative = better). Normalize to 0-1 (higher = better):

```rust
/// Normalize FTS5 rank to 0-1 scale
/// FTS5 rank: negative values, more negative = better match
fn normalize_fts_rank(rank: f64) -> f64 {
    // rank is typically -10 to 0
    // Convert to 0-1 where 1 is best
    1.0 / (1.0 + rank.abs())
}
```

### Hybrid Search Module (`hybrid.rs`)

```rust
const RRF_K: f64 = 60.0;  // Standard RRF constant

impl SqliteStore {
    pub async fn search_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: HybridWeights,
    ) -> Result<Vec<SearchHit>>;
}

/// RRF score calculation
fn rrf_score(fts_rank: Option<usize>, vec_rank: Option<usize>, weights: &HybridWeights) -> f64 {
    let fts_score = fts_rank
        .map(|r| weights.fts_weight / (RRF_K + r as f64))
        .unwrap_or(0.0);
    let vec_score = vec_rank
        .map(|r| weights.vector_weight / (RRF_K + r as f64))
        .unwrap_or(0.0);
    fts_score + vec_score
}
```

### Semantic Ranking

Reuse existing `normalize_for_exact_match` from `src/search/fts.rs`:

```rust
use crate::search::fts::normalize_for_exact_match;

pub struct SemanticRanking {
    pub kind_multipliers: HashMap<String, f64>,
    pub exact_match_boost: f64,
}

impl Default for SemanticRanking {
    fn default() -> Self {
        let mut kind_multipliers = HashMap::new();
        kind_multipliers.insert("function".to_string(), 1.2);
        kind_multipliers.insert("method".to_string(), 1.2);
        kind_multipliers.insert("class".to_string(), 1.1);
        kind_multipliers.insert("struct".to_string(), 1.1);
        kind_multipliers.insert("interface".to_string(), 1.1);
        kind_multipliers.insert("module".to_string(), 1.0);
        kind_multipliers.insert("variable".to_string(), 0.8);
        kind_multipliers.insert("constant".to_string(), 0.9);

        Self {
            kind_multipliers,
            exact_match_boost: 1.5,
        }
    }
}

/// Apply semantic ranking to search results
fn apply_semantic_ranking(
    results: &mut [SearchHit],
    query: &str,
    ranking: &SemanticRanking,
) {
    let normalized_query = normalize_for_exact_match(query);

    for hit in results.iter_mut() {
        // Apply kind multiplier
        let kind_mult = ranking.kind_multipliers
            .get(&hit.kind)
            .copied()
            .unwrap_or(1.0);

        // Apply exact match boost
        let exact_mult = if let Some(ref symbol) = hit.symbol_name {
            let normalized_symbol = normalize_for_exact_match(symbol);
            if normalized_symbol.contains(&normalized_query) {
                ranking.exact_match_boost
            } else {
                1.0
            }
        } else {
            1.0
        };

        hit.score *= kind_mult * exact_mult;
    }
}
```

### Graph Traversal Module (`graph.rs`)

```rust
impl SqliteStore {
    /// Find chunks that call target (with depth limit)
    pub async fn find_callers(
        &self,
        target_chunk_id: i64,
        max_depth: usize,  // Default: 3, Max: 10
    ) -> Result<Vec<GraphResult>>;

    /// Find chunks called by source
    pub async fn find_callees(
        &self,
        source_chunk_id: i64,
        max_depth: usize,
    ) -> Result<Vec<GraphResult>>;
}
```

**Graph Traversal Limits**:
- Default max_depth: 3
- Hard limit max_depth: 10 (prevent runaway recursion)
- Cycle detection via visited set in CTE

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SqliteError {
    #[error("Connection failed: {0}")]
    Connection(#[from] r2d2::Error),

    #[error("Query failed: {0}")]
    Query(#[from] rusqlite::Error),

    #[error("sqlite-vec extension not loaded - vector search disabled")]
    VecExtensionMissing,

    #[error("Embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Worktree not found: {0}")]
    WorktreeNotFound(String),

    #[error("Migration failed at version {version}: {message}")]
    MigrationFailed { version: i32, message: String },
}
```

## Configuration

```rust
pub struct SqliteConfig {
    pub path: String,                    // Database file path (required)
    pub pool_size: u32,                  // Connection pool size (default: 10)
    pub busy_timeout_ms: u32,            // Lock timeout (default: 5000)
    pub hybrid_weights: HybridWeights,   // FTS vs vector balance
}

pub struct HybridWeights {
    pub fts_weight: f64,     // Default 0.3
    pub vector_weight: f64,  // Default 0.7
}
```

**Note**: Database path is a parameter to `SqliteStore::connect()`. The `~/.maproom/` location is for documentation only - the caller decides the path.

## Performance Considerations

### Write Performance
- Batch inserts in transactions (1000 chunks per transaction)
- Defer FTS index updates until batch complete
- Use prepared statements for repeated operations

### Read Performance
- FTS5 and vec0 provide native indexing
- Junction table with covering index for worktree filters
- Over-fetch by 3x for hybrid search fusion (limit * 3)

### Concurrency
- WAL mode enables concurrent reads
- Single writer (SQLite limitation)
- busy_timeout handles lock contention
- Connection pool manages concurrent access

## Future Enhancements (Post-MVP)

1. **768-dim embedding support**: Add `vec_code_768` table and dimension routing
2. **FTS5 column weights**: Custom BM25 weights per column
3. **WAL checkpoint command**: Manual checkpoint for large imports
4. **Query plan caching**: Prepared statement caching for hot paths
