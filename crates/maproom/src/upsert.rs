//! Cache-aware chunk upsert logic using content-addressed blob SHA.
//!
//! This module provides chunk insertion with embedding deduplication:
//! - Compute blob SHA from chunk content
//! - Check if embedding exists in code_embeddings table
//! - Track cache hits/misses for cost analysis
//!
//! This is the core implementation of BLOBSHA Phase 3 (planning/plan.md lines 331-439).

use crate::content_hash::compute_blob_sha;
use crate::metrics::CacheMetrics;
use anyhow::{Context, Result};
use tokio_postgres::Client;
use tracing::{debug, info};

/// Check if an embedding exists for a given blob SHA.
///
/// This is the cache check operation - returns true if we can reuse an existing
/// embedding, false if we need to generate a new one.
///
/// # Arguments
///
/// * `client` - Database client
/// * `blob_sha` - Content hash of the chunk
///
/// # Returns
///
/// `Ok(true)` if embedding exists (cache hit), `Ok(false)` if not (cache miss)
pub async fn check_embedding_exists(client: &Client, blob_sha: &str) -> Result<bool> {
    let row = client
        .query_opt(
            "SELECT 1 FROM maproom.code_embeddings WHERE blob_sha = $1 LIMIT 1",
            &[&blob_sha],
        )
        .await
        .context("Failed to check embedding existence")?;

    Ok(row.is_some())
}

/// Insert or update a chunk with cache-aware embedding lookup.
///
/// This function implements the cache-aware upsert logic from planning/architecture.md:
/// 1. Compute blob_sha from content
/// 2. Check if embedding exists in code_embeddings
/// 3. Record cache hit or miss in metrics
/// 4. Insert chunk (embedding generation handled separately)
///
/// # Arguments
///
/// * `client` - Database client
/// * `file_id` - File ID from files table
/// * `content` - Full chunk content (for blob SHA computation)
/// * `symbol_name` - Optional symbol name
/// * `kind` - Chunk kind (function, class, etc.)
/// * `signature` - Optional function/class signature
/// * `docstring` - Optional documentation
/// * `start_line` - Starting line number
/// * `end_line` - Ending line number
/// * `preview` - Preview text
/// * `ts_doc_text` - Full-text search document
/// * `recency_score` - Git recency score
/// * `churn_score` - Git churn score
/// * `metadata` - Optional JSON metadata
/// * `metrics` - Cache metrics tracker
///
/// # Returns
///
/// The chunk ID of the inserted/updated chunk
pub async fn upsert_chunk_with_cache(
    client: &Client,
    file_id: i64,
    content: &str,
    symbol_name: Option<&str>,
    kind: &str,
    signature: Option<&str>,
    docstring: Option<&str>,
    start_line: i32,
    end_line: i32,
    preview: &str,
    ts_doc_text: &str,
    recency_score: f32,
    churn_score: f32,
    metadata: Option<&serde_json::Value>,
    metrics: &CacheMetrics,
) -> Result<i64> {
    // Step 1: Compute blob SHA from chunk content
    let blob_sha = compute_blob_sha(content);

    // Step 2: Check if embedding exists (cache check)
    let embedding_exists = check_embedding_exists(client, &blob_sha)
        .await
        .context("Failed to check embedding cache")?;

    // Step 3: Record cache hit or miss
    if embedding_exists {
        metrics.record_hit();
        debug!(
            blob_sha = %blob_sha,
            symbol = ?symbol_name,
            "Cache hit: reusing existing embedding"
        );
    } else {
        metrics.record_miss();
        debug!(
            blob_sha = %blob_sha,
            symbol = ?symbol_name,
            "Cache miss: new embedding needed"
        );
    }

    // Step 4: Insert chunk with blob_sha
    // Note: Actual embedding generation and insertion into code_embeddings
    // happens in the embedding pipeline (not in this upsert path).
    // This just records the blob_sha reference.
    let chunk_id = crate::db::insert_chunk(
        client,
        file_id,
        symbol_name,
        kind,
        signature,
        docstring,
        start_line,
        end_line,
        preview,
        ts_doc_text,
        recency_score,
        churn_score,
        metadata,
    )
    .await
    .context("Failed to insert chunk")?;

    Ok(chunk_id)
}

/// Batch insert chunks with cache-aware checking.
///
/// More efficient version for inserting multiple chunks at once.
/// Checks cache for all chunks first, then performs batch insert.
///
/// # Arguments
///
/// * `client` - Database client
/// * `chunks` - Vector of chunk data with content for blob SHA computation
/// * `metrics` - Cache metrics tracker
///
/// # Returns
///
/// Vector of chunk IDs in the same order as input chunks
pub async fn upsert_chunks_batch_with_cache(
    client: &Client,
    chunks: &[(
        i64,                          // file_id
        String,                       // content (for blob_sha)
        Option<String>,               // symbol_name
        String,                       // kind
        Option<String>,               // signature
        Option<String>,               // docstring
        i32,                          // start_line
        i32,                          // end_line
        String,                       // preview
        String,                       // ts_doc_text
        f32,                          // recency_score
        f32,                          // churn_score
        Option<serde_json::Value>,    // metadata
    )],
    metrics: &CacheMetrics,
) -> Result<Vec<i64>> {
    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    // Step 1: Compute blob SHAs for all chunks
    let blob_shas: Vec<String> = chunks
        .iter()
        .map(|(_, content, ..)| compute_blob_sha(content))
        .collect();

    // Step 2: Batch check which embeddings exist
    // Build IN clause for efficient lookup
    let placeholders: Vec<String> = (1..=blob_shas.len())
        .map(|i| format!("${}", i))
        .collect();

    let query = format!(
        "SELECT blob_sha FROM maproom.code_embeddings WHERE blob_sha IN ({})",
        placeholders.join(", ")
    );

    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = blob_shas
        .iter()
        .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();

    let rows = client
        .query(&query, &params)
        .await
        .context("Failed to batch check embedding existence")?;

    // Build set of existing blob SHAs
    let existing_blob_shas: std::collections::HashSet<String> = rows
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();

    // Step 3: Record cache hits and misses
    for blob_sha in &blob_shas {
        if existing_blob_shas.contains(blob_sha) {
            metrics.record_hit();
        } else {
            metrics.record_miss();
        }
    }

    debug!(
        total_chunks = chunks.len(),
        cache_hits = existing_blob_shas.len(),
        cache_misses = blob_shas.len() - existing_blob_shas.len(),
        "Batch cache check complete"
    );

    // Step 4: Insert all chunks
    // Convert to format expected by insert_chunks_batch
    let insert_data: Vec<_> = chunks
        .iter()
        .map(
            |(
                file_id,
                _content,
                symbol_name,
                kind,
                signature,
                docstring,
                start_line,
                end_line,
                preview,
                ts_doc_text,
                recency_score,
                churn_score,
                metadata,
            )| {
                (
                    *file_id,
                    symbol_name.clone(),
                    kind.clone(),
                    signature.clone(),
                    docstring.clone(),
                    *start_line,
                    *end_line,
                    preview.clone(),
                    ts_doc_text.clone(),
                    *recency_score,
                    *churn_score,
                    metadata.clone(),
                )
            },
        )
        .collect();

    let chunk_ids = crate::db::insert_chunks_batch(client, &insert_data)
        .await
        .context("Failed to batch insert chunks")?;

    Ok(chunk_ids)
}

/// Print cache metrics summary after scan completion.
///
/// Format matches specification from planning/architecture.md lines 457-465.
///
/// # Arguments
///
/// * `metrics` - Cache metrics to report
/// * `total_chunks` - Total number of chunks processed
pub fn log_cache_metrics(metrics: &CacheMetrics, total_chunks: usize) {
    let hits = metrics.hits();
    let misses = metrics.misses();
    let hit_rate = metrics.hit_rate() * 100.0;
    let cost = metrics.estimated_cost_usd();

    info!("Indexing complete:");
    info!("  - Chunks processed: {}", total_chunks);
    info!("  - Cache hits: {} ({:.1}%)", hits, hit_rate);
    info!(
        "  - Cache misses: {} ({:.1}%)",
        misses,
        if total_chunks > 0 {
            (misses as f64 / total_chunks as f64) * 100.0
        } else {
            0.0
        }
    );
    info!("  - Embeddings generated: {}", misses);
    info!("  - Estimated cost: ${:.4}", cost);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_blob_sha_consistency() {
        let content1 = "function foo() { return 1; }";
        let content2 = "function foo() { return 1; }";
        let content3 = "function bar() { return 2; }";

        let sha1 = compute_blob_sha(content1);
        let sha2 = compute_blob_sha(content2);
        let sha3 = compute_blob_sha(content3);

        // Same content = same SHA
        assert_eq!(sha1, sha2);

        // Different content = different SHA
        assert_ne!(sha1, sha3);

        // Valid SHA-256 hex (64 chars)
        assert_eq!(sha1.len(), 64);
        assert!(sha1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_metrics_tracking() {
        let metrics = CacheMetrics::new();

        // Initial state
        assert_eq!(metrics.hits(), 0);
        assert_eq!(metrics.misses(), 0);
        assert_eq!(metrics.hit_rate(), 0.0);

        // Record some hits and misses (80% hit rate)
        for _ in 0..8 {
            metrics.record_hit();
        }
        for _ in 0..2 {
            metrics.record_miss();
        }

        assert_eq!(metrics.hits(), 8);
        assert_eq!(metrics.misses(), 2);
        assert_eq!(metrics.hit_rate(), 0.8);
        assert_eq!(metrics.embeddings_generated(), 2);

        // Cost: 2 embeddings × $0.00002 = $0.00004
        let cost = metrics.estimated_cost_usd();
        assert!((cost - 0.00004).abs() < 0.000001);
    }
}
