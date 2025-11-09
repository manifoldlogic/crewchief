//! Parallel indexing pipeline for high-throughput code indexing.
//!
//! This module implements parallel batch processing to address the primary bottleneck
//! identified in PERF_OPT-1002: database INSERT operations consume 90-95% of indexing time.
//!
//! # Architecture
//!
//! The pipeline consists of stages connected by bounded channels:
//!
//! ```text
//! File Walk → Parse → Batch → Insert (parallel) → Complete
//!     ↓         ↓       ↓            ↓
//!  [Files]  [Chunks] [Batches]  [Database]
//! ```
//!
//! # Performance Goals
//!
//! - Parsing: Already achieves 462k files/min (PERF_OPT-1001 baseline)
//! - Database: Target 5-10x improvement through batching
//! - Memory: Bounded by channel capacity (prevent unbounded growth)
//!
//! # Configuration
//!
//! - `batch_size`: Number of chunks per database batch (50-100 recommended)
//! - `parallel_workers`: Number of concurrent database workers (4-8 recommended)
//! - `max_file_size`: Skip files larger than this (10MB default)

use anyhow::Context;
use crossbeam::channel::{self, Receiver};
use rayon::prelude::*;
use std::path::PathBuf;
use tracing::{debug, warn};

use crate::db::PgPool;

/// Configuration for parallel indexing pipeline.
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of chunks to batch per database INSERT (50-100 recommended).
    pub batch_size: usize,

    /// Number of concurrent database workers (4-8 recommended).
    pub parallel_workers: usize,

    /// Maximum file size to index (bytes). Files larger are skipped.
    pub max_file_size: usize,

    /// Channel capacity for file queue (limits memory usage).
    pub file_queue_capacity: usize,

    /// Channel capacity for chunk queue (limits memory usage).
    pub chunk_queue_capacity: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            parallel_workers: 4,
            max_file_size: 10 * 1024 * 1024, // 10MB
            file_queue_capacity: 1000,
            chunk_queue_capacity: 10000,
        }
    }
}

/// File processing task for the pipeline.
#[derive(Debug, Clone)]
pub struct FileTask {
    pub path: PathBuf,
    pub relpath: PathBuf,
    pub language: String,
    pub content: String,
    pub file_id: i64,
}

/// Chunk batch for database insertion.
#[derive(Debug)]
pub struct ChunkBatch {
    pub chunks: Vec<(
        i64,                       // file_id
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
    )>,
}

impl ChunkBatch {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            chunks: Vec::with_capacity(capacity),
        }
    }

    pub fn push(
        &mut self,
        file_id: i64,
        symbol_name: Option<String>,
        kind: String,
        signature: Option<String>,
        docstring: Option<String>,
        start_line: i32,
        end_line: i32,
        preview: String,
        ts_doc_text: String,
        recency_score: f32,
        churn_score: f32,
        metadata: Option<serde_json::Value>,
    ) {
        self.chunks.push((
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
        ));
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn is_full(&self, batch_size: usize) -> bool {
        self.chunks.len() >= batch_size
    }
}

/// Parallel indexing pipeline.
pub struct ParallelIndexer {
    config: ParallelConfig,
    pool: PgPool,
}

impl ParallelIndexer {
    pub fn new(pool: PgPool, config: ParallelConfig) -> Self {
        Self { config, pool }
    }

    /// Process files in parallel using work-stealing rayon thread pool.
    ///
    /// This uses rayon for CPU-bound parsing work and tokio for async database operations.
    /// Files are parsed in parallel, then batched and inserted concurrently.
    pub async fn process_files(&self, files: Vec<FileTask>) -> anyhow::Result<IndexingStats> {
        let config = self.config.clone();
        let pool = self.pool.clone();
        let num_files = files.len();

        // Create channels for pipeline stages
        let (chunk_tx, chunk_rx) = channel::bounded(config.chunk_queue_capacity);

        // Spawn database worker tasks
        let num_workers = config.parallel_workers;
        let batch_size = config.batch_size;

        let workers: Vec<_> = (0..num_workers)
            .map(|worker_id| {
                let pool = pool.clone();
                let chunk_rx = chunk_rx.clone();

                tokio::spawn(
                    async move { database_worker(worker_id, pool, chunk_rx, batch_size).await },
                )
            })
            .collect();

        // Drop the receiver in main thread so workers can detect completion
        drop(chunk_rx);

        // Parse files in parallel using rayon
        let chunk_tx_clone = chunk_tx.clone();
        let parse_result = std::thread::spawn(move || {
            files
                .par_iter()
                .for_each(|file_task| match parse_file_task(file_task) {
                    Ok(chunks) => {
                        for chunk_data in chunks {
                            if chunk_tx_clone.send(chunk_data).is_err() {
                                warn!("Failed to send chunk (channel closed)");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!(path = %file_task.path.display(), error = %e, "Failed to parse file");
                    }
                });
        });

        // Wait for parsing to complete
        parse_result.join().expect("Parse thread panicked");

        // Close chunk channel to signal workers
        drop(chunk_tx);

        // Wait for all workers to complete and aggregate stats
        let mut stats = IndexingStats::default();
        for worker in workers {
            match worker.await {
                Ok(Ok(worker_stats)) => {
                    stats.chunks_inserted += worker_stats.chunks_inserted;
                    stats.batches_processed += worker_stats.batches_processed;
                }
                Ok(Err(e)) => {
                    warn!(error = %e, "Worker failed");
                    stats.errors += 1;
                }
                Err(e) => {
                    warn!(error = %e, "Worker task panicked");
                    stats.errors += 1;
                }
            }
        }

        stats.files_processed = num_files;
        Ok(stats)
    }
}

/// Database worker that consumes chunks and inserts them in batches.
async fn database_worker(
    worker_id: usize,
    pool: PgPool,
    chunk_rx: Receiver<ChunkData>,
    batch_size: usize,
) -> anyhow::Result<IndexingStats> {
    let mut stats = IndexingStats::default();
    let mut current_batch = ChunkBatch::with_capacity(batch_size);

    debug!(worker_id, "Database worker started");

    loop {
        match chunk_rx.recv() {
            Ok(chunk_data) => {
                current_batch.push(
                    chunk_data.file_id,
                    chunk_data.symbol_name,
                    chunk_data.kind,
                    chunk_data.signature,
                    chunk_data.docstring,
                    chunk_data.start_line,
                    chunk_data.end_line,
                    chunk_data.preview,
                    chunk_data.ts_doc_text,
                    chunk_data.recency_score,
                    chunk_data.churn_score,
                    chunk_data.metadata,
                );

                // Insert batch when full
                if current_batch.is_full(batch_size) {
                    match insert_batch(&pool, &current_batch).await {
                        Ok(count) => {
                            stats.chunks_inserted += count;
                            stats.batches_processed += 1;
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to insert batch");
                            stats.errors += 1;
                        }
                    }
                    current_batch = ChunkBatch::with_capacity(batch_size);
                }
            }
            Err(_) => {
                // Channel closed, flush remaining chunks
                if !current_batch.is_empty() {
                    match insert_batch(&pool, &current_batch).await {
                        Ok(count) => {
                            stats.chunks_inserted += count;
                            stats.batches_processed += 1;
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to insert final batch");
                            stats.errors += 1;
                        }
                    }
                }
                break;
            }
        }
    }

    debug!(
        worker_id,
        chunks = stats.chunks_inserted,
        batches = stats.batches_processed,
        "Database worker finished"
    );
    Ok(stats)
}

/// Insert a batch of chunks using the batch insert function.
async fn insert_batch(pool: &PgPool, batch: &ChunkBatch) -> anyhow::Result<usize> {
    let client = pool
        .get()
        .await
        .context("Failed to get database connection")?;
    let chunk_ids = crate::db::insert_chunks_batch(&client, &batch.chunks).await?;
    Ok(chunk_ids.len())
}

/// Chunk data for pipeline.
#[derive(Debug, Clone)]
struct ChunkData {
    file_id: i64,
    symbol_name: Option<String>,
    kind: String,
    signature: Option<String>,
    docstring: Option<String>,
    start_line: i32,
    end_line: i32,
    preview: String,
    ts_doc_text: String,
    recency_score: f32,
    churn_score: f32,
    metadata: Option<serde_json::Value>,
}

/// Parse a file task and extract chunks.
fn parse_file_task(file_task: &FileTask) -> anyhow::Result<Vec<ChunkData>> {
    let chunks = crate::indexer::parser::extract_chunks(&file_task.content, &file_task.language);

    let mut chunk_data_vec = Vec::with_capacity(chunks.len());

    for chunk in chunks {
        let preview = first_n_lines(
            &file_task
                .content
                .split('\n')
                .skip(chunk.start_line as usize - 1)
                .take((chunk.end_line - chunk.start_line + 1) as usize)
                .collect::<Vec<&str>>()
                .join("\n"),
            40,
        );

        let ts_doc_text = build_ts_doc(
            file_task.relpath.to_string_lossy().as_ref(),
            chunk.symbol_name.as_deref(),
            chunk.signature.as_deref(),
            chunk.docstring.as_deref(),
            &preview,
        );

        chunk_data_vec.push(ChunkData {
            file_id: file_task.file_id,
            symbol_name: chunk.symbol_name,
            kind: chunk.kind,
            signature: chunk.signature,
            docstring: chunk.docstring,
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            preview,
            ts_doc_text,
            recency_score: 1.0,
            churn_score: 0.0,
            metadata: chunk.metadata,
        });
    }

    Ok(chunk_data_vec)
}

fn first_n_lines(s: &str, n: usize) -> String {
    s.lines().take(n).collect::<Vec<_>>().join("\n")
}

fn build_ts_doc(
    relpath: &str,
    symbol_name: Option<&str>,
    signature: Option<&str>,
    docstring: Option<&str>,
    preview: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(relpath.to_owned());
    if let Some(s) = symbol_name {
        parts.push(s.to_owned());
    }
    if let Some(s) = signature {
        parts.push(s.to_owned());
    }
    if let Some(s) = docstring {
        parts.push(s.to_owned());
    }
    parts.push(preview.to_owned());
    parts.join(" \n ")
}

/// Indexing statistics.
#[derive(Debug, Default, Clone)]
pub struct IndexingStats {
    pub files_processed: usize,
    pub chunks_inserted: usize,
    pub batches_processed: usize,
    pub errors: usize,
}

impl IndexingStats {
    /// Calculate average chunks per file.
    pub fn avg_chunks_per_file(&self) -> f64 {
        if self.files_processed == 0 {
            0.0
        } else {
            self.chunks_inserted as f64 / self.files_processed as f64
        }
    }

    /// Calculate average chunks per batch.
    pub fn avg_chunks_per_batch(&self) -> f64 {
        if self.batches_processed == 0 {
            0.0
        } else {
            self.chunks_inserted as f64 / self.batches_processed as f64
        }
    }
}
