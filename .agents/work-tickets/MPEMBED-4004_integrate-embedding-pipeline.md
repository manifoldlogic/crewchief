# Ticket: MPEMBED-4004: Integrate provider dimension with embedding pipeline

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- embeddings-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the embedding pipeline to query provider.dimension() and pass it to upsert_embeddings(). Update batch processing to handle dimension parameter correctly for incremental embedding generation.

## Background
This ticket completes Phase 4 (Database and Search Integration) by connecting the provider abstraction (Phase 2) with the database layer (MPEMBED-4002). The embedding pipeline must extract the dimension from the provider and pass it through to the database upsert operations, enabling dimension-aware persistence.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-4-database-search-integration.md

## Acceptance Criteria
- [ ] Pipeline queries provider.dimension() once during initialization
- [ ] Dimension passed to all upsert_embeddings() calls
- [ ] Batch processing includes dimension parameter
- [ ] Incremental embedding generation uses correct columns
- [ ] Statistics output includes dimension information
- [ ] Error handling for provider/dimension mismatches
- [ ] Integration test: full scan with Ollama (768-dim)
- [ ] Integration test: full scan with OpenAI (1536-dim)
- [ ] Integration test: incremental update preserves dimension

## Technical Requirements
- Modify EmbeddingPipeline to store provider dimension
- Pass dimension to single and batch upsert operations
- Update progress/statistics reporting to include dimension
- Maintain backward compatibility with existing embeddings
- Handle migration scenario: existing 1536-dim + new 768-dim chunks
- Validate embedding vectors match provider dimension before upsert
- Add telemetry/logging for dimension tracking
- Performance: dimension query should not be repeated per chunk

## Implementation Notes
**Current Pipeline (to be modified):**
```rust
// crates/maproom/src/embedding/pipeline.rs
pub struct EmbeddingPipeline {
    provider: Arc<dyn EmbeddingProvider>,
    pool: PgPool,
    batch_size: usize,
}

impl EmbeddingPipeline {
    pub fn new(provider: Arc<dyn EmbeddingProvider>, pool: PgPool) -> Self {
        Self {
            provider,
            pool,
            batch_size: 100,
        }
    }

    pub async fn process_chunks(&self, chunk_ids: Vec<Uuid>) -> Result<EmbeddingStats> {
        let mut stats = EmbeddingStats::default();

        for batch in chunk_ids.chunks(self.batch_size) {
            let chunks = self.fetch_chunks(batch).await?;
            let embeddings = self.generate_embeddings(&chunks).await?;
            self.upsert_embeddings(chunks, embeddings).await?;
            stats.chunks_processed += batch.len();
        }

        Ok(stats)
    }

    async fn upsert_embeddings(
        &self,
        chunks: Vec<Chunk>,
        embeddings: Vec<(Vec<f32>, Vec<f32>)>,
    ) -> Result<()> {
        for (chunk, (code_emb, doc_emb)) in chunks.iter().zip(embeddings) {
            upsert_embeddings(
                &self.pool,
                chunk.id,
                Some(code_emb),
                Some(doc_emb),
            ).await?;
        }
        Ok(())
    }
}
```

**Updated Pipeline with Dimension Support:**
```rust
// crates/maproom/src/embedding/pipeline.rs
use crate::db::chunks::upsert_embeddings;

pub struct EmbeddingPipeline {
    provider: Arc<dyn EmbeddingProvider>,
    pool: PgPool,
    batch_size: usize,
    dimension: usize, // Cache provider dimension
}

impl EmbeddingPipeline {
    pub fn new(provider: Arc<dyn EmbeddingProvider>, pool: PgPool) -> Self {
        let dimension = provider.dimension(); // Query once
        tracing::info!(
            "Initialized embedding pipeline: provider={}, dimension={}",
            provider.name(),
            dimension
        );

        Self {
            provider,
            pool,
            batch_size: 100,
            dimension,
        }
    }

    /// Get the embedding dimension for this pipeline
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Process chunks and generate embeddings
    pub async fn process_chunks(&self, chunk_ids: Vec<Uuid>) -> Result<EmbeddingStats> {
        let mut stats = EmbeddingStats {
            chunks_processed: 0,
            chunks_failed: 0,
            dimension: self.dimension,
            provider: self.provider.name().to_string(),
            ..Default::default()
        };

        for batch in chunk_ids.chunks(self.batch_size) {
            match self.process_batch(batch).await {
                Ok(batch_stats) => {
                    stats.chunks_processed += batch_stats.chunks_processed;
                    stats.total_tokens += batch_stats.total_tokens;
                }
                Err(e) => {
                    tracing::error!("Batch processing failed: {}", e);
                    stats.chunks_failed += batch.len();
                }
            }
        }

        Ok(stats)
    }

    async fn process_batch(&self, chunk_ids: &[Uuid]) -> Result<EmbeddingStats> {
        let chunks = self.fetch_chunks(chunk_ids).await?;
        let embeddings = self.generate_embeddings(&chunks).await?;

        // Validate embedding dimensions before upsert
        for (i, (code_emb, doc_emb)) in embeddings.iter().enumerate() {
            if code_emb.len() != self.dimension {
                anyhow::bail!(
                    "Code embedding dimension mismatch: expected {}, got {} for chunk {}",
                    self.dimension,
                    code_emb.len(),
                    chunks[i].id
                );
            }
            if doc_emb.len() != self.dimension {
                anyhow::bail!(
                    "Doc embedding dimension mismatch: expected {}, got {} for chunk {}",
                    self.dimension,
                    doc_emb.len(),
                    chunks[i].id
                );
            }
        }

        self.upsert_embeddings(chunks, embeddings).await?;

        Ok(EmbeddingStats {
            chunks_processed: chunk_ids.len(),
            total_tokens: chunks.iter().map(|c| c.content.len()).sum(),
            dimension: self.dimension,
            provider: self.provider.name().to_string(),
            ..Default::default()
        })
    }

    async fn upsert_embeddings(
        &self,
        chunks: Vec<Chunk>,
        embeddings: Vec<(Vec<f32>, Vec<f32>)>,
    ) -> Result<()> {
        for (chunk, (code_emb, doc_emb)) in chunks.iter().zip(embeddings) {
            upsert_embeddings(
                &self.pool,
                chunk.id,
                Some(code_emb),
                Some(doc_emb),
                self.dimension, // Pass dimension to upsert
            ).await.context(format!("Failed to upsert embeddings for chunk {}", chunk.id))?;
        }
        Ok(())
    }

    /// Process only chunks missing embeddings (incremental mode)
    pub async fn process_missing_embeddings(&self, repo: &str, worktree: &str) -> Result<EmbeddingStats> {
        let columns = select_columns_for_dimension(self.dimension)?;

        // Query chunks missing embeddings for this dimension
        let query = format!(
            r#"
            SELECT id
            FROM chunks
            WHERE repo = $1
              AND worktree = $2
              AND ({} IS NULL OR {} IS NULL)
            "#,
            columns.code_embedding,
            columns.doc_embedding
        );

        let chunk_ids: Vec<Uuid> = sqlx::query_scalar(&query)
            .bind(repo)
            .bind(worktree)
            .fetch_all(&self.pool)
            .await?;

        tracing::info!(
            "Found {} chunks missing {}-dimensional embeddings",
            chunk_ids.len(),
            self.dimension
        );

        self.process_chunks(chunk_ids).await
    }
}

#[derive(Debug, Default)]
pub struct EmbeddingStats {
    pub chunks_processed: usize,
    pub chunks_failed: usize,
    pub total_tokens: usize,
    pub dimension: usize,
    pub provider: String,
    pub duration_secs: f64,
}

impl std::fmt::Display for EmbeddingStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Embedding Stats:\n\
             Provider: {} ({} dimensions)\n\
             Chunks processed: {}\n\
             Chunks failed: {}\n\
             Total tokens: {}\n\
             Duration: {:.2}s\n\
             Throughput: {:.1} chunks/sec",
            self.provider,
            self.dimension,
            self.chunks_processed,
            self.chunks_failed,
            self.total_tokens,
            self.duration_secs,
            if self.duration_secs > 0.0 {
                self.chunks_processed as f64 / self.duration_secs
            } else {
                0.0
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::factory::create_provider;

    #[tokio::test]
    async fn test_pipeline_dimension_caching() {
        let provider = create_provider("ollama").unwrap();
        let pool = create_test_pool().await;

        let pipeline = EmbeddingPipeline::new(provider.clone(), pool);
        assert_eq!(pipeline.dimension(), 768);
        assert_eq!(pipeline.dimension(), provider.dimension()); // Should match
    }

    #[sqlx::test]
    async fn test_process_chunks_with_dimension(pool: PgPool) -> Result<()> {
        // Set up Ollama provider (768-dim)
        std::env::set_var("EMBEDDING_PROVIDER", "ollama");
        let provider = create_provider("ollama")?;
        let pipeline = EmbeddingPipeline::new(provider, pool.clone());

        // Create test chunks
        let chunk_ids = vec![create_test_chunk(&pool).await?];

        // Process
        let stats = pipeline.process_chunks(chunk_ids.clone()).await?;

        assert_eq!(stats.chunks_processed, 1);
        assert_eq!(stats.dimension, 768);
        assert_eq!(stats.provider, "ollama");

        // Verify embeddings in correct columns
        let row = sqlx::query!(
            "SELECT code_embedding_ollama, doc_embedding_ollama FROM chunks WHERE id = $1",
            chunk_ids[0]
        )
        .fetch_one(&pool)
        .await?;

        assert!(row.code_embedding_ollama.is_some());
        assert_eq!(row.code_embedding_ollama.unwrap().len(), 768);

        Ok(())
    }

    #[sqlx::test]
    async fn test_incremental_embedding_by_dimension(pool: PgPool) -> Result<()> {
        // Create chunk with only 1536-dim embeddings
        let chunk_id = create_test_chunk(&pool).await?;
        sqlx::query!(
            "UPDATE chunks SET code_embedding = $1 WHERE id = $2",
            vec![0.1f32; 1536].as_slice(),
            chunk_id
        )
        .execute(&pool)
        .await?;

        // Run pipeline with Ollama (768-dim)
        std::env::set_var("EMBEDDING_PROVIDER", "ollama");
        let provider = create_provider("ollama")?;
        let pipeline = EmbeddingPipeline::new(provider, pool.clone());

        let stats = pipeline.process_missing_embeddings("test", "main").await?;

        // Should process chunk (missing 768-dim embeddings)
        assert_eq!(stats.chunks_processed, 1);

        // Verify both embeddings now present
        let row = sqlx::query!(
            "SELECT code_embedding, code_embedding_ollama FROM chunks WHERE id = $1",
            chunk_id
        )
        .fetch_one(&pool)
        .await?;

        assert!(row.code_embedding.is_some()); // Original 1536-dim preserved
        assert!(row.code_embedding_ollama.is_some()); // New 768-dim added

        Ok(())
    }
}
```

## Dependencies
- MPEMBED-4002 (Updated upsert_embeddings signature must exist)
- MPEMBED-2005 (Refactored embedding service with provider abstraction)

## Risk Assessment
- **Risk**: Dimension caching may become stale if provider changes
  - **Mitigation**: Pipeline is created per-run, dimension queried at construction; providers are immutable
- **Risk**: Validation overhead may impact throughput
  - **Mitigation**: Validation is O(1) per embedding, negligible compared to network I/O
- **Risk**: Incremental mode query may be slow on large datasets
  - **Mitigation**: Query uses indexed columns, add LIMIT to process in batches
- **Risk**: Mixed-dimension chunks may confuse users
  - **Mitigation**: Statistics clearly show which dimension was processed, logs explain behavior

## Files/Packages Affected
- crates/maproom/src/embedding/pipeline.rs (modify - add dimension tracking)
- crates/maproom/src/embedding/stats.rs (modify - add dimension to stats)
- crates/maproom/src/cli/scan.rs (modify - display dimension in output)
- crates/maproom/tests/integration/pipeline_test.rs (create)
