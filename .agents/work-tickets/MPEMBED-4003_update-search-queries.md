# Ticket: MPEMBED-4003: Update search queries for mixed embeddings

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vector-database-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Update hybrid and vector search queries to handle mixed embeddings using COALESCE pattern. Prefer 768-dim (*_ollama) columns over 1536-dim (original) columns. Vector search selects columns based on query embedding dimension.

## Background
This ticket implements Phase 4 (Database and Search Integration) from the MPEMBED multi-provider embeddings plan. The database now contains chunks with embeddings in different columns: some in *_ollama (768-dim from Ollama/Google), some in original columns (1536-dim from OpenAI), and some in both. Search queries must intelligently handle this mixed state while maintaining backward compatibility with existing OpenAI-only indexes.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-4-database-search-integration.md

## Acceptance Criteria
- [ ] Hybrid search uses COALESCE(code_embedding_ollama, code_embedding) pattern
- [ ] Preference order: 768-dim > 1536-dim (prefers Ollama columns)
- [ ] Vector search selects columns based on query embedding dimension
- [ ] Full-text search (FTS) component unchanged
- [ ] Search returns results from both embedding types
- [ ] Cosine similarity calculation works with both dimensions
- [ ] Performance regression < 5% vs baseline (MPEMBED-0002)
- [ ] Unit tests for COALESCE logic
- [ ] Integration tests with mixed embeddings

## Technical Requirements
- Modify hybrid search query to use COALESCE for embedding columns
- Modify vector-only search to select columns by query dimension
- Ensure pgvector operations handle NULL gracefully
- Maintain existing similarity scoring formulas
- Preserve search ranking quality
- Support queries with both 768 and 1536 dimensions
- Add SQL query explain plans to verify index usage
- Document COALESCE preference rationale

## Implementation Notes
**Current Hybrid Search (to be modified):**
```sql
-- crates/maproom/src/search/hybrid.rs
SELECT
  c.id,
  c.relpath,
  c.start_line,
  c.end_line,
  c.content,
  -- FTS score
  ts_rank(c.fts_content, websearch_to_tsquery('english', $1)) AS fts_score,
  -- Vector similarity (cosine distance)
  1 - (c.code_embedding <=> $2::vector(1536)) AS vector_score,
  -- Combined hybrid score
  (
    0.5 * ts_rank(c.fts_content, websearch_to_tsquery('english', $1)) +
    0.5 * (1 - (c.code_embedding <=> $2::vector(1536)))
  ) AS hybrid_score
FROM chunks c
WHERE
  c.fts_content @@ websearch_to_tsquery('english', $1)
  OR c.code_embedding IS NOT NULL
ORDER BY hybrid_score DESC
LIMIT $3;
```

**Updated Hybrid Search with COALESCE:**
```sql
-- Hybrid search supporting mixed embeddings
SELECT
  c.id,
  c.relpath,
  c.start_line,
  c.end_line,
  c.content,
  c.language,
  -- FTS score (unchanged)
  ts_rank(c.fts_content, websearch_to_tsquery('english', $1)) AS fts_score,
  -- Vector similarity with COALESCE preference (768 > 1536)
  CASE
    WHEN COALESCE(c.code_embedding_ollama, c.code_embedding) IS NOT NULL THEN
      1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $2)
    ELSE 0
  END AS vector_score,
  -- Indicator of which embedding was used
  CASE
    WHEN c.code_embedding_ollama IS NOT NULL THEN '768'
    WHEN c.code_embedding IS NOT NULL THEN '1536'
    ELSE NULL
  END AS embedding_dimension,
  -- Combined hybrid score (weighted average)
  (
    $3 * ts_rank(c.fts_content, websearch_to_tsquery('english', $1)) +
    $4 * CASE
      WHEN COALESCE(c.code_embedding_ollama, c.code_embedding) IS NOT NULL THEN
        1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $2)
      ELSE 0
    END
  ) AS hybrid_score
FROM chunks c
WHERE
  c.repo = $5
  AND c.worktree = $6
  AND (
    c.fts_content @@ websearch_to_tsquery('english', $1)
    OR c.code_embedding_ollama IS NOT NULL
    OR c.code_embedding IS NOT NULL
  )
ORDER BY hybrid_score DESC
LIMIT $7;
```

**Implementation in Rust:**
```rust
// crates/maproom/src/search/hybrid.rs
use crate::db::columns::select_columns_for_dimension;

pub struct HybridSearchResult {
    pub chunk_id: Uuid,
    pub relpath: String,
    pub start_line: i32,
    pub end_line: i32,
    pub content: String,
    pub language: String,
    pub fts_score: f64,
    pub vector_score: f64,
    pub hybrid_score: f64,
    pub embedding_dimension: Option<String>, // "768" or "1536"
}

/// Hybrid search with multi-dimension embedding support
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `query_text` - Full-text search query
/// * `query_embedding` - Query embedding vector (768 or 1536 dim)
/// * `repo` - Repository name
/// * `worktree` - Worktree name
/// * `limit` - Maximum results to return
/// * `fts_weight` - Weight for FTS score (0.0-1.0)
/// * `vector_weight` - Weight for vector score (0.0-1.0)
///
/// # Notes
/// * COALESCE prefers 768-dim embeddings over 1536-dim
/// * Query embedding dimension determines vector type for comparison
pub async fn hybrid_search(
    pool: &PgPool,
    query_text: &str,
    query_embedding: Vec<f32>,
    repo: &str,
    worktree: &str,
    limit: i64,
    fts_weight: f64,
    vector_weight: f64,
) -> Result<Vec<HybridSearchResult>> {
    let dimension = query_embedding.len();

    // Build query based on query embedding dimension
    let query_str = format!(
        r#"
        SELECT
          c.id,
          c.relpath,
          c.start_line,
          c.end_line,
          c.content,
          c.language,
          ts_rank(c.fts_content, websearch_to_tsquery('english', $1)) AS fts_score,
          CASE
            WHEN COALESCE(c.code_embedding_ollama, c.code_embedding) IS NOT NULL THEN
              1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $2::vector({}))
            ELSE 0
          END AS vector_score,
          CASE
            WHEN c.code_embedding_ollama IS NOT NULL THEN '768'
            WHEN c.code_embedding IS NOT NULL THEN '1536'
            ELSE NULL
          END AS embedding_dimension,
          (
            $3 * ts_rank(c.fts_content, websearch_to_tsquery('english', $1)) +
            $4 * CASE
              WHEN COALESCE(c.code_embedding_ollama, c.code_embedding) IS NOT NULL THEN
                1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $2::vector({}))
              ELSE 0
            END
          ) AS hybrid_score
        FROM chunks c
        WHERE
          c.repo = $5
          AND c.worktree = $6
          AND (
            c.fts_content @@ websearch_to_tsquery('english', $1)
            OR c.code_embedding_ollama IS NOT NULL
            OR c.code_embedding IS NOT NULL
          )
        ORDER BY hybrid_score DESC
        LIMIT $7
        "#,
        dimension, dimension
    );

    let rows = sqlx::query_as::<_, HybridSearchResult>(&query_str)
        .bind(query_text)
        .bind(&query_embedding)
        .bind(fts_weight)
        .bind(vector_weight)
        .bind(repo)
        .bind(worktree)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Hybrid search query failed")?;

    Ok(rows)
}

/// Vector-only search (no FTS component)
pub async fn vector_search(
    pool: &PgPool,
    query_embedding: Vec<f32>,
    repo: &str,
    worktree: &str,
    limit: i64,
) -> Result<Vec<HybridSearchResult>> {
    let dimension = query_embedding.len();

    let query_str = format!(
        r#"
        SELECT
          c.id,
          c.relpath,
          c.start_line,
          c.end_line,
          c.content,
          c.language,
          0.0 AS fts_score,
          1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $1::vector({})) AS vector_score,
          CASE
            WHEN c.code_embedding_ollama IS NOT NULL THEN '768'
            WHEN c.code_embedding IS NOT NULL THEN '1536'
            ELSE NULL
          END AS embedding_dimension,
          1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $1::vector({})) AS hybrid_score
        FROM chunks c
        WHERE
          c.repo = $2
          AND c.worktree = $3
          AND (c.code_embedding_ollama IS NOT NULL OR c.code_embedding IS NOT NULL)
        ORDER BY hybrid_score DESC
        LIMIT $4
        "#,
        dimension, dimension
    );

    let rows = sqlx::query_as::<_, HybridSearchResult>(&query_str)
        .bind(&query_embedding)
        .bind(repo)
        .bind(worktree)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Vector search query failed")?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_hybrid_search_with_mixed_embeddings(pool: PgPool) -> Result<()> {
        // Create chunks with different embedding types
        let chunk_768 = insert_test_chunk(&pool, "768-dim chunk", Some(vec![0.1; 768]), None).await?;
        let chunk_1536 = insert_test_chunk(&pool, "1536-dim chunk", None, Some(vec![0.2; 1536])).await?;
        let chunk_both = insert_test_chunk(&pool, "both dims", Some(vec![0.3; 768]), Some(vec![0.4; 1536])).await?;

        // Search with 768-dim query
        let query_emb = vec![0.15; 768];
        let results = hybrid_search(&pool, "chunk", query_emb, "test", "main", 10, 0.5, 0.5).await?;

        assert!(results.len() >= 2); // Should find 768-dim and both
        // Verify chunk_both uses 768-dim embedding (COALESCE preference)
        let both_result = results.iter().find(|r| r.chunk_id == chunk_both).unwrap();
        assert_eq!(both_result.embedding_dimension, Some("768".to_string()));

        Ok(())
    }

    #[sqlx::test]
    async fn test_coalesce_preference_order(pool: PgPool) -> Result<()> {
        // Create chunk with both embeddings
        let chunk_id = insert_test_chunk(&pool, "test", Some(vec![0.1; 768]), Some(vec![0.2; 1536])).await?;

        let query_emb = vec![0.15; 768];
        let results = vector_search(&pool, query_emb, "test", "main", 1).await?;

        // Should prefer 768-dim (ollama) over 1536-dim (openai)
        assert_eq!(results[0].embedding_dimension, Some("768".to_string()));

        Ok(())
    }
}
```

**Index Considerations:**
```sql
-- Ensure indexes exist for both embedding columns
CREATE INDEX IF NOT EXISTS idx_chunks_code_embedding_ollama ON chunks USING ivfflat (code_embedding_ollama vector_cosine_ops);
CREATE INDEX IF NOT EXISTS idx_chunks_code_embedding ON chunks USING ivfflat (code_embedding vector_cosine_ops);

-- Note: COALESCE expressions cannot use indexes directly
-- PostgreSQL will use index on first non-NULL column in COALESCE
-- Since we prefer ollama columns, they should be indexed first
```

## Dependencies
- MPEMBED-4001 (Column selection logic)

## Risk Assessment
- **Risk**: COALESCE may prevent index usage, degrading performance
  - **Mitigation**: Benchmark with EXPLAIN ANALYZE, consider separate queries if performance degrades
- **Risk**: Comparing 768 and 1536 embeddings may produce inconsistent similarity scores
  - **Mitigation**: Acceptable as they represent different semantic spaces; document behavior
- **Risk**: Preference for 768-dim may bias search results
  - **Mitigation**: Preference only applies when chunk has both; most chunks will have one or the other

## Files/Packages Affected
- crates/maproom/src/search/hybrid.rs (modify - update hybrid_search and vector_search)
- crates/maproom/src/search/mod.rs (modify - update exports)
- crates/maproom/tests/search/mixed_embeddings_test.rs (create)
