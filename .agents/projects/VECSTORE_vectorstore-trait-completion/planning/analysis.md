# VECSTORE Analysis: VectorStore Trait Completion

## Problem Definition

The `crewchief-maproom` codebase currently has two database backends:
1. **PostgreSQL** (production, feature-complete, supports Ollama 768-dim + OpenAI 1536-dim)
2. **SQLite** (recently added, zero-config option, **OpenAI 1536-dim ONLY**)

Both backends implement the `VectorStore` trait in `crates/maproom/src/db/mod.rs`. However, the trait is incomplete—many database operations still bypass the trait and call PostgreSQL-specific code directly via:
- `db::connect()`
- `db::create_pool()`
- `pool.get()` → direct PostgreSQL client

This architectural gap means:
- **CLI commands don't work with SQLite** - they call PostgreSQL directly
- **Daemon doesn't support SQLite** - hardcoded PostgreSQL pool
- **Indexer bypasses the trait** - direct pool.get() calls
- **No unified database abstraction** - code is database-aware when it should be database-agnostic
- **SQLite doesn't support Ollama embeddings** - hardcoded to 1536-dim only

### Critical: Ollama Embedding Dimension Gap

**Priority**: HIGH - Ollama is the zero-config default embedding provider

| Backend | 768-dim (Ollama) | 1536-dim (OpenAI) |
|---------|------------------|-------------------|
| PostgreSQL | ✅ Supported | ✅ Supported |
| SQLite | ❌ **NOT SUPPORTED** | ✅ Supported |

The SQLite backend is **hardcoded to 1536 dimensions**:

```rust
// crates/maproom/src/db/sqlite/embeddings.rs:38-41
if embedding.len() != 1536 {
    anyhow::bail!(
        "Unsupported embedding dimension: {}. Only 1536-dimensional embeddings are currently supported.",
        embedding.len()
    );
}
```

This breaks the "zero-config" promise:
- SQLite = zero-config database ✅
- Ollama = zero-config embedding provider ✅
- SQLite + Ollama = **FAILS** ❌

**Impact**: Users who want the simplest setup (no PostgreSQL, no API keys) cannot use the system at all.

**Dependency**: The EMBPERF project (Ollama parallel optimization) produces 768-dim embeddings. Without SQLite 768-dim support, EMBPERF benefits are PostgreSQL-only.

## Current State Analysis

### VectorStore Trait (Current Methods)

```rust
// crates/maproom/src/db/mod.rs:91-157
pub trait VectorStore: Send + Sync {
    // Repository & Worktree
    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64>;
    async fn get_or_create_worktree(&self, repo_id: i64, name: &str, abs_path: &str) -> anyhow::Result<i64>;
    async fn get_or_create_commit(&self, repo_id: i64, sha: &str, committed_at: Option<DateTime>) -> anyhow::Result<i64>;

    // Indexing
    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64>;
    async fn insert_chunk(&self, chunk: &ChunkRecord) -> anyhow::Result<i64>;
    async fn insert_chunks_batch(&self, chunks: &[ChunkRecord]) -> anyhow::Result<Vec<i64>>;
    async fn insert_chunk_edge(&self, src_chunk_id: i64, dst_chunk_id: i64, edge_type: &str) -> anyhow::Result<()>;

    // Embeddings
    async fn upsert_embeddings(&self, chunk_id: i64, code_embedding: Option<&[f32]>, text_embedding: Option<&[f32]>, dimension: usize) -> anyhow::Result<()>;
    async fn batch_upsert_embeddings(&self, embeddings: &[(i64, Option<Vec<f32>>, Option<Vec<f32>>)], dimension: usize) -> anyhow::Result<()>;

    // Search (FTS only)
    async fn search_chunks_fts(&self, repo: &str, worktree: Option<&str>, query: &str, k: i64, debug: bool) -> anyhow::Result<Vec<SearchHit>>;

    // Lookup
    async fn find_chunk_by_symbol(&self, repo_id: i64, worktree_id: Option<i64>, symbol_name: &str, relpath: Option<&str>) -> anyhow::Result<Option<i64>>;

    // Migrations
    async fn migrate(&self) -> anyhow::Result<()>;
    async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>>;
}
```

### Direct PostgreSQL Bypasses (Audit)

**main.rs** (12 direct calls):
- Line 386: `db::connect()` for vector search
- Line 522, 532: `db::connect()` for search commands
- Line 681: `db::connect()` for context assembly
- Line 766: `db::create_pool()` for indexer
- Line 853: `db::connect()` for status check
- Line 944: `db::connect()` for cleanup
- Line 1002, 1013, 1033: `db::connect()` for admin commands
- Line 1224, 1266: `db::connect()` for debug commands

**indexer/mod.rs** (9 direct calls):
- Lines 275, 1016, 1506, 1529: `pool.get()` for chunk operations
- Lines 1100, 1615, 1770, 1935: `db::create_pool()` for indexing
- Line 1142, 1633, 1891: `pool.get()` for indexing

**daemon/mod.rs** (1 direct call):
- Line 112: `state.pool.get()` - daemon uses PostgreSQL pool directly

**embedding/pipeline.rs** (2 direct calls):
- Line 558: `db::queries::upsert_embeddings`
- Line 1004: `db::queries::connect()`

### Missing Trait Methods

Based on the bypass audit, the following capabilities need to be added to `VectorStore`:

1. **Vector Search** (missing from trait)
   - `search_chunks_vector(repo, worktree, embedding, k, debug)` → Vec<SearchHit>
   - `search_chunks_hybrid(repo, worktree, query, embedding, k, debug)` → Vec<SearchHit>
   - **Note**: Must support both 768-dim (Ollama) and 1536-dim (OpenAI) embeddings

2. **Context Assembly** (missing from trait)
   - `get_chunk_context(chunk_id)` → ChunkContext
   - `get_file_chunks(file_id)` → Vec<Chunk>
   - `get_chunk_by_id(chunk_id)` → Option<Chunk>

3. **Repository Queries** (missing from trait)
   - `get_repo_by_name(name)` → Option<Repo>
   - `get_worktree_by_name(repo_id, name)` → Option<Worktree>
   - `list_repos()` → Vec<Repo>
   - `list_worktrees(repo_id)` → Vec<Worktree>

4. **Index State** (missing from trait)
   - `get_last_indexed_tree(worktree_id)` → Option<TreeSha>
   - `update_index_state(worktree_id, tree_sha, stats)` → Result<()>

5. **Cleanup** (missing from trait)
   - `detect_stale_worktrees(repo_id)` → Vec<StaleWorktree>
   - `delete_stale_chunks(worktree_id)` → CleanupReport

6. **Incremental** (missing from trait)
   - `get_chunks_by_blob_sha(blob_sha)` → Vec<Chunk>
   - `delete_chunks_by_file(file_id)` → Result<()>

7. **Multi-Dimension Embedding Support** (SQLite missing)
   - SQLite must support 768-dim embeddings (Ollama/nomic-embed-text)
   - SQLite must support 1536-dim embeddings (OpenAI/text-embedding-3-small)
   - Schema changes: separate tables or columns for each dimension
   - Migration path for existing 1536-only databases

## Existing Industry Solutions

### Database Abstraction Patterns

**1. Repository Pattern** (Domain-Driven Design)
- Abstract data access behind interface
- Each entity has its own repository
- Good for complex domains

**2. Active Record** (Rails, Django)
- Objects know how to persist themselves
- Database-aware models
- Simpler but tightly coupled

**3. Data Mapper** (TypeORM, SQLAlchemy)
- Separate data layer from domain
- More complex but cleaner

**4. Trait-Based Abstraction** (Rust idiom)
- Define behavior contract via trait
- Implement per-backend
- What we're already doing—just incomplete

Our approach (trait-based abstraction) is correct for Rust. The issue is incomplete trait coverage, not architectural flaws.

### Similar Projects

**SeaORM** (Rust):
- Async ORM with multiple backend support
- Uses traits for database operations
- Good reference for async trait patterns

**Diesel** (Rust):
- Compile-time checked queries
- Backend traits for PostgreSQL, SQLite, MySQL
- Mature example of multi-backend support

## Gap Analysis

### What Works
- ✅ Basic CRUD operations through trait
- ✅ FTS search through trait
- ✅ Migrations through trait
- ✅ Both backends implement current trait

### What's Missing
- ❌ Vector search not in trait
- ❌ Hybrid search not in trait
- ❌ Context assembly not in trait
- ❌ Repository queries not in trait
- ❌ Index state management not in trait
- ❌ Cleanup operations not in trait
- ❌ **SQLite doesn't support 768-dim (Ollama) embeddings** - CRITICAL
- ❌ CLI uses `db::connect()` instead of trait
- ❌ Daemon uses `pool` instead of `Arc<dyn VectorStore>`
- ❌ Indexer mixes trait and direct calls

### Root Causes
1. **Incremental Development**: SQLite was added after most code was written for PostgreSQL
2. **Missing Planning**: No upfront decision to abstract database access
3. **Convenience**: Direct pool access was simpler during initial development
4. **Hardcoded Dimensions**: SQLite embedding schema was built for OpenAI's 1536-dim only, without considering Ollama's 768-dim

## Research Findings

### SQLite Implementation Review

The SQLite backend (`db/sqlite/`) already has sophisticated implementations:
- **vector.rs**: sqlite-vec integration for vector search
- **fts.rs**: FTS5 with BM25 ranking
- **hybrid.rs**: RRF (Reciprocal Rank Fusion) combining FTS + vector
- **graph.rs**: Recursive CTEs for caller/callee traversal
- **embeddings.rs**: Content-addressed embedding storage

These modules have the implementations—they just aren't wired into the trait.

### PostgreSQL Implementation Review

The PostgreSQL backend (`db/postgres/mod.rs`) wraps `db/queries.rs` functions. Moving to trait-based access requires:
1. Adding methods to `VectorStore` trait
2. Implementing in `PostgresStore` (usually wrapping queries.rs)
3. Implementing in `SqliteStore` (wiring to sqlite/* modules)

### Performance Considerations

- SQLite FTS5 and sqlite-vec are well-optimized
- PostgreSQL has pgvector with HNSW indexes
- Both backends should have similar query patterns
- Connection pooling handled by r2d2 (SQLite) and deadpool (PostgreSQL)

## Recommendations

### Approach: Incremental Trait Expansion

1. **Phase 1**: Add search methods to trait (vector, hybrid)
2. **Phase 2**: Add context methods to trait
3. **Phase 3**: Add repository query methods to trait
4. **Phase 4**: Add index state methods to trait
5. **Phase 5**: Add cleanup methods to trait
6. **Phase 6**: Contract and parity tests

**Note:** CLI/daemon/indexer migration is handled by MAPROOMCLI project after VECSTORE completes.

### Success Criteria

1. `cargo test --features sqlite` passes all new trait tests
2. `cargo test` (PostgreSQL) passes all trait tests
3. No raw SQL outside `db/postgres/` or `db/sqlite/`
4. `get_store()` returns working store for both backends
5. All new trait methods implemented in both `PostgresStore` and `SqliteStore`

### Out of Scope

- **CLI migration to trait** (MAPROOMCLI project) - CLI commands using `Arc<dyn VectorStore>`
- **Daemon migration to trait** (MAPROOMCLI project) - Daemon using `Arc<dyn VectorStore>`
- **Indexer migration to trait** (MAPROOMCLI project) - Indexer using trait instead of pool
- CLI command flags for backend selection (MAPROOMCLI project)
- TypeScript changes (MCPDB project)
- VSCode extension changes (VSCODEDB project)
- CI/CD changes (SQLITEINFRA project)

**Note:** VECSTORE completes when the trait is expanded and both stores implement it. Consumer migration (CLI/daemon/indexer using the trait) is a separate project (MAPROOMCLI).
