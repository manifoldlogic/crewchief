//! Cache-aware chunk upsert logic using content-addressed blob SHA.
//!
//! This module provides chunk insertion with embedding deduplication:
//! - Compute blob SHA from chunk content
//! - Check if embedding exists in code_embeddings table
//! - Track cache hits/misses for cost analysis
//! - Track worktree membership via JSONB arrays (BRANCHX)
//!
//! This is the core implementation of BLOBSHA Phase 3 (planning/plan.md lines 331-439).

use crate::content_hash::compute_blob_sha;
use crate::db::traits::StoreChunks;
use crate::db::{ChunkRecord, SqliteStore};
use crate::metrics::CacheMetrics;
use anyhow::{Context, Result};
use tracing::{debug, info};

/// Check if an embedding exists for a given blob SHA.
///
/// This is the cache check operation - returns true if we can reuse an existing
/// embedding, false if we need to generate a new one.
///
/// # Arguments
///
/// * `store` - SQLite database store
/// * `blob_sha` - Content hash of the chunk
///
/// # Returns
///
/// `Ok(true)` if embedding exists (cache hit), `Ok(false)` if not (cache miss)
pub async fn check_embedding_exists(store: &SqliteStore, blob_sha: &str) -> Result<bool> {
    store
        .has_embedding(blob_sha)
        .await
        .context("Failed to check embedding existence")
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
/// * `store` - SQLite database store
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
/// * `worktree_id` - Worktree ID for this chunk
/// * `metrics` - Cache metrics tracker
///
/// # Returns
///
/// The chunk ID of the inserted/updated chunk
pub async fn upsert_chunk_with_cache(
    store: &SqliteStore,
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
    worktree_id: i64,
    metrics: &CacheMetrics,
) -> Result<i64> {
    // Step 1: Compute blob SHA from chunk content
    let blob_sha = compute_blob_sha(content);

    // Step 2: Check if embedding exists (cache check)
    let embedding_exists = check_embedding_exists(store, &blob_sha)
        .await
        .context("Failed to check embedding cache")?;

    // Step 3: Record cache hit or miss
    if embedding_exists {
        metrics.record_hit();
        debug!(
            blob_sha = %blob_sha,
            symbol = ?symbol_name,
            worktree_id = worktree_id,
            "Cache hit: reusing existing embedding"
        );
    } else {
        metrics.record_miss();
        debug!(
            blob_sha = %blob_sha,
            symbol = ?symbol_name,
            worktree_id = worktree_id,
            "Cache miss: new embedding needed"
        );
    }

    // Step 4: Insert chunk with blob_sha
    // Note: Actual embedding generation and insertion into code_embeddings
    // happens in the embedding pipeline (not in this upsert path).
    // This just records the blob_sha reference.
    let chunk = ChunkRecord {
        file_id,
        blob_sha,
        symbol_name: symbol_name.map(|s| s.to_string()),
        kind: kind.to_string(),
        signature: signature.map(|s| s.to_string()),
        docstring: docstring.map(|s| s.to_string()),
        start_line,
        end_line,
        preview: preview.to_string(),
        ts_doc_text: ts_doc_text.to_string(),
        recency_score,
        churn_score,
        metadata: metadata.cloned(),
        worktree_id,
    };

    let chunk_id = store
        .insert_chunk(&chunk)
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
/// * `store` - SQLite database store
/// * `chunks` - Vector of chunk data with content for blob SHA computation
/// * `worktree_id` - Worktree ID for these chunks
/// * `metrics` - Cache metrics tracker
///
/// # Returns
///
/// Vector of chunk IDs in the same order as input chunks
pub async fn upsert_chunks_batch_with_cache(
    store: &SqliteStore,
    chunks: &[(
        i64,                       // file_id
        String,                    // content (for blob_sha)
        Option<String>,            // symbol_name
        String,                    // kind
        Option<String>,            // signature
        Option<String>,            // docstring
        i32,                       // start_line
        i32,                       // end_line
        String,                    // preview
        String,                    // ts_doc_text
        f32,                       // recency_score
        f32,                       // churn_score
        Option<serde_json::Value>, // metadata
    )],
    worktree_id: i64,
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
    // Check each blob_sha individually (SQLite doesn't have batch checking yet)
    let mut existing_blob_shas = std::collections::HashSet::new();
    for blob_sha in &blob_shas {
        if store
            .has_embedding(blob_sha)
            .await
            .context("Failed to check embedding existence")?
        {
            existing_blob_shas.insert(blob_sha.clone());
        }
    }

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
    // Convert to ChunkRecord format expected by insert_chunks_batch
    let chunk_records: Vec<ChunkRecord> = chunks
        .iter()
        .zip(blob_shas.iter())
        .map(
            |(
                (
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
                ),
                blob_sha,
            )| ChunkRecord {
                file_id: *file_id,
                blob_sha: blob_sha.clone(),
                symbol_name: symbol_name.clone(),
                kind: kind.clone(),
                signature: signature.clone(),
                docstring: docstring.clone(),
                start_line: *start_line,
                end_line: *end_line,
                preview: preview.clone(),
                ts_doc_text: ts_doc_text.clone(),
                recency_score: *recency_score,
                churn_score: *churn_score,
                metadata: metadata.clone(),
                worktree_id,
            },
        )
        .collect();

    let chunk_ids = store
        .insert_chunks_batch(&chunk_records)
        .await
        .context("Failed to batch insert chunks")?;

    Ok(chunk_ids)
}

/// Simplified chunk representation for worktree-aware upsert.
///
/// This struct contains the minimal information needed to upsert a chunk
/// with worktree tracking. Used by the incremental update algorithm (BRANCHX-1007).
#[derive(Debug, Clone)]
pub struct ParsedChunk {
    /// Relative path to file containing this chunk
    pub relpath: String,
    /// Optional symbol name (function, class, etc.)
    pub symbol_name: Option<String>,
    /// Full chunk content (for blob SHA computation)
    pub content: String,
    /// Starting line number in file
    pub start_line: i32,
    /// Ending line number in file
    pub end_line: i32,
    /// Chunk type/kind (function, class, module, etc.)
    pub kind: String,
}

/// Upsert a chunk with worktree tracking.
///
/// This function implements the core BRANCHX upsert logic:
/// 1. Compute blob_sha from content
/// 2. Check if embedding exists for this blob_sha (cache check)
/// 3. INSERT chunk with worktree_ids = [worktree_id]
/// 4. ON CONFLICT: append worktree_id to array if not already present
/// 5. Return chunk_id (UUID)
///
/// The function is idempotent: calling twice with the same worktree_id will not
/// create duplicate entries in the worktree_ids array.
///
/// # Arguments
///
/// * `client` - Database client
/// * `chunk` - Parsed chunk data
/// * `worktree_id` - ID of worktree containing this chunk
/// * `metrics` - Cache metrics tracker (for recording cache hits/misses)
///
/// # Returns
///
/// * `Ok(Uuid)` - The chunk_id of the inserted/updated chunk
/// * `Err` - Database errors or processing errors
///
/// # Example
///
/// ```no_run
/// # use crewchief_maproom::upsert::{ParsedChunk, upsert_chunk_with_worktree};
/// # use crewchief_maproom::metrics::CacheMetrics;
/// # use crewchief_maproom::db;
/// # async fn example() -> anyhow::Result<()> {
/// let store = db::connect().await?;
/// let metrics = CacheMetrics::new();
///
/// let chunk = ParsedChunk {
///     relpath: "src/main.rs".to_string(),
///     symbol_name: Some("main".to_string()),
///     content: "fn main() { println!(\"Hello\"); }".to_string(),
///     start_line: 1,
///     end_line: 3,
///     kind: "function".to_string(),
/// };
///
/// let chunk_id = upsert_chunk_with_worktree(&store, &chunk, 1, &metrics).await?;
/// println!("Chunk ID: {}", chunk_id);
/// # Ok(())
/// # }
/// ```
pub async fn upsert_chunk_with_worktree(
    store: &SqliteStore,
    chunk: &ParsedChunk,
    worktree_id: i64,
    metrics: &CacheMetrics,
) -> Result<i64> {
    // Step 1: Compute blob SHA from chunk content
    let blob_sha = compute_blob_sha(&chunk.content);

    // Step 2: Check if embedding exists (cache check)
    let embedding_exists = check_embedding_exists(store, &blob_sha)
        .await
        .context("Failed to check embedding cache")?;

    // Step 3: Record cache hit or miss
    if embedding_exists {
        metrics.record_hit();
        debug!(
            blob_sha = %blob_sha,
            symbol = ?chunk.symbol_name,
            worktree_id = worktree_id,
            "Cache hit: reusing existing embedding"
        );
    } else {
        metrics.record_miss();
        debug!(
            blob_sha = %blob_sha,
            symbol = ?chunk.symbol_name,
            worktree_id = worktree_id,
            "Cache miss: new embedding needed"
        );
    }

    // Step 4: Upsert chunk with worktree tracking
    // SQLite version: Insert chunk and use add_chunk_to_worktree for tracking
    // Note: This is a simplified implementation - the original PostgreSQL version
    // used JSONB arrays and complex ON CONFLICT logic. For SQLite, we use the
    // chunk_worktrees junction table instead.

    // First, create a ChunkRecord (note: we need a file_id, which we don't have here)
    // This is a stub implementation - in practice, the caller should provide file_id
    // or we need to look it up from the relpath
    // For now, we'll return an error indicating this needs to be implemented
    anyhow::bail!(
        "upsert_chunk_with_worktree is not yet fully implemented for SQLite. \
         Use insert_chunk with a ChunkRecord that includes file_id instead."
    );
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

    #[test]
    fn test_parsed_chunk_creation() {
        let chunk = ParsedChunk {
            relpath: "src/main.rs".to_string(),
            symbol_name: Some("main".to_string()),
            content: "fn main() {}".to_string(),
            start_line: 1,
            end_line: 1,
            kind: "function".to_string(),
        };

        assert_eq!(chunk.relpath, "src/main.rs");
        assert_eq!(chunk.symbol_name, Some("main".to_string()));
        assert_eq!(chunk.content, "fn main() {}");
        assert_eq!(chunk.start_line, 1);
        assert_eq!(chunk.end_line, 1);
        assert_eq!(chunk.kind, "function");
    }

    #[test]
    fn test_parsed_chunk_clone() {
        let chunk = ParsedChunk {
            relpath: "src/lib.rs".to_string(),
            symbol_name: None,
            content: "mod test;".to_string(),
            start_line: 5,
            end_line: 5,
            kind: "module".to_string(),
        };

        let cloned = chunk.clone();
        assert_eq!(cloned.relpath, chunk.relpath);
        assert_eq!(cloned.symbol_name, chunk.symbol_name);
        assert_eq!(cloned.content, chunk.content);
        assert_eq!(cloned.start_line, chunk.start_line);
        assert_eq!(cloned.end_line, chunk.end_line);
        assert_eq!(cloned.kind, chunk.kind);
    }

    #[test]
    fn test_parsed_chunk_debug() {
        let chunk = ParsedChunk {
            relpath: "test.rs".to_string(),
            symbol_name: Some("test_fn".to_string()),
            content: "test content".to_string(),
            start_line: 10,
            end_line: 20,
            kind: "function".to_string(),
        };

        let debug_str = format!("{:?}", chunk);
        assert!(debug_str.contains("ParsedChunk"));
        assert!(debug_str.contains("test.rs"));
        assert!(debug_str.contains("test_fn"));
    }
}
