# Ticket: MPEMBED-4002: Update embedding upsert for multi-dimension support

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Modify upsert_embeddings() function to accept a dimension parameter and dynamically select database columns using the column selection logic from MPEMBED-4001. Ensure parameterized queries for SQL injection safety.

## Background
This ticket extends the embedding upsert functionality to support multiple embedding dimensions by dynamically choosing columns based on the provider's dimension. The upsert function currently hard-codes column names (code_embedding, doc_embedding), which needs to be abstracted to support both 768-dim and 1536-dim embeddings.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-4-database-search-integration.md

## Acceptance Criteria
- [ ] upsert_embeddings() accepts dimension parameter
- [ ] Function uses select_columns_for_dimension() for column selection
- [ ] SQL queries are parameterized (no string interpolation of values)
- [ ] Supports upserting 768-dim embeddings to *_ollama columns
- [ ] Supports upserting 1536-dim embeddings to original columns
- [ ] Transaction safety maintained
- [ ] Error handling for dimension mismatches
- [ ] Unit tests for both 768 and 1536 dimensions
- [ ] Integration test with actual database

## Technical Requirements
- Modify function signature to include dimension: usize parameter
- Use select_columns_for_dimension() from MPEMBED-4001
- Build SQL query strings with column names (compile-time safe)
- Use parameterized queries ($1, $2, etc.) for all values
- Validate embedding vector length matches dimension parameter
- Maintain existing batch upsert performance characteristics
- Use sqlx::query! macro where possible for compile-time SQL validation
- Handle NULL values correctly (existing embeddings may be NULL)

## Implementation Notes
**Current Implementation (to be modified):**
```rust
// crates/maproom/src/db/chunks.rs
pub async fn upsert_embeddings(
    pool: &PgPool,
    chunk_id: Uuid,
    code_embedding: Option<Vec<f32>>,
    doc_embedding: Option<Vec<f32>>,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE chunks
        SET code_embedding = $1,
            doc_embedding = $2,
            updated_at = NOW()
        WHERE id = $3
        "#,
        code_embedding.as_deref(),
        doc_embedding.as_deref(),
        chunk_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

**Updated Implementation:**
```rust
// crates/maproom/src/db/chunks.rs
use crate::db::columns::select_columns_for_dimension;

/// Upsert embeddings for a chunk, selecting columns based on dimension
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `chunk_id` - Chunk UUID to update
/// * `code_embedding` - Optional code embedding vector
/// * `doc_embedding` - Optional doc embedding vector
/// * `dimension` - Embedding dimension (768 or 1536)
///
/// # Errors
/// * Returns error if dimension is unsupported
/// * Returns error if embedding length doesn't match dimension
/// * Returns error if database update fails
pub async fn upsert_embeddings(
    pool: &PgPool,
    chunk_id: Uuid,
    code_embedding: Option<Vec<f32>>,
    doc_embedding: Option<Vec<f32>>,
    dimension: usize,
) -> Result<()> {
    // Validate embedding dimensions
    if let Some(ref vec) = code_embedding {
        if vec.len() != dimension {
            anyhow::bail!(
                "Code embedding length {} does not match dimension {}",
                vec.len(),
                dimension
            );
        }
    }
    if let Some(ref vec) = doc_embedding {
        if vec.len() != dimension {
            anyhow::bail!(
                "Doc embedding length {} does not match dimension {}",
                vec.len(),
                dimension
            );
        }
    }

    // Select columns based on dimension
    let columns = select_columns_for_dimension(dimension)?;

    // Build SQL query with dynamic column names
    // Note: Column names are from constants, not user input, so safe from injection
    let query_str = format!(
        r#"
        UPDATE chunks
        SET {} = $1,
            {} = $2,
            updated_at = NOW()
        WHERE id = $3
        "#,
        columns.code_embedding,
        columns.doc_embedding
    );

    sqlx::query(&query_str)
        .bind(code_embedding.as_deref())
        .bind(doc_embedding.as_deref())
        .bind(chunk_id)
        .execute(pool)
        .await
        .context("Failed to upsert embeddings")?;

    Ok(())
}

/// Batch upsert embeddings for multiple chunks
pub async fn batch_upsert_embeddings(
    pool: &PgPool,
    embeddings: Vec<(Uuid, Option<Vec<f32>>, Option<Vec<f32>>)>,
    dimension: usize,
) -> Result<()> {
    let columns = select_columns_for_dimension(dimension)?;

    // Use transaction for batch operation
    let mut tx = pool.begin().await?;

    let query_str = format!(
        r#"
        UPDATE chunks
        SET {} = $1,
            {} = $2,
            updated_at = NOW()
        WHERE id = $3
        "#,
        columns.code_embedding,
        columns.doc_embedding
    );

    for (chunk_id, code_emb, doc_emb) in embeddings {
        // Validate dimensions
        if let Some(ref vec) = code_emb {
            if vec.len() != dimension {
                anyhow::bail!("Code embedding dimension mismatch for chunk {}", chunk_id);
            }
        }
        if let Some(ref vec) = doc_emb {
            if vec.len() != dimension {
                anyhow::bail!("Doc embedding dimension mismatch for chunk {}", chunk_id);
            }
        }

        sqlx::query(&query_str)
            .bind(code_emb.as_deref())
            .bind(doc_emb.as_deref())
            .bind(chunk_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_upsert_768_dimension(pool: PgPool) -> Result<()> {
        // Create test chunk
        let chunk_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO chunks (id, repo, worktree, relpath, start_line, end_line, content, language) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            chunk_id, "test", "main", "test.rs", 1, 10, "test content", "rust"
        )
        .execute(&pool)
        .await?;

        // Create 768-dim embeddings
        let code_emb = vec![0.1f32; 768];
        let doc_emb = vec![0.2f32; 768];

        // Upsert
        upsert_embeddings(&pool, chunk_id, Some(code_emb.clone()), Some(doc_emb.clone()), 768).await?;

        // Verify stored in ollama columns
        let row = sqlx::query!(
            "SELECT code_embedding_ollama, doc_embedding_ollama FROM chunks WHERE id = $1",
            chunk_id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.code_embedding_ollama.unwrap().len(), 768);
        assert_eq!(row.doc_embedding_ollama.unwrap().len(), 768);

        Ok(())
    }

    #[sqlx::test]
    async fn test_upsert_1536_dimension(pool: PgPool) -> Result<()> {
        // Create test chunk
        let chunk_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO chunks (id, repo, worktree, relpath, start_line, end_line, content, language) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            chunk_id, "test", "main", "test.rs", 1, 10, "test content", "rust"
        )
        .execute(&pool)
        .await?;

        // Create 1536-dim embeddings
        let code_emb = vec![0.1f32; 1536];
        let doc_emb = vec![0.2f32; 1536];

        // Upsert
        upsert_embeddings(&pool, chunk_id, Some(code_emb.clone()), Some(doc_emb.clone()), 1536).await?;

        // Verify stored in original columns
        let row = sqlx::query!(
            "SELECT code_embedding, doc_embedding FROM chunks WHERE id = $1",
            chunk_id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.code_embedding.unwrap().len(), 1536);
        assert_eq!(row.doc_embedding.unwrap().len(), 1536);

        Ok(())
    }

    #[sqlx::test]
    async fn test_dimension_mismatch_error(pool: PgPool) -> Result<()> {
        let chunk_id = Uuid::new_v4();
        let code_emb = vec![0.1f32; 1536]; // Wrong size for 768

        let result = upsert_embeddings(&pool, chunk_id, Some(code_emb), None, 768).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not match dimension"));

        Ok(())
    }
}
```

**Migration Guide for Existing Code:**
All call sites must be updated to pass dimension parameter:
```rust
// Before
upsert_embeddings(&pool, chunk_id, Some(code_vec), Some(doc_vec)).await?;

// After
let dimension = provider.dimension();
upsert_embeddings(&pool, chunk_id, Some(code_vec), Some(doc_vec), dimension).await?;
```

## Dependencies
- MPEMBED-4001 (Column selection logic must exist)

## Risk Assessment
- **Risk**: Dynamic SQL query construction may be vulnerable to SQL injection
  - **Mitigation**: Column names come from compile-time constants (ColumnSet), all values use parameterized queries ($1, $2, etc.)
- **Risk**: Existing code may break due to signature change
  - **Mitigation**: Compiler will catch all call sites, update all in single commit
- **Risk**: Dimension validation overhead may impact performance
  - **Mitigation**: Validation is O(1) comparison, negligible impact; benchmark before/after
- **Risk**: Transaction handling in batch upsert may cause deadlocks
  - **Mitigation**: Keep transactions short, use same pattern as existing batch operations

## Files/Packages Affected
- crates/maproom/src/db/chunks.rs (modify - update upsert_embeddings and batch_upsert_embeddings)
- crates/maproom/src/embedding/pipeline.rs (modify - pass dimension to upsert calls)
- crates/maproom/tests/db/upsert_test.rs (create - integration tests)
