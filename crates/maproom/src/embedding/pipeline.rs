//! Embedding generation pipeline for batch processing of code chunks.
//!
//! This module provides a batch embedding generation pipeline that:
//! - Generates embeddings for all existing code chunks in the database
//! - Supports incremental updates (only process chunks with NULL embeddings)
//! - Provides progress reporting and cost tracking
//! - Handles errors and rate limiting gracefully

use crate::db::traits::StoreEmbeddings;
use crate::db::traits::StoreEncoding;
use crate::db::SqliteStore;
use crate::embedding::service::EmbeddingService;
use anyhow::{Context, Result};
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

    /// Chunks copied from code_embeddings table
    pub copied_from_cache: usize,

    /// Cost saved from reusing embeddings (USD)
    pub cost_saved_usd: f64,

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

    /// Embedding dimension
    pub dimension: usize,

    /// Provider name
    pub provider: String,
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
             Provider: {} ({} dimensions)\n\
             Generated: {}, Cached: {}, Copied from DB: {}, Failed: {}\n\
             Cache hit rate: {:.1}%\n\
             API calls: {}, Tokens: {}, Cost: ${:.4}\n\
             Cost saved from reuse: ${:.4}",
            self.total_chunks,
            self.duration_secs,
            self.chunks_per_second(),
            self.provider,
            self.dimension,
            self.embeddings_generated,
            self.embeddings_cached,
            self.copied_from_cache,
            self.failed_chunks,
            self.cache_hit_rate * 100.0,
            self.api_calls,
            self.total_tokens,
            self.estimated_cost_usd,
            self.cost_saved_usd
        )
    }
}

/// Embedding generation pipeline.
pub struct EmbeddingPipeline {
    service: EmbeddingService,
    config: PipelineConfig,
    dimension: usize,
    provider_name: String,
}

impl EmbeddingPipeline {
    /// Create a new embedding pipeline.
    pub fn new(service: EmbeddingService, config: PipelineConfig) -> Self {
        let dimension = service.dimension();
        let provider_name = service.provider_name().to_string();

        info!(
            "Initialized embedding pipeline: provider={}, dimension={}",
            provider_name, dimension
        );

        Self {
            service,
            config,
            dimension,
            provider_name,
        }
    }

    /// Get the embedding dimension for this pipeline.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Get the provider name for this pipeline.
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Copy embeddings from code_embeddings table to chunks with NULL embeddings.
    ///
    /// This method queries the code_embeddings deduplication table and copies
    /// existing embeddings to chunks that have matching blob_sha but NULL embeddings.
    /// This is the critical step that enables embedding inheritance across worktrees.
    ///
    /// # Arguments
    /// * `store` - SQLite database store
    ///
    /// # Returns
    /// Number of chunks that had embeddings copied
    ///
    /// # Errors
    /// Returns error if database query fails
    pub async fn copy_existing_embeddings(&self, store: &SqliteStore) -> Result<usize> {
        info!("Copying existing embeddings from code_embeddings table");

        let count = store
            .copy_existing_embeddings_from_cache()
            .await
            .context("Failed to copy embeddings from code_embeddings table")?;

        if count > 0 {
            info!(
                "Copied embeddings for {} chunks from code_embeddings table",
                count
            );
        } else {
            debug!("No embeddings to copy from code_embeddings table");
        }

        Ok(count as usize)
    }

    /// Run the embedding generation pipeline.
    pub async fn run(&self, store: &SqliteStore) -> Result<PipelineStats> {
        self.run_with_progress(store, None).await
    }

    /// Run the embedding pipeline with optional progress callback
    pub async fn run_with_progress(
        &self,
        store: &SqliteStore,
        progress_callback: Option<&dyn Fn(usize, usize)>,
    ) -> Result<PipelineStats> {
        let start_time = std::time::Instant::now();
        let mut stats = PipelineStats {
            dimension: self.dimension,
            provider: self.provider_name.clone(),
            ..Default::default()
        };

        info!("Starting embedding generation pipeline");
        info!(
            "Config: batch_size={}, incremental={}, dry_run={}, sample_size={:?}",
            self.config.batch_size,
            self.config.incremental,
            self.config.dry_run,
            self.config.sample_size
        );
        info!(
            "Provider: {} (dimension: {})",
            self.provider_name, self.dimension
        );

        // PROGRESS TRACKING: Mark stale runs as failed before starting new run
        if let Err(e) = store.mark_stale_runs_as_failed().await {
            warn!("Failed to mark stale encoding runs as failed: {}", e);
        }

        // STEP 1: Copy existing embeddings from code_embeddings table
        // This is the critical missing step from BLOBSHA infrastructure
        match self.copy_existing_embeddings(store).await {
            Ok(copied_count) => {
                stats.copied_from_cache = copied_count;
                // Calculate cost saved: $0.00013 per 1K tokens (OpenAI text-embedding-3-small)
                // Average chunk is ~1K tokens, so we use copied_count directly
                stats.cost_saved_usd = copied_count as f64 * 0.00013;
                info!(
                    "Copied {} embeddings from cache, saved ${:.4}",
                    copied_count, stats.cost_saved_usd
                );
            }
            Err(e) => {
                warn!("Failed to copy embeddings from cache: {}", e);
                // Continue with generation - this is not a fatal error
            }
        }

        // STEP 2: Fetch chunks that still need embeddings (after copy step)
        let chunks = self.fetch_chunks_needing_embeddings(store).await?;
        stats.total_chunks = chunks.len();

        if chunks.is_empty() {
            info!("No chunks need embeddings");
            return Ok(stats);
        }

        info!("Found {} chunks needing embeddings", chunks.len());

        // PROGRESS TRACKING: Create encoding run record
        let run_id: Option<i64> = match store
            .create_encoding_run(
                chunks.len() as i64,
                Some(&self.provider_name),
                Some(self.dimension as i32),
            )
            .await
        {
            Ok(id) => {
                info!("Created encoding run {} for {} chunks", id, chunks.len());
                Some(id)
            }
            Err(e) => {
                warn!("Failed to create encoding run record: {}", e);
                None
            }
        };

        // Run the main processing loop, capturing any fatal error so we can
        // mark the encoding run as "failed" before propagating it.
        let processing_result = self
            .run_batch_loop(
                store,
                &chunks,
                &mut stats,
                progress_callback,
                start_time,
                run_id,
            )
            .await;

        // Gather final metrics regardless of success/failure
        let cache_metrics = self.service.cache_metrics().await;
        stats.cache_hit_rate = cache_metrics.hit_rate();

        // Get provider metrics if available
        if let Some(provider_metrics) = self.service.provider_metrics() {
            stats.total_tokens = provider_metrics.total_tokens;
            stats.estimated_cost_usd = provider_metrics.estimated_cost_usd;
            stats.api_calls = provider_metrics.total_requests as usize;
        }

        stats.duration_secs = start_time.elapsed().as_secs_f64();

        // PROGRESS TRACKING: Mark encoding run as completed or failed
        if let Some(id) = run_id {
            let final_status = if processing_result.is_ok() {
                "completed"
            } else {
                "failed"
            };
            if let Err(e) = store.complete_encoding_run(id, final_status).await {
                warn!("Failed to mark encoding run as {}: {}", final_status, e);
            }
        }

        // Propagate fatal errors after marking the run
        processing_result?;

        info!("Pipeline completed");
        info!("{}", stats.summary());

        Ok(stats)
    }

    /// Run the batch processing loop for embedding generation.
    ///
    /// This is extracted from `run_with_progress` so that any fatal error can be
    /// caught by the caller and the encoding run can be marked as "failed" before
    /// the error is propagated.
    async fn run_batch_loop(
        &self,
        store: &SqliteStore,
        chunks: &[ChunkRow],
        stats: &mut PipelineStats,
        progress_callback: Option<&dyn Fn(usize, usize)>,
        start_time: std::time::Instant,
        run_id: Option<i64>,
    ) -> Result<()> {
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
                if let Some(metrics) = self.service.provider_metrics() {
                    let current_cost = metrics.estimated_cost_usd;
                    if current_cost >= max_cost {
                        warn!(
                            "Cost ceiling reached: ${:.4} >= ${:.4}",
                            current_cost, max_cost
                        );
                        break;
                    }
                }
            }

            // Generate embeddings for batch
            match self.process_batch(store, batch, stats).await {
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
            info!("Progress: {}% ({}/{})", progress, batch_num, total_batches);

            // Call progress callback if provided
            if let Some(callback) = progress_callback {
                let chunks_processed =
                    std::cmp::min(batch_num * self.config.batch_size, chunks.len());
                callback(chunks_processed, chunks.len());
            }

            // PROGRESS TRACKING: Update encoding run progress after each batch
            if let Some(id) = run_id {
                let chunks_completed =
                    std::cmp::min(batch_num * self.config.batch_size, chunks.len()) as i64;
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                let chunks_per_second = if elapsed_secs > 0.0 {
                    chunks_completed as f64 / elapsed_secs
                } else {
                    0.0
                };
                if let Err(e) = store
                    .update_encoding_run_progress(id, chunks_completed, Some(chunks_per_second))
                    .await
                {
                    warn!("Failed to update encoding run progress: {}", e);
                }
            }
        }

        // If every chunk failed, treat the entire run as a fatal error so the
        // encoding run is marked as "failed" rather than "completed".
        if stats.failed_chunks > 0 && stats.failed_chunks >= stats.total_chunks {
            anyhow::bail!(
                "All {} chunks failed during embedding generation",
                stats.total_chunks
            );
        }

        Ok(())
    }

    /// Fetch chunks that need embeddings.
    async fn fetch_chunks_needing_embeddings(&self, store: &SqliteStore) -> Result<Vec<ChunkRow>> {
        let chunks = store
            .fetch_chunks_needing_embeddings(self.config.incremental, self.config.sample_size)
            .await
            .context("Failed to fetch chunks")?;

        // Convert ChunkForEmbedding to ChunkRow
        let chunk_rows: Vec<ChunkRow> = chunks
            .into_iter()
            .map(|chunk| ChunkRow {
                id: chunk.id,
                signature: chunk.signature,
                docstring: chunk.docstring,
                preview: chunk.preview,
                blob_sha: Some(chunk.blob_sha),
            })
            .collect();

        Ok(chunk_rows)
    }

    /// Process a batch of chunks.
    async fn process_batch(
        &self,
        store: &SqliteStore,
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
        let (code_embeddings, code_batch_stats) =
            match self.service.embed_batch_with_stats(code_texts).await {
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
                // Store embedding using blob_sha for deduplication
                if let Some(blob_sha) = &chunk.blob_sha {
                    store
                        .upsert_embedding(blob_sha, &code_embeddings[i], &self.provider_name)
                        .await?;
                }
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
        for emb in embeddings.iter() {
            if emb.len() != self.dimension {
                use crate::embedding::error::{DimensionMismatchError, EmbeddingError};
                return Err(
                    EmbeddingError::DimensionMismatch(DimensionMismatchError::new(
                        self.dimension,
                        emb.len(),
                        self.provider_name.clone(),
                        "unknown".to_string(), // Pipeline doesn't have access to model name
                        self.dimension,
                    ))
                    .into(),
                );
            }
        }

        Ok(())
    }

    /// Process only chunks missing embeddings for this dimension (incremental mode).
    ///
    /// This method queries for chunks that are missing embeddings for the pipeline's
    /// configured dimension and processes only those chunks. This allows for efficient
    /// incremental updates when adding new embedding dimensions without reprocessing
    /// chunks that already have embeddings from other providers.
    ///
    /// # Arguments
    /// * `client` - Database client
    /// * `repo` - Repository name to filter chunks
    /// * `worktree` - Worktree name to filter chunks
    ///
    /// # Returns
    /// Pipeline statistics for the incremental update
    ///
    /// # Example
    /// ```no_run
    /// # use crewchief_maproom::embedding::pipeline::{EmbeddingPipeline, PipelineConfig};
    /// # use crewchief_maproom::embedding::service::EmbeddingService;
    /// # async fn example() -> anyhow::Result<()> {
    /// # let service = EmbeddingService::from_env().await?;
    /// # let pipeline = EmbeddingPipeline::new(service, PipelineConfig::default());
    /// # let client = crate::db::queries::connect().await?;
    /// // Process only chunks missing 768-dim Ollama embeddings
    /// let stats = pipeline.process_missing_embeddings(&client, "crewchief", "main").await?;
    /// println!("Processed {} chunks with {}-dim embeddings", stats.total_chunks, stats.dimension);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn process_missing_embeddings(
        &self,
        store: &SqliteStore,
        _repo: &str, // TODO: Add repo/worktree filtering to fetch_chunks_needing_embeddings
        _worktree: &str, // Currently unused - all chunks are processed regardless of repo/worktree
    ) -> Result<PipelineStats> {
        info!(
            "Finding chunks missing {}-dimensional embeddings (provider: {})",
            self.dimension, self.provider_name
        );

        // For SQLite, we query all chunks that need embeddings (by blob_sha)
        // and then filter by repo/worktree
        // This is less efficient than a JOIN but simpler for now
        // TODO: Add repo/worktree filtering to fetch_chunks_needing_embeddings
        let all_chunks = store
            .fetch_chunks_needing_embeddings(true, None)
            .await
            .context("Failed to query chunks missing embeddings")?;

        // For now, process all chunks (repo/worktree filtering not implemented yet)
        let chunk_ids: Vec<i64> = all_chunks.iter().map(|c| c.id).collect();

        info!(
            "Found {} chunks missing {}-dimensional embeddings (provider: {})",
            chunk_ids.len(),
            self.dimension,
            self.provider_name
        );

        if chunk_ids.is_empty() {
            return Ok(PipelineStats {
                dimension: self.dimension,
                provider: self.provider_name.clone(),
                ..Default::default()
            });
        }

        // Convert to ChunkRow format and process
        let chunks = self.fetch_chunks_by_ids(store, &chunk_ids).await?;
        let start_time = std::time::Instant::now();
        let mut stats = PipelineStats {
            dimension: self.dimension,
            provider: self.provider_name.clone(),
            total_chunks: chunks.len(),
            ..Default::default()
        };

        // Process chunks in batches
        for (batch_idx, batch) in chunks.chunks(self.config.batch_size).enumerate() {
            let batch_num = batch_idx + 1;
            let total_batches = chunks.len().div_ceil(self.config.batch_size);

            info!(
                "Processing incremental batch {}/{} ({} chunks)",
                batch_num,
                total_batches,
                batch.len()
            );

            // Check cost ceiling
            if let Some(max_cost) = self.config.max_cost_usd {
                if let Some(metrics) = self.service.provider_metrics() {
                    let current_cost = metrics.estimated_cost_usd;
                    if current_cost >= max_cost {
                        warn!(
                            "Cost ceiling reached: ${:.4} >= ${:.4}",
                            current_cost, max_cost
                        );
                        break;
                    }
                }
            }

            // Generate embeddings for batch
            match self.process_batch(store, batch, &mut stats).await {
                Ok(_) => {
                    debug!("Incremental batch {} completed successfully", batch_num);
                }
                Err(e) => {
                    warn!("Incremental batch {} failed: {}", batch_num, e);
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
                "Incremental progress: {}% ({}/{})",
                progress, batch_num, total_batches
            );
        }

        // Gather final metrics
        let cache_metrics = self.service.cache_metrics().await;
        stats.cache_hit_rate = cache_metrics.hit_rate();

        // Get provider metrics if available
        if let Some(provider_metrics) = self.service.provider_metrics() {
            stats.total_tokens = provider_metrics.total_tokens;
            stats.estimated_cost_usd = provider_metrics.estimated_cost_usd;
            stats.api_calls = provider_metrics.total_requests as usize;
        }

        stats.duration_secs = start_time.elapsed().as_secs_f64();

        info!("Incremental embedding generation completed");
        info!("{}", stats.summary());

        Ok(stats)
    }

    /// Fetch chunks by their IDs.
    async fn fetch_chunks_by_ids(
        &self,
        store: &SqliteStore,
        chunk_ids: &[i64],
    ) -> Result<Vec<ChunkRow>> {
        if chunk_ids.is_empty() {
            return Ok(Vec::new());
        }

        // For SQLite, we need to fetch chunks from the database
        // Since we don't have a direct method for this, we'll fetch all chunks that need embeddings
        // and filter by ID (inefficient but works for now)
        let all_chunks = store
            .fetch_chunks_needing_embeddings(true, None)
            .await
            .context("Failed to fetch chunks by IDs")?;

        let chunk_id_set: std::collections::HashSet<i64> = chunk_ids.iter().copied().collect();
        let chunks: Vec<ChunkRow> = all_chunks
            .into_iter()
            .filter(|c| chunk_id_set.contains(&c.id))
            .map(|chunk| ChunkRow {
                id: chunk.id,
                signature: chunk.signature,
                docstring: chunk.docstring,
                preview: chunk.preview,
                blob_sha: Some(chunk.blob_sha),
            })
            .collect();

        Ok(chunks)
    }
}

/// Row data for a chunk from the database.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields read from database but not all are used directly
struct ChunkRow {
    id: i64,
    signature: Option<String>,
    docstring: Option<String>,
    preview: String,
    blob_sha: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::cache::EmbeddingCache;
    use crate::embedding::config::CacheConfig;
    use crate::embedding::error::EmbeddingError;
    use crate::embedding::provider::{EmbeddingProvider, ProviderMetrics};
    use async_trait::async_trait;
    use std::sync::Arc;

    // Mock provider for testing
    struct MockProvider {
        dimension: usize,
        name: &'static str,
    }

    #[async_trait]
    impl EmbeddingProvider for MockProvider {
        async fn embed(&self, _text: String) -> Result<Vec<f32>, EmbeddingError> {
            Ok(vec![0.0; self.dimension])
        }

        async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
            Ok(vec![vec![0.0; self.dimension]; texts.len()])
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }

        fn metrics(&self) -> Option<ProviderMetrics> {
            Some(ProviderMetrics {
                total_requests: 10,
                total_tokens: 1000,
                failed_requests: 0,
                estimated_cost_usd: 0.001,
            })
        }
    }

    fn create_test_service(dimension: usize, name: &'static str) -> EmbeddingService {
        let provider = Box::new(MockProvider { dimension, name });
        let cache_config = CacheConfig {
            max_entries: 100,
            ttl_seconds: 3600,
            enable_metrics: true,
        };
        let cache = EmbeddingCache::new(cache_config).unwrap();
        EmbeddingService::new(provider, Arc::new(cache))
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
            copied_from_cache: 0,
            cost_saved_usd: 0.0,
            failed_chunks: 0,
            api_calls: 10,
            total_tokens: 50000,
            estimated_cost_usd: 1.0,
            cache_hit_rate: 0.8,
            duration_secs: 10.0,
            dimension: 1536,
            provider: "openai".to_string(),
        };

        assert_eq!(stats.chunks_per_second(), 100.0);
        assert!(stats.summary().contains("1000 chunks"));
        assert!(stats.summary().contains("$1.0000"));
        assert!(stats.summary().contains("openai"));
        assert!(stats.summary().contains("1536 dimensions"));
    }

    #[test]
    fn test_pipeline_dimension_caching() {
        let service = create_test_service(768, "ollama");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        assert_eq!(pipeline.dimension(), 768);
        assert_eq!(pipeline.provider_name(), "ollama");
    }

    #[test]
    fn test_pipeline_dimension_matches_service() {
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig::default();

        // Store dimension and provider name before moving service
        let expected_dim = service.dimension();
        let expected_provider = service.provider_name().to_string();

        let pipeline = EmbeddingPipeline::new(service, config);

        assert_eq!(pipeline.dimension(), expected_dim);
        assert_eq!(pipeline.provider_name(), expected_provider);
    }

    #[test]
    fn test_prepare_code_text() {
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        let chunk = ChunkRow {
            id: 1,
            signature: Some("function foo()".to_string()),
            docstring: Some("A test function".to_string()),
            preview: "console.log('test')".to_string(),
            blob_sha: Some("abc123".to_string()),
        };

        let text = pipeline.prepare_code_text(&chunk);
        assert!(text.contains("function foo()"));
        assert!(text.contains("A test function"));
        assert!(text.contains("console.log"));
    }

    #[test]
    fn test_prepare_text_summary() {
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        let chunk = ChunkRow {
            id: 1,
            signature: Some("function foo()".to_string()),
            docstring: Some("A test function".to_string()),
            preview: "console.log('test')".to_string(),
            blob_sha: Some("abc123".to_string()),
        };

        let text = pipeline.prepare_text_summary(&chunk);
        assert_eq!(text, "A test function");
    }

    #[test]
    fn test_validate_embeddings() {
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        let valid_embeddings = vec![vec![0.1; 1536], vec![0.2; 1536]];
        assert!(pipeline.validate_embeddings(&valid_embeddings).is_ok());

        let invalid_embeddings = vec![vec![0.1; 768], vec![0.2; 1536]];
        assert!(pipeline.validate_embeddings(&invalid_embeddings).is_err());
    }

    #[test]
    fn test_validate_embeddings_dimension_mismatch() {
        // Test with 768-dim pipeline
        let service = create_test_service(768, "ollama");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        // Should fail with 1536-dim embeddings
        let wrong_dim_embeddings = vec![vec![0.1; 1536]];
        let result = pipeline.validate_embeddings(&wrong_dim_embeddings);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("768"));
        assert!(err_msg.contains("1536"));
        assert!(err_msg.contains("ollama"));
    }

    // ========================================================================
    // Tests for pipeline progress instrumentation - ENCPROG.2002
    // ========================================================================

    use crate::db::sqlite::SqliteStore;
    use crate::db::traits::StoreChunks;
    use crate::db::traits::StoreCore;
    use crate::db::traits::StoreEncoding;
    use crate::db::{ChunkRecord, FileRecord};
    use rusqlite::params;

    /// Helper to create a test store with migrations applied.
    async fn setup_test_store() -> SqliteStore {
        SqliteStore::connect(":memory:").await.unwrap()
    }

    /// Helper to create test data: a repo, worktree, commit, file, and N chunks.
    async fn setup_pipeline_test_data(store: &SqliteStore, num_chunks: usize) {
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();

        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash_test".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        for i in 0..num_chunks {
            let chunk = ChunkRecord {
                file_id,
                worktree_id,
                blob_sha: format!("blob_test_{}", i),
                symbol_name: Some(format!("fn_{}", i)),
                kind: "function".to_string(),
                signature: Some(format!("fn fn_{}()", i)),
                docstring: Some(format!("Test function {}", i)),
                start_line: (i * 10 + 1) as i32,
                end_line: (i * 10 + 10) as i32,
                preview: format!("fn fn_{}() {{}}", i),
                ts_doc_text: String::new(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
            };
            store.insert_chunk(&chunk).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_pipeline_progress_persisted_during_run() {
        // Setup: create store with test chunks
        let store = setup_test_store().await;
        setup_pipeline_test_data(&store, 5).await;

        // Create pipeline with small batch size so we get multiple batches
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig {
            batch_size: 2,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);

        // Run the pipeline
        let stats = pipeline.run(&store).await.unwrap();
        assert_eq!(stats.total_chunks, 5);

        // Verify: encoding run was created and completed
        // The run should be completed (not active anymore)
        let active_run = store.get_active_encoding_run().await.unwrap();
        assert!(active_run.is_none(), "Run should be completed, not active");

        // Query the encoding_runs table directly to verify the record
        store
            .run(move |conn| {
                let (status, total_chunks, chunks_completed, provider, dimension, finished_at): (
                    String,
                    i64,
                    i64,
                    Option<String>,
                    Option<i32>,
                    Option<String>,
                ) = conn.query_row(
                    "SELECT status, total_chunks, chunks_completed, provider, dimension, finished_at FROM encoding_runs ORDER BY id DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                )?;

                assert_eq!(status, "completed");
                assert_eq!(total_chunks, 5);
                // chunks_completed should be 5 (all chunks processed across 3 batches: 2+2+1)
                assert_eq!(chunks_completed, 5);
                assert_eq!(provider, Some("openai".to_string()));
                assert_eq!(dimension, Some(1536));
                assert!(finished_at.is_some(), "finished_at should be set");

                // Verify chunks_per_second was set
                let cps: Option<f64> = conn.query_row(
                    "SELECT chunks_per_second FROM encoding_runs ORDER BY id DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )?;
                assert!(cps.is_some(), "chunks_per_second should have been set");
                assert!(cps.unwrap() > 0.0, "chunks_per_second should be positive");

                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_pipeline_stale_runs_cleaned_on_startup() {
        // Setup: create store and insert a "stale" running run
        let store = setup_test_store().await;
        let stale_run_id = store
            .create_encoding_run(500, Some("ollama"), Some(768))
            .await
            .unwrap();

        // Verify it's active
        let active = store.get_active_encoding_run().await.unwrap();
        assert!(active.is_some(), "Stale run should be active initially");
        assert_eq!(active.unwrap().id, stale_run_id);

        // Create some test chunks so the pipeline has work to do
        setup_pipeline_test_data(&store, 2).await;

        // Run the pipeline - it should mark stale runs as failed first
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig {
            batch_size: 10,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);
        let _stats = pipeline.run(&store).await.unwrap();

        // Verify: the stale run should now be marked as failed
        store
            .run(move |conn| {
                let status: String = conn.query_row(
                    "SELECT status FROM encoding_runs WHERE id = ?1",
                    params![stale_run_id],
                    |row| row.get(0),
                )?;
                assert_eq!(status, "failed", "Stale run should be marked as failed");

                // Also verify a new run was created and completed
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM encoding_runs WHERE status = 'completed'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(
                    count, 1,
                    "There should be exactly one completed run (the new pipeline run)"
                );

                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_pipeline_no_run_created_when_no_chunks() {
        // Setup: create store with no chunks
        let store = setup_test_store().await;

        let service = create_test_service(1536, "openai");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        // Run the pipeline - should return early with no encoding run created
        let stats = pipeline.run(&store).await.unwrap();
        assert_eq!(stats.total_chunks, 0);

        // Verify: no encoding run was created
        store
            .run(move |conn| {
                let count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM encoding_runs", [], |row| row.get(0))?;
                assert_eq!(
                    count, 0,
                    "No encoding run should be created when there are no chunks"
                );
                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_pipeline_progress_tracks_provider_and_dimension() {
        let store = setup_test_store().await;
        setup_pipeline_test_data(&store, 3).await;

        // Use a specific provider and dimension
        let service = create_test_service(768, "ollama");
        let config = PipelineConfig {
            batch_size: 10,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);
        let _stats = pipeline.run(&store).await.unwrap();

        // Verify provider and dimension were persisted
        store
            .run(move |conn| {
                let (provider, dimension): (Option<String>, Option<i32>) = conn.query_row(
                    "SELECT provider, dimension FROM encoding_runs ORDER BY id DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                assert_eq!(provider, Some("ollama".to_string()));
                assert_eq!(dimension, Some(768));
                Ok(())
            })
            .await
            .unwrap();
    }

    // Mock provider that always fails - used to test error handling
    struct FailingProvider {
        dimension: usize,
        name: &'static str,
    }

    #[async_trait]
    impl EmbeddingProvider for FailingProvider {
        async fn embed(&self, _text: String) -> Result<Vec<f32>, EmbeddingError> {
            Err(EmbeddingError::Other(
                "simulated embedding failure".to_string(),
            ))
        }

        async fn embed_batch(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
            Err(EmbeddingError::Other(
                "simulated batch embedding failure".to_string(),
            ))
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }

        fn metrics(&self) -> Option<ProviderMetrics> {
            None
        }
    }

    fn create_failing_service(dimension: usize, name: &'static str) -> EmbeddingService {
        let provider = Box::new(FailingProvider { dimension, name });
        let cache_config = CacheConfig {
            max_entries: 100,
            ttl_seconds: 3600,
            enable_metrics: true,
        };
        let cache = EmbeddingCache::new(cache_config).unwrap();
        EmbeddingService::new(provider, Arc::new(cache))
    }

    #[tokio::test]
    async fn test_pipeline_run_marks_encoding_run_as_failed_on_error() {
        // Setup: create store with test chunks
        let store = setup_test_store().await;
        setup_pipeline_test_data(&store, 3).await;

        // Create pipeline with a provider that always fails
        let service = create_failing_service(1536, "failing-provider");
        let config = PipelineConfig {
            batch_size: 2,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);

        // Run the pipeline - it should return an error since all batches fail
        let result = pipeline.run(&store).await;
        assert!(
            result.is_err(),
            "Pipeline should return an error when all chunks fail"
        );

        // Verify: encoding run should be marked as "failed", NOT left as "running"
        let active_run = store.get_active_encoding_run().await.unwrap();
        assert!(
            active_run.is_none(),
            "No run should be active (running) after a failure"
        );

        // Query the encoding_runs table directly to verify the record
        store
            .run(move |conn| {
                let (status, total_chunks, finished_at): (String, i64, Option<String>) =
                    conn.query_row(
                        "SELECT status, total_chunks, finished_at FROM encoding_runs ORDER BY id DESC LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )?;

                assert_eq!(status, "failed", "Encoding run should be marked as 'failed'");
                assert_eq!(total_chunks, 3);
                assert!(
                    finished_at.is_some(),
                    "finished_at should be set even on failure"
                );

                Ok(())
            })
            .await
            .unwrap();
    }

    // ========================================================================
    // Tests for copy_existing_embeddings() - EMBCOPY-1002
    // ========================================================================
    //
    // REMOVED: PostgreSQL-specific test helpers that reference removed dependencies
    // (tokio_postgres, pgvector, crate::db::queries). These tests are disabled after
    // the SQLite migration. If PostgreSQL support is re-added, these can be restored.

    /*
    // PostgreSQL-specific test helpers - only compile when NOT using sqlite feature
    #[cfg(not(feature = "sqlite"))]
    /// Helper function to create a test database client
    async fn create_test_client() -> Result<tokio_postgres::Client> {
        crate::db::queries::connect().await
    }

    #[cfg(not(feature = "sqlite"))]
    /// Helper function to set up test data for embedding copy tests
    /// Returns (repo_id, worktree_id, file_id, chunk_id, blob_sha)
    async fn setup_test_chunk(
        client: &tokio_postgres::Client,
        with_embeddings: bool,
    ) -> Result<(i64, i64, i64, i64, String)> {
        // Generate unique repo name to avoid conflicts in parallel tests
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let repo_name = format!("test_repo_{}", unique_id);

        // Create test repo
        let repo_row = client
            .query_one(
                "INSERT INTO maproom.repos (name, root_path) VALUES ($1, $2) RETURNING id",
                &[&repo_name, &"/tmp/test_repo"],
            )
            .await?;
        let repo_id: i64 = repo_row.get(0);

        // Create test worktree
        let worktree_row = client
            .query_one(
                "INSERT INTO maproom.worktrees (repo_id, name, abs_path) VALUES ($1, $2, $3) RETURNING id",
                &[&repo_id, &"test_worktree", &"/tmp/test"],
            )
            .await?;
        let worktree_id: i64 = worktree_row.get(0);

        // Create test commit with unique SHA
        let commit_sha = format!("sha_{}", unique_id);
        let commit_row = client
            .query_one(
                "INSERT INTO maproom.commits (repo_id, sha) VALUES ($1, $2) RETURNING id",
                &[&repo_id, &commit_sha],
            )
            .await?;
        let commit_id: i64 = commit_row.get(0);

        // Create test file
        let file_row = client
            .query_one(
                "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
                &[&repo_id, &worktree_id, &commit_id, &"test.rs", &"rust", &"hash123"],
            )
            .await?;
        let file_id: i64 = file_row.get(0);

        // Create unique blob_sha for this chunk to avoid test contamination
        let blob_sha = format!("blob_sha_{}", unique_id);

        // Create test chunk with or without embeddings
        // Convert to pgvector::Vector for PostgreSQL compatibility
        let code_emb = if with_embeddings {
            Some(pgvector::Vector::from(vec![0.1; 1536]))
        } else {
            None
        };
        let text_emb = if with_embeddings {
            Some(pgvector::Vector::from(vec![0.2; 1536]))
        } else {
            None
        };

        let chunk_row = client
            .query_one(
                r#"
                INSERT INTO maproom.chunks
                (file_id, start_line, end_line, kind, symbol_name, preview, blob_sha, code_embedding, text_embedding)
                VALUES ($1, $2, $3, 'func'::maproom.symbol_kind, $4, $5, $6, $7, $8)
                RETURNING id
                "#,
                &[
                    &file_id,
                    &1i32,
                    &10i32,
                    &"test_fn",
                    &"fn test_fn() {}",
                    &blob_sha,
                    &code_emb,
                    &text_emb,
                ],
            )
            .await?;
        let chunk_id: i64 = chunk_row.get(0);

        Ok((
            repo_id,
            worktree_id,
            file_id,
            chunk_id,
            blob_sha.to_string(),
        ))
    }

    #[cfg(not(feature = "sqlite"))]
    /// Helper function to insert a code_embeddings cache entry
    async fn insert_cache_entry(client: &tokio_postgres::Client, blob_sha: &str) -> Result<()> {
        let embedding_vec = pgvector::Vector::from(vec![0.5; 1536]);
        client
            .execute(
                r#"
                INSERT INTO maproom.code_embeddings (blob_sha, embedding)
                VALUES ($1, $2)
                ON CONFLICT (blob_sha) DO NOTHING
                "#,
                &[&blob_sha, &embedding_vec],
            )
            .await?;
        Ok(())
    }

    #[cfg(not(feature = "sqlite"))]
    /// Helper function to clean up test data
    /// Also accepts the blob_sha to ensure we clean up code_embeddings even if chunks are deleted
    async fn cleanup_test_data(
        client: &tokio_postgres::Client,
        repo_id: i64,
        blob_sha: Option<&str>,
    ) -> Result<()> {
        // Delete code_embeddings entry if blob_sha provided
        if let Some(sha) = blob_sha {
            client
                .execute(
                    "DELETE FROM maproom.code_embeddings WHERE blob_sha = $1",
                    &[&sha],
                )
                .await?;
        }

        // Delete in reverse order of dependencies
        client
            .execute("DELETE FROM maproom.chunks WHERE file_id IN (SELECT id FROM maproom.files WHERE worktree_id IN (SELECT id FROM maproom.worktrees WHERE repo_id = $1))", &[&repo_id])
            .await?;
        client
            .execute("DELETE FROM maproom.files WHERE worktree_id IN (SELECT id FROM maproom.worktrees WHERE repo_id = $1)", &[&repo_id])
            .await?;
        client
            .execute(
                "DELETE FROM maproom.worktrees WHERE repo_id = $1",
                &[&repo_id],
            )
            .await?;
        client
            .execute(
                "DELETE FROM maproom.commits WHERE repo_id = $1",
                &[&repo_id],
            )
            .await?;
        client
            .execute("DELETE FROM maproom.repos WHERE id = $1", &[&repo_id])
            .await?;
        Ok(())
    }
    */

    // PostgreSQL tests also disabled (they reference the removed helper functions above)
    /*
    #[tokio::test]
    #[serial_test::serial]
    #[cfg(not(feature = "sqlite"))]
    async fn test_copy_existing_embeddings_success() {
        let client = create_test_client()
            .await
            .expect("Failed to connect to test database");

        // Setup: Create chunk with NULL embeddings
        let (repo_id, _worktree_id, _file_id, chunk_id, blob_sha) =
            setup_test_chunk(&client, false)
                .await
                .expect("Failed to setup test chunk");

        // Insert matching code_embeddings entry
        insert_cache_entry(&client, &blob_sha)
            .await
            .expect("Failed to insert cache entry");

        // Get initial updated_at timestamp
        let initial_row = client
            .query_one(
                "SELECT updated_at FROM maproom.chunks WHERE id = $1",
                &[&chunk_id],
            )
            .await
            .expect("Failed to get initial timestamp");
        let initial_updated_at: std::time::SystemTime = initial_row.get(0);

        // Small delay to ensure timestamp will differ
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Create pipeline and execute copy
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        let count = pipeline
            .copy_existing_embeddings(&client)
            .await
            .expect("Failed to copy embeddings");

        // Assert: Return count should be 1
        assert_eq!(count, 1, "Expected to copy 1 embedding");

        // Assert: Chunk should now have embeddings
        let updated_row = client
            .query_one(
                "SELECT code_embedding, text_embedding, updated_at FROM maproom.chunks WHERE id = $1",
                &[&chunk_id],
            )
            .await
            .expect("Failed to get updated chunk");

        let code_emb: Option<pgvector::Vector> = updated_row.get(0);
        let text_emb: Option<pgvector::Vector> = updated_row.get(1);
        let updated_at: std::time::SystemTime = updated_row.get(2);

        assert!(code_emb.is_some(), "Code embedding should be populated");
        assert!(text_emb.is_some(), "Text embedding should be populated");
        assert_eq!(
            code_emb.unwrap().to_vec().len(),
            1536,
            "Code embedding should have correct dimension"
        );
        assert_eq!(
            text_emb.unwrap().to_vec().len(),
            1536,
            "Text embedding should have correct dimension"
        );

        // Assert: updated_at timestamp should have changed
        assert!(
            updated_at > initial_updated_at,
            "updated_at timestamp should have changed"
        );

        // Cleanup
        cleanup_test_data(&client, repo_id, Some(&blob_sha))
            .await
            .expect("Failed to cleanup test data");
    }

    #[tokio::test]
    #[serial_test::serial]
    #[cfg(not(feature = "sqlite"))]
    async fn test_copy_skips_without_cache() {
        let client = create_test_client()
            .await
            .expect("Failed to connect to test database");

        // Setup: Create chunk with NULL embeddings, but NO matching cache entry
        let (repo_id, _worktree_id, _file_id, chunk_id, blob_sha) =
            setup_test_chunk(&client, false)
                .await
                .expect("Failed to setup test chunk");

        // Create pipeline and execute copy (no cache entry exists)
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        let count = pipeline
            .copy_existing_embeddings(&client)
            .await
            .expect("Should not error when no cache entry");

        // Assert: Return count should be 0
        assert_eq!(count, 0, "Expected to copy 0 embeddings (no cache entry)");

        // Assert: Chunk should still have NULL embeddings
        let row = client
            .query_one(
                "SELECT code_embedding, text_embedding FROM maproom.chunks WHERE id = $1",
                &[&chunk_id],
            )
            .await
            .expect("Failed to get chunk");

        let code_emb: Option<pgvector::Vector> = row.get(0);
        let text_emb: Option<pgvector::Vector> = row.get(1);

        assert!(code_emb.is_none(), "Code embedding should still be NULL");
        assert!(text_emb.is_none(), "Text embedding should still be NULL");

        // Cleanup
        cleanup_test_data(&client, repo_id, Some(&blob_sha))
            .await
            .expect("Failed to cleanup test data");
    }

    #[tokio::test]
    #[serial_test::serial]
    #[cfg(not(feature = "sqlite"))]
    async fn test_copy_idempotent() {
        let client = create_test_client()
            .await
            .expect("Failed to connect to test database");

        // Setup: Create chunk with embeddings already set
        let (repo_id, _worktree_id, _file_id, chunk_id, blob_sha) = setup_test_chunk(&client, true)
            .await
            .expect("Failed to setup test chunk");

        // Insert matching code_embeddings entry (with different values)
        insert_cache_entry(&client, &blob_sha)
            .await
            .expect("Failed to insert cache entry");

        // Get initial embedding values
        let initial_row = client
            .query_one(
                "SELECT code_embedding, text_embedding FROM maproom.chunks WHERE id = $1",
                &[&chunk_id],
            )
            .await
            .expect("Failed to get initial embeddings");

        let initial_code_emb: pgvector::Vector =
            initial_row.get::<_, Option<pgvector::Vector>>(0).unwrap();
        let initial_text_emb: pgvector::Vector =
            initial_row.get::<_, Option<pgvector::Vector>>(1).unwrap();

        // Create pipeline
        let service = create_test_service(1536, "openai");
        let config = PipelineConfig::default();
        let pipeline = EmbeddingPipeline::new(service, config);

        // Execute copy first time
        let count1 = pipeline
            .copy_existing_embeddings(&client)
            .await
            .expect("First copy should not error");

        // Assert: Return count should be 0 (chunk already has embeddings)
        assert_eq!(
            count1, 0,
            "Expected to copy 0 embeddings (chunk already has embeddings)"
        );

        // Execute copy second time (idempotent test)
        let count2 = pipeline
            .copy_existing_embeddings(&client)
            .await
            .expect("Second copy should not error");

        // Assert: Return count should still be 0
        assert_eq!(count2, 0, "Expected second copy to also return 0");

        // Assert: Embeddings should be unchanged (original values preserved)
        let final_row = client
            .query_one(
                "SELECT code_embedding, text_embedding FROM maproom.chunks WHERE id = $1",
                &[&chunk_id],
            )
            .await
            .expect("Failed to get final embeddings");

        let final_code_emb: pgvector::Vector =
            final_row.get::<_, Option<pgvector::Vector>>(0).unwrap();
        let final_text_emb: pgvector::Vector =
            final_row.get::<_, Option<pgvector::Vector>>(1).unwrap();

        assert_eq!(
            final_code_emb, initial_code_emb,
            "Code embedding should be unchanged"
        );
        assert_eq!(
            final_text_emb, initial_text_emb,
            "Text embedding should be unchanged"
        );

        // Cleanup
        cleanup_test_data(&client, repo_id, Some(&blob_sha))
            .await
            .expect("Failed to cleanup test data");
    }
    */
}
