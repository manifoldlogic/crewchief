//! Embedding generation pipeline for batch processing of code chunks.
//!
//! This module provides a batch embedding generation pipeline that:
//! - Generates embeddings for all existing code chunks in the database
//! - Supports incremental updates (only process chunks with NULL embeddings)
//! - Provides progress reporting and cost tracking
//! - Handles errors and rate limiting gracefully

use crate::embedding::service::EmbeddingService;
use anyhow::{Context, Result};
use tokio_postgres::Client;
use tracing::{debug, error, info, warn};

/// Configuration for the embedding generation pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Batch size for processing chunks (default: 100)
    pub batch_size: usize,

    /// Only process chunks where embeddings are NULL (default: true)
    pub incremental: bool,

    /// Dry run mode - don't write to database (default: false)
    pub dry_run: bool,

    /// Process only a sample of N chunks (None = all chunks)
    pub sample_size: Option<usize>,

    /// Delay between batches in milliseconds (default: 100ms)
    pub batch_delay_ms: u64,

    /// Maximum cost ceiling in USD (None = no limit)
    pub max_cost_usd: Option<f64>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 100,
            max_cost_usd: None,
        }
    }
}

/// Statistics for a pipeline run.
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Total chunks processed
    pub total_chunks: usize,

    /// Chunks with embeddings generated
    pub embeddings_generated: usize,

    /// Chunks from cache
    pub embeddings_cached: usize,

    /// Failed chunks
    pub failed_chunks: usize,

    /// Total API calls made
    pub api_calls: usize,

    /// Total tokens consumed
    pub total_tokens: u64,

    /// Estimated cost in USD
    pub estimated_cost_usd: f64,

    /// Cache hit rate
    pub cache_hit_rate: f64,

    /// Duration in seconds
    pub duration_secs: f64,
}

impl PipelineStats {
    /// Calculate chunks processed per second.
    pub fn chunks_per_second(&self) -> f64 {
        if self.duration_secs > 0.0 {
            self.total_chunks as f64 / self.duration_secs
        } else {
            0.0
        }
    }

    /// Format a summary of the stats.
    pub fn summary(&self) -> String {
        format!(
            "Processed {} chunks in {:.1}s ({:.1} chunks/s)\n\
             Generated: {}, Cached: {}, Failed: {}\n\
             Cache hit rate: {:.1}%\n\
             API calls: {}, Tokens: {}, Cost: ${:.4}",
            self.total_chunks,
            self.duration_secs,
            self.chunks_per_second(),
            self.embeddings_generated,
            self.embeddings_cached,
            self.failed_chunks,
            self.cache_hit_rate * 100.0,
            self.api_calls,
            self.total_tokens,
            self.estimated_cost_usd
        )
    }
}

/// Embedding generation pipeline.
pub struct EmbeddingPipeline {
    service: EmbeddingService,
    config: PipelineConfig,
}

impl EmbeddingPipeline {
    /// Create a new embedding pipeline.
    pub fn new(service: EmbeddingService, config: PipelineConfig) -> Self {
        Self { service, config }
    }

    /// Run the embedding generation pipeline.
    pub async fn run(&self, client: &Client) -> Result<PipelineStats> {
        let start_time = std::time::Instant::now();
        let mut stats = PipelineStats::default();

        info!("Starting embedding generation pipeline");
        info!(
            "Config: batch_size={}, incremental={}, dry_run={}, sample_size={:?}",
            self.config.batch_size,
            self.config.incremental,
            self.config.dry_run,
            self.config.sample_size
        );

        // Fetch chunks that need embeddings
        let chunks = self.fetch_chunks_needing_embeddings(client).await?;
        stats.total_chunks = chunks.len();

        if chunks.is_empty() {
            info!("No chunks need embeddings");
            return Ok(stats);
        }

        info!("Found {} chunks needing embeddings", chunks.len());

        // Process chunks in batches
        for (batch_idx, batch) in chunks.chunks(self.config.batch_size).enumerate() {
            let batch_num = batch_idx + 1;
            let total_batches = chunks.len().div_ceil(self.config.batch_size);

            info!(
                "Processing batch {}/{} ({} chunks)",
                batch_num,
                total_batches,
                batch.len()
            );

            // Check cost ceiling
            if let Some(max_cost) = self.config.max_cost_usd {
                let current_cost = self.service.cost_metrics().estimated_cost_usd();
                if current_cost >= max_cost {
                    warn!(
                        "Cost ceiling reached: ${:.4} >= ${:.4}",
                        current_cost, max_cost
                    );
                    break;
                }
            }

            // Generate embeddings for batch
            match self.process_batch(client, batch, &mut stats).await {
                Ok(_) => {
                    debug!("Batch {} completed successfully", batch_num);
                }
                Err(e) => {
                    warn!("Batch {} failed: {}", batch_num, e);
                    stats.failed_chunks += batch.len();
                }
            }

            // Delay between batches to avoid rate limiting
            if batch_idx < total_batches - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    self.config.batch_delay_ms,
                ))
                .await;
            }

            // Report progress
            let progress = ((batch_num as f64 / total_batches as f64) * 100.0) as u32;
            info!(
                "Progress: {}% ({}/{})",
                progress, batch_num, total_batches
            );
        }

        // Gather final metrics
        let cache_metrics = self.service.cache_metrics().await;
        let cost_metrics = self.service.cost_metrics();

        stats.cache_hit_rate = cache_metrics.hit_rate();
        stats.total_tokens = cost_metrics.total_tokens();
        stats.estimated_cost_usd = cost_metrics.estimated_cost_usd();
        stats.api_calls = cost_metrics.total_requests() as usize;
        stats.duration_secs = start_time.elapsed().as_secs_f64();

        info!("Pipeline completed");
        info!("{}", stats.summary());

        Ok(stats)
    }

    /// Fetch chunks that need embeddings.
    async fn fetch_chunks_needing_embeddings(
        &self,
        client: &Client,
    ) -> Result<Vec<ChunkRow>> {
        let query = if self.config.incremental {
            // Only fetch chunks where embeddings are NULL
            "SELECT c.id, c.signature, c.docstring, c.preview
             FROM maproom.chunks c
             WHERE c.code_embedding IS NULL OR c.text_embedding IS NULL
             ORDER BY c.id"
        } else {
            // Fetch all chunks
            "SELECT c.id, c.signature, c.docstring, c.preview
             FROM maproom.chunks c
             ORDER BY c.id"
        };

        let limit_query = if let Some(sample_size) = self.config.sample_size {
            format!("{} LIMIT {}", query, sample_size)
        } else {
            query.to_string()
        };

        let rows = client
            .query(&limit_query, &[])
            .await
            .context("Failed to fetch chunks")?;

        let chunks: Vec<ChunkRow> = rows
            .into_iter()
            .map(|row| ChunkRow {
                id: row.get(0),
                signature: row.get(1),
                docstring: row.get(2),
                preview: row.get(3),
            })
            .collect();

        Ok(chunks)
    }

    /// Process a batch of chunks.
    async fn process_batch(
        &self,
        client: &Client,
        batch: &[ChunkRow],
        stats: &mut PipelineStats,
    ) -> Result<()> {
        // Prepare texts for embedding
        let code_texts: Vec<String> = batch
            .iter()
            .map(|chunk| self.prepare_code_text(chunk))
            .collect();

        let text_texts: Vec<String> = batch
            .iter()
            .map(|chunk| self.prepare_text_summary(chunk))
            .collect();

        // Generate code embeddings
        let (code_embeddings, code_batch_stats) = match self
            .service
            .embed_batch_with_stats(code_texts)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to generate code embeddings: {:?}", e);
                return Err(e).context("Failed to generate code embeddings");
            }
        };

        stats.embeddings_generated += code_batch_stats.from_api;
        stats.embeddings_cached += code_batch_stats.cached;

        // Generate text embeddings
        let (text_embeddings, text_batch_stats) = self
            .service
            .embed_batch_with_stats(text_texts)
            .await
            .context("Failed to generate text embeddings")?;

        stats.embeddings_generated += text_batch_stats.from_api;
        stats.embeddings_cached += text_batch_stats.cached;

        // Validate embedding dimensions
        self.validate_embeddings(&code_embeddings)?;
        self.validate_embeddings(&text_embeddings)?;

        // Write to database if not dry run
        if !self.config.dry_run {
            for (i, chunk) in batch.iter().enumerate() {
                self.update_chunk_embeddings(
                    client,
                    chunk.id,
                    &code_embeddings[i],
                    &text_embeddings[i],
                )
                .await?;
            }

            debug!("Wrote {} chunk embeddings to database", batch.len());
        } else {
            debug!("Dry run: skipped writing {} embeddings", batch.len());
        }

        Ok(())
    }

    /// Prepare code text for embedding.
    fn prepare_code_text(&self, chunk: &ChunkRow) -> String {
        let mut parts = Vec::new();

        // Include signature if available
        if let Some(sig) = &chunk.signature {
            if !sig.is_empty() {
                parts.push(sig.clone());
            }
        }

        // Include docstring if available
        if let Some(doc) = &chunk.docstring {
            if !doc.is_empty() {
                parts.push(doc.clone());
            }
        }

        // Include preview (truncated body)
        parts.push(chunk.preview.clone());

        parts.join("\n")
    }

    /// Prepare text summary for embedding.
    fn prepare_text_summary(&self, chunk: &ChunkRow) -> String {
        // For now, use docstring as text summary
        // Future: implement LLM-based summarization
        if let Some(doc) = &chunk.docstring {
            if !doc.is_empty() {
                return doc.clone();
            }
        }

        // Fallback: use signature or preview
        if let Some(sig) = &chunk.signature {
            if !sig.is_empty() {
                return sig.clone();
            }
        }

        chunk.preview.clone()
    }

    /// Validate embedding dimensions.
    fn validate_embeddings(&self, embeddings: &[Vec<f32>]) -> Result<()> {
        let expected_dim = self.service.dimension();

        for (i, emb) in embeddings.iter().enumerate() {
            if emb.len() != expected_dim {
                return Err(anyhow::anyhow!(
                    "Invalid embedding dimension at index {}: expected {}, got {}",
                    i,
                    expected_dim,
                    emb.len()
                ));
            }
        }

        Ok(())
    }

    /// Update chunk embeddings in database.
    async fn update_chunk_embeddings(
        &self,
        client: &Client,
        chunk_id: i64,
        code_embedding: &[f32],
        text_embedding: &[f32],
    ) -> Result<()> {
        debug!(
            "Updating embeddings for chunk {} (code_dim={}, text_dim={})",
            chunk_id,
            code_embedding.len(),
            text_embedding.len()
        );

        // Format vectors as PostgreSQL array literal strings
        // Without the pgvector Rust crate, we must embed vectors directly in SQL
        // This is safe because vectors contain only f32 numbers (no user input)
        let code_vec_str = format!(
            "[{}]",
            code_embedding
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let text_vec_str = format!(
            "[{}]",
            text_embedding
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        // Use formatted SQL with embedded vector literals instead of parameters
        // because tokio-postgres doesn't support pgvector type conversion
        let sql = format!(
            "UPDATE maproom.chunks
             SET code_embedding = '{}'::vector,
                 text_embedding = '{}'::vector
             WHERE id = $1",
            code_vec_str, text_vec_str
        );

        client
            .execute(&sql, &[&chunk_id])
            .await
            .map(|_| ())
            .map_err(|e| {
                error!(
                    "Failed to update embeddings for chunk {}: Code dim={}, Text dim={}, Error: {:?}",
                    chunk_id,
                    code_embedding.len(),
                    text_embedding.len(),
                    e
                );
                e
            })
            .context("Failed to update chunk embeddings")?;

        Ok(())
    }
}

/// Row data for a chunk from the database.
#[derive(Debug, Clone)]
struct ChunkRow {
    id: i64,
    signature: Option<String>,
    docstring: Option<String>,
    preview: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::config::EmbeddingConfig;

    fn test_embedding_service() -> EmbeddingService {
        let mut config = EmbeddingConfig::default();
        config.api_key = Some("test-key".to_string());
        EmbeddingService::new(config).unwrap()
    }

    #[test]
    fn test_pipeline_config_defaults() {
        let config = PipelineConfig::default();
        assert_eq!(config.batch_size, 100);
        assert!(config.incremental);
        assert!(!config.dry_run);
        assert_eq!(config.sample_size, None);
        assert_eq!(config.batch_delay_ms, 100);
        assert_eq!(config.max_cost_usd, None);
    }

    #[test]
    fn test_pipeline_stats_summary() {
        let stats = PipelineStats {
            total_chunks: 1000,
            embeddings_generated: 200,
            embeddings_cached: 800,
            failed_chunks: 0,
            api_calls: 10,
            total_tokens: 50000,
            estimated_cost_usd: 1.0,
            cache_hit_rate: 0.8,
            duration_secs: 10.0,
        };

        assert_eq!(stats.chunks_per_second(), 100.0);
        assert!(stats.summary().contains("1000 chunks"));
        assert!(stats.summary().contains("$1.0000"));
    }

    #[test]
    fn test_prepare_code_text() {
        let service = test_embedding_service();
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        let chunk = ChunkRow {
            id: 1,
            signature: Some("function foo()".to_string()),
            docstring: Some("A test function".to_string()),
            preview: "console.log('test')".to_string(),
        };

        let text = pipeline.prepare_code_text(&chunk);
        assert!(text.contains("function foo()"));
        assert!(text.contains("A test function"));
        assert!(text.contains("console.log"));
    }

    #[test]
    fn test_prepare_text_summary() {
        let service = test_embedding_service();
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        let chunk = ChunkRow {
            id: 1,
            signature: Some("function foo()".to_string()),
            docstring: Some("A test function".to_string()),
            preview: "console.log('test')".to_string(),
        };

        let text = pipeline.prepare_text_summary(&chunk);
        assert_eq!(text, "A test function");
    }

    #[test]
    fn test_validate_embeddings() {
        let service = test_embedding_service();
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        let valid_embeddings = vec![vec![0.1; 1536], vec![0.2; 1536]];
        assert!(pipeline.validate_embeddings(&valid_embeddings).is_ok());

        let invalid_embeddings = vec![vec![0.1; 768], vec![0.2; 1536]];
        assert!(pipeline.validate_embeddings(&invalid_embeddings).is_err());
    }
}
