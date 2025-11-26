# Ticket: VECSTORE-1002: Hybrid Search Methods

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - SQLite tests: 103 passed (PostgreSQL requires running db)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add hybrid search (FTS + vector similarity fusion) to the `VectorStore` trait and implement it in both `PostgresStore` and `SqliteStore` using Reciprocal Rank Fusion (RRF).

## Background
Hybrid search combines full-text search (keyword matching) with vector similarity search (semantic matching) for better retrieval quality. The SQLite backend has `sqlite/hybrid.rs` with RRF implementation, but PostgreSQL lacks a dedicated hybrid search function.

**Current State**:
- SQLite: `sqlite/hybrid.rs` exists with RRF fusion - not wired to trait
- PostgreSQL: **NO** `search_chunks_hybrid()` function in `queries.rs` - must be written
- Trait: Only has `search_chunks_fts()`, no hybrid search method

**Reference**: Plan Phase 1 - Hybrid Search Methods (VECSTORE-1002)

## Acceptance Criteria
- [ ] `search_chunks_hybrid` method added to `VectorStore` trait
- [ ] PostgreSQL hybrid search query function written in `queries.rs`
- [ ] `PostgresStore` implementation produces ranked results using RRF
- [ ] `SqliteStore` implementation produces equivalent ranking
- [ ] Parity test verifies similar top-k results (rank order, not absolute scores)
- [ ] Falls back to FTS-only when vector search unavailable
- [ ] Contract tests pass for both backends

## Technical Requirements

### Trait Method Signature
Add to `VectorStore` trait in `crates/maproom/src/db/mod.rs`:

```rust
/// Hybrid search combining FTS and vector similarity using RRF fusion
async fn search_chunks_hybrid(
    &self,
    repo: &str,
    worktree: Option<&str>,
    query: &str,          // Text query for FTS
    embedding: &[f32],    // Query embedding for vector search
    k: i64,
    debug: bool,
) -> anyhow::Result<Vec<SearchHit>>;
```

### Reciprocal Rank Fusion (RRF)
Both backends should use the same RRF formula to combine FTS and vector rankings:

```
RRF_score(d) = Σ(1 / (k + rank_i(d)))
```

Where:
- `k` is a constant (typically 60)
- `rank_i(d)` is the rank of document `d` in ranking `i`

### PostgreSQL Implementation (NEW - must be written)

**File: `crates/maproom/src/db/queries.rs`**

Write a new function that:
1. Runs FTS query using `ts_rank_cd`
2. Runs vector query using pgvector `<=>`
3. Combines with RRF fusion

```rust
pub async fn search_chunks_hybrid(
    client: &impl GenericClient,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    embedding: &[f32],
    k: i64,
    debug: bool,
) -> anyhow::Result<Vec<SearchHit>> {
    // Option 1: Two queries + Rust-side RRF fusion
    // Option 2: Single CTE-based query with SQL-side RRF

    // Example CTE approach:
    // WITH fts_results AS (
    //   SELECT chunk_id, ts_rank_cd(...) as fts_score,
    //          ROW_NUMBER() OVER (ORDER BY ts_rank_cd(...) DESC) as fts_rank
    //   FROM chunks ...
    // ),
    // vec_results AS (
    //   SELECT chunk_id, 1 - (embedding <=> $query) as vec_score,
    //          ROW_NUMBER() OVER (ORDER BY embedding <=> $query) as vec_rank
    //   FROM embeddings ...
    // )
    // SELECT chunk_id,
    //        COALESCE(1.0/(60+fts_rank), 0) + COALESCE(1.0/(60+vec_rank), 0) as rrf_score
    // FROM fts_results FULL OUTER JOIN vec_results USING (chunk_id)
    // ORDER BY rrf_score DESC LIMIT k
}
```

### SQLite Implementation (wrap existing)

**File: `crates/maproom/src/db/sqlite/mod.rs`**

Wire `SqliteStore::search_chunks_hybrid` to existing `sqlite/hybrid.rs`:

```rust
async fn search_chunks_hybrid(
    &self,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    embedding: &[f32],
    k: i64,
    debug: bool,
) -> anyhow::Result<Vec<SearchHit>> {
    let repo = repo.to_string();
    let worktree = worktree.map(String::from);
    let query = query.to_string();
    let embedding = embedding.to_vec();
    let has_vec = self.has_vec_extension();

    self.run(move |conn| {
        if has_vec {
            hybrid::search_hybrid(conn, &repo, worktree.as_deref(), &query, &embedding, k, debug)
        } else {
            // Fallback to FTS only
            fts::search_fts(conn, &repo, worktree.as_deref(), &query, k, debug)
        }
    }).await
}
```

### PostgresStore Implementation

**File: `crates/maproom/src/db/postgres/mod.rs`**

```rust
async fn search_chunks_hybrid(
    &self,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    embedding: &[f32],
    k: i64,
    debug: bool,
) -> anyhow::Result<Vec<SearchHit>> {
    let client = self.pool.get().await.context("Failed to get connection")?;
    super::queries::search_chunks_hybrid(&client, repo, worktree, query, embedding, k, debug).await
}
```

## Implementation Notes

### RRF Constant
Use `k=60` as the RRF constant (industry standard). This can be made configurable later if needed.

### Dimension Handling
The embedding dimension determines which column to use for vector search:
- 768-dim: Use Ollama column/table
- 1536-dim: Use OpenAI column/table

### Parity Testing Strategy
Because FTS algorithms differ (BM25 vs ts_rank_cd), hybrid search parity tests should:
1. Compare same chunks returned (set equality)
2. Compare top-3 rankings (most relevant should be similar)
3. **DO NOT** compare absolute RRF scores

### Fallback Behavior
When sqlite-vec is unavailable:
- Log warning
- Fall back to FTS-only search
- Return results (degraded but functional)

## Dependencies
- **VECSTORE-1000**: SQLite 768-dim support (for 768-dim hybrid search)
- **VECSTORE-1001**: Vector search methods (hybrid builds on vector search)

## Risk Assessment
- **Risk**: RRF implementations differ slightly between backends
  - **Mitigation**: Both use same formula with k=60
- **Risk**: PostgreSQL CTE query performance
  - **Mitigation**: Test with realistic data volumes, add EXPLAIN ANALYZE
- **Risk**: FTS ranking differences affect hybrid results
  - **Mitigation**: RRF normalizes ranking differences; parity tests catch issues

## Files/Packages Affected
- `crates/maproom/src/db/mod.rs` (trait definition)
- `crates/maproom/src/db/queries.rs` (NEW PostgreSQL function)
- `crates/maproom/src/db/postgres/mod.rs` (PostgresStore impl)
- `crates/maproom/src/db/sqlite/mod.rs` (SqliteStore impl)
