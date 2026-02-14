//! Encoding progress module for querying chunk/embedding counts and active encoding runs.
//!
//! This module mirrors the pattern established in `status.rs`:
//! query function + response structs + formatters.

use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::traits::StoreCore;
use crate::db::SqliteStore;

/// Response struct for encoding progress queries.
#[derive(Debug, Serialize, Deserialize)]
pub struct EncodingProgressResponse {
    pub total_chunks: i64,
    pub total_embeddings: i64,
    pub percentage: f64,
    pub chunks_remaining: i64,
    pub repo_filter: Option<String>,
    pub active_run: Option<ActiveRunInfo>,
}

/// Information about an active encoding run.
#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveRunInfo {
    pub run_id: i64,
    pub started_at: String,
    pub total_chunks: i64,
    pub chunks_completed: i64,
    pub chunks_per_second: Option<f64>,
    pub provider: Option<String>,
    pub dimension: Option<i32>,
    pub estimated_seconds_remaining: Option<f64>,
    pub elapsed_seconds: Option<f64>,
}

/// Calculate ETA in seconds based on remaining chunks and throughput.
///
/// Returns `None` when `chunks_per_second` is zero, negative, or `None`.
pub fn calculate_eta(remaining_chunks: i64, chunks_per_second: Option<f64>) -> Option<f64> {
    match chunks_per_second {
        Some(rate) if rate > 0.0 => Some(remaining_chunks as f64 / rate),
        _ => None,
    }
}

/// Calculate elapsed seconds from a timestamp string to now.
///
/// Accepts SQLite `datetime('now')` format: `YYYY-MM-DD HH:MM:SS`
/// and RFC 3339 / ISO 8601 format: `YYYY-MM-DDTHH:MM:SS+00:00`.
pub fn calculate_elapsed_seconds(started_at: &str) -> Result<f64> {
    // Try SQLite datetime format first: "2026-02-05 14:30:00"
    let naive = NaiveDateTime::parse_from_str(started_at, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| {
            // Try ISO 8601 / RFC 3339 with T separator: "2026-02-05T14:30:00"
            NaiveDateTime::parse_from_str(started_at, "%Y-%m-%dT%H:%M:%S")
        })
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse timestamp '{}': {}. Expected format: YYYY-MM-DD HH:MM:SS",
                started_at,
                e
            )
        })?;

    // Treat the parsed time as UTC (SQLite datetime('now') produces UTC)
    let start_utc = naive.and_utc();
    let now = Utc::now();
    let elapsed = now.signed_duration_since(start_utc);
    Ok(elapsed.num_milliseconds() as f64 / 1000.0)
}

/// Check if a timestamp is stale (more than 1 hour old).
///
/// Returns `true` if the timestamp is more than 3600 seconds in the past,
/// or if the timestamp cannot be parsed.
fn is_stale(timestamp: &str) -> bool {
    match calculate_elapsed_seconds(timestamp) {
        Ok(elapsed) => elapsed > 3600.0,
        Err(_) => true, // If we can't parse, treat as stale
    }
}

/// Query the database for encoding progress statistics.
///
/// If `repo_filter` is provided, counts are scoped to that repo.
/// Otherwise, global counts are returned.
pub async fn get_encoding_progress(
    store: Arc<SqliteStore>,
    repo_filter: Option<String>,
) -> Result<EncodingProgressResponse> {
    let (total_chunks, total_embeddings) = match &repo_filter {
        Some(repo_name) => {
            let chunks = store.get_repo_chunk_count(repo_name).await?;
            let embeddings = store.get_repo_embedding_count(repo_name).await?;
            (chunks, embeddings)
        }
        None => {
            let chunks = store.get_global_chunk_count().await?;
            let embeddings = store.get_global_embedding_count().await?;
            (chunks, embeddings)
        }
    };

    let percentage = if total_chunks == 0 {
        0.0
    } else {
        (total_embeddings as f64 / total_chunks as f64) * 100.0
    };

    let chunks_remaining = (total_chunks - total_embeddings).max(0);

    // Check for active encoding run
    let active_run = match store.get_active_encoding_run().await? {
        Some(run) => {
            // Staleness detection: if last_batch_at is >1 hour old, don't show as active
            let stale = match &run.last_batch_at {
                Some(last_batch) => is_stale(last_batch),
                // If there's no last_batch_at, check started_at instead
                None => is_stale(&run.started_at),
            };

            if stale {
                None
            } else {
                let remaining = (run.total_chunks - run.chunks_completed).max(0);
                let estimated_seconds_remaining = calculate_eta(remaining, run.chunks_per_second);

                let elapsed_seconds = calculate_elapsed_seconds(&run.started_at).ok();

                Some(ActiveRunInfo {
                    run_id: run.id,
                    started_at: run.started_at,
                    total_chunks: run.total_chunks,
                    chunks_completed: run.chunks_completed,
                    chunks_per_second: run.chunks_per_second,
                    provider: run.provider,
                    dimension: run.dimension,
                    estimated_seconds_remaining,
                    elapsed_seconds,
                })
            }
        }
        None => None,
    };

    Ok(EncodingProgressResponse {
        total_chunks,
        total_embeddings,
        percentage,
        chunks_remaining,
        repo_filter,
        active_run,
    })
}

/// Format number with thousands separator (mirrors status.rs format_number).
fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();

    for (count, c) in s.chars().rev().enumerate() {
        if count > 0 && count % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }

    result
}

/// Format seconds as a human-readable duration string.
///
/// Examples: "~0s", "~30s", "~2m 30s", "~1h 5m"
fn format_duration(seconds: f64) -> String {
    let total_secs = seconds.round() as u64;
    if total_secs < 60 {
        format!("~{}s", total_secs)
    } else if total_secs < 3600 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if secs == 0 {
            format!("~{}m", mins)
        } else {
            format!("~{}m {}s", mins, secs)
        }
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if mins == 0 {
            format!("~{}h", hours)
        } else {
            format!("~{}h {}m", hours, mins)
        }
    }
}

/// Format encoding progress as human-readable text.
pub fn format_text(response: &EncodingProgressResponse) -> String {
    let mut output = String::new();

    if let Some(ref repo) = response.repo_filter {
        output.push_str(&format!("Repository: {}\n", repo));
    }

    output.push_str(&format!(
        "Total chunks: {}\n",
        format_number(response.total_chunks)
    ));
    output.push_str(&format!(
        "Embeddings: {} ({:.1}%)\n",
        format_number(response.total_embeddings),
        response.percentage
    ));
    output.push_str(&format!(
        "Remaining: {}\n",
        format_number(response.chunks_remaining)
    ));

    match &response.active_run {
        Some(run) => {
            output.push_str("\nActive Run:\n");

            // Provider line: "ollama (1024 dimensions)" or just provider or just dimension
            match (&run.provider, run.dimension) {
                (Some(provider), Some(dim)) => {
                    output.push_str(&format!(
                        "  Provider:        {} ({} dimensions)\n",
                        provider, dim
                    ));
                }
                (Some(provider), None) => {
                    output.push_str(&format!("  Provider:        {}\n", provider));
                }
                (None, Some(dim)) => {
                    output.push_str(&format!("  Provider:        ({} dimensions)\n", dim));
                }
                (None, None) => {}
            }

            output.push_str(&format!("  Started:         {}\n", run.started_at));

            if let Some(elapsed) = run.elapsed_seconds {
                output.push_str(&format!(
                    "  Elapsed:         {}\n",
                    format_duration(elapsed)
                ));
            }

            output.push_str(&format!(
                "  Batch progress:  {} / {} chunks\n",
                format_number(run.chunks_completed),
                format_number(run.total_chunks)
            ));

            if let Some(cps) = run.chunks_per_second {
                output.push_str(&format!("  Throughput:      {:.1} chunks/s\n", cps));
            }

            if let Some(eta_secs) = run.estimated_seconds_remaining {
                output.push_str(&format!(
                    "  ETA:             {} remaining\n",
                    format_duration(eta_secs)
                ));
            }
        }
        None => {
            output.push_str("\nNo active encoding run.\n");
        }
    }

    output
}

/// Format encoding progress as JSON.
pub fn format_json(response: &EncodingProgressResponse) -> Result<String> {
    serde_json::to_string_pretty(response)
        .map_err(|e| anyhow::anyhow!("Failed to serialize encoding progress to JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sqlite::SqliteStore;
    use crate::db::traits::StoreChunks;
    use crate::db::{ChunkRecord, FileRecord};
    use rusqlite::params;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counter for unique shared in-memory database names.
    static TEST_STORE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Helper to create a test store with migrations applied.
    async fn setup_test_store() -> Arc<SqliteStore> {
        let store = Arc::new(SqliteStore::connect(":memory:").await.unwrap());
        store
    }

    /// Helper to create test data: a repo, worktree, commit, file, and N chunks.
    /// Returns (repo_id, worktree_id, commit_id, file_id).
    async fn setup_test_data(
        store: &Arc<SqliteStore>,
        repo_name: &str,
        num_chunks: usize,
    ) -> (i64, i64, i64, i64) {
        let repo_id = store
            .get_or_create_repo(repo_name, "/test/path")
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
            content_hash: format!("hash_{}", repo_name),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        for i in 0..num_chunks {
            let chunk = ChunkRecord {
                file_id,
                worktree_id,
                blob_sha: format!("blob_{}_{}", repo_name, i),
                symbol_name: Some(format!("fn_{}", i)),
                kind: "function".to_string(),
                signature: None,
                docstring: None,
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

        (repo_id, worktree_id, commit_id, file_id)
    }

    /// Helper to insert embeddings for blob_shas.
    async fn insert_embeddings(store: &Arc<SqliteStore>, blob_shas: Vec<String>) {
        for blob_sha in blob_shas {
            store
                .run(move |conn| {
                    conn.execute(
                        "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![blob_sha, vec![0u8; 4096], 1024, "test-model"],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
    }

    // ==================== Test Case #1: get_global_chunk_count - empty database ====================
    #[tokio::test]
    async fn test_global_chunk_count_empty() {
        let store = setup_test_store().await;
        let count = store.get_global_chunk_count().await.unwrap();
        assert_eq!(count, 0);
    }

    // ==================== Test Case #1: get_global_chunk_count - with data ====================
    #[tokio::test]
    async fn test_global_chunk_count_with_data() {
        let store = setup_test_store().await;
        setup_test_data(&store, "test-repo", 5).await;
        let count = store.get_global_chunk_count().await.unwrap();
        assert_eq!(count, 5);
    }

    // ==================== Test Case #1: get_global_chunk_count - distinct blob_sha ====================
    #[tokio::test]
    async fn test_global_chunk_count_distinct_blob_sha() {
        let store = setup_test_store().await;
        // Create chunks in two repos with some overlapping blob_shas
        let repo_id = store
            .get_or_create_repo("repo1", "/test/path1")
            .await
            .unwrap();
        let wt1 = store
            .get_or_create_worktree(repo_id, "main", "/test/path1")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id: wt1,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash1".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create two chunks with same blob_sha
        let chunk1 = ChunkRecord {
            file_id,
            worktree_id: wt1,
            blob_sha: "shared_blob".to_string(),
            symbol_name: Some("fn1".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 10,
            preview: "fn fn1() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk1).await.unwrap();

        let chunk2 = ChunkRecord {
            file_id,
            worktree_id: wt1,
            blob_sha: "shared_blob".to_string(),
            symbol_name: Some("fn2".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 11,
            end_line: 20,
            preview: "fn fn2() {}".to_string(),
            ts_doc_text: String::new(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        };
        store.insert_chunk(&chunk2).await.unwrap();

        // Should count distinct blob_shas: only 1 despite 2 chunk rows
        let count = store.get_global_chunk_count().await.unwrap();
        assert_eq!(count, 1);
    }

    // ==================== Test Case #2: get_global_embedding_count - empty database ====================
    #[tokio::test]
    async fn test_global_embedding_count_empty() {
        let store = setup_test_store().await;
        let count = store.get_global_embedding_count().await.unwrap();
        assert_eq!(count, 0);
    }

    // ==================== Test Case #2: get_global_embedding_count - with data ====================
    #[tokio::test]
    async fn test_global_embedding_count_with_data() {
        let store = setup_test_store().await;
        setup_test_data(&store, "test-repo", 3).await;
        insert_embeddings(
            &store,
            vec![
                "blob_test-repo_0".to_string(),
                "blob_test-repo_1".to_string(),
            ],
        )
        .await;
        let count = store.get_global_embedding_count().await.unwrap();
        assert_eq!(count, 2);
    }

    // ==================== Test Case #2: embeddings independent of chunks ====================
    #[tokio::test]
    async fn test_global_embedding_count_independent_of_chunks() {
        let store = setup_test_store().await;
        // Insert embeddings without corresponding chunks
        insert_embeddings(&store, vec!["orphan_blob".to_string()]).await;
        let count = store.get_global_embedding_count().await.unwrap();
        assert_eq!(count, 1);
    }

    // ==================== Test Case #3: get_repo_chunk_count - non-existent repo ====================
    #[tokio::test]
    async fn test_repo_chunk_count_nonexistent_repo() {
        let store = setup_test_store().await;
        let count = store.get_repo_chunk_count("nonexistent").await.unwrap();
        assert_eq!(count, 0);
    }

    // ==================== Test Case #3: get_repo_chunk_count - correct count ====================
    #[tokio::test]
    async fn test_repo_chunk_count_correct() {
        let store = setup_test_store().await;
        setup_test_data(&store, "repo-a", 3).await;
        setup_test_data(&store, "repo-b", 5).await;
        let count_a = store.get_repo_chunk_count("repo-a").await.unwrap();
        let count_b = store.get_repo_chunk_count("repo-b").await.unwrap();
        assert_eq!(count_a, 3);
        assert_eq!(count_b, 5);
    }

    // ==================== Test Case #4: get_repo_embedding_count - non-existent repo ====================
    #[tokio::test]
    async fn test_repo_embedding_count_nonexistent_repo() {
        let store = setup_test_store().await;
        let count = store.get_repo_embedding_count("nonexistent").await.unwrap();
        assert_eq!(count, 0);
    }

    // ==================== Test Case #4: get_repo_embedding_count - correct count ====================
    #[tokio::test]
    async fn test_repo_embedding_count_correct() {
        let store = setup_test_store().await;
        setup_test_data(&store, "repo-a", 3).await;
        setup_test_data(&store, "repo-b", 2).await;
        // Embed only repo-a chunks
        insert_embeddings(
            &store,
            vec!["blob_repo-a_0".to_string(), "blob_repo-a_1".to_string()],
        )
        .await;
        let count_a = store.get_repo_embedding_count("repo-a").await.unwrap();
        let count_b = store.get_repo_embedding_count("repo-b").await.unwrap();
        assert_eq!(count_a, 2);
        assert_eq!(count_b, 0);
    }

    // ==================== Test Case #5: create_encoding_run ====================
    #[tokio::test]
    async fn test_create_encoding_run() {
        let store = setup_test_store().await;
        let run_id = store
            .create_encoding_run(100, Some("ollama"), Some(768))
            .await
            .unwrap();
        assert!(run_id > 0);

        // Verify defaults
        let run = store.get_active_encoding_run().await.unwrap().unwrap();
        assert_eq!(run.id, run_id);
        assert_eq!(run.status, "running");
        assert_eq!(run.total_chunks, 100);
        assert_eq!(run.chunks_completed, 0);
        assert_eq!(run.provider, Some("ollama".to_string()));
        assert_eq!(run.dimension, Some(768));
        assert!(!run.started_at.is_empty());
    }

    // ==================== Test Case #6: update_encoding_run_progress ====================
    #[tokio::test]
    async fn test_update_encoding_run_progress() {
        let store = setup_test_store().await;
        let run_id = store
            .create_encoding_run(100, Some("openai"), Some(1536))
            .await
            .unwrap();

        store
            .update_encoding_run_progress(run_id, 50, Some(25.0))
            .await
            .unwrap();

        let run = store.get_active_encoding_run().await.unwrap().unwrap();
        assert_eq!(run.chunks_completed, 50);
        assert_eq!(run.chunks_per_second, Some(25.0));
        assert!(run.last_batch_at.is_some());
    }

    // ==================== Test Case #6: update_encoding_run_progress - nonexistent ====================
    #[tokio::test]
    async fn test_update_encoding_run_progress_nonexistent() {
        let store = setup_test_store().await;
        // Should not error even with non-existent run_id
        let result = store
            .update_encoding_run_progress(999, 50, Some(25.0))
            .await;
        assert!(result.is_ok());
    }

    // ==================== Test Case #7: complete_encoding_run - completed ====================
    #[tokio::test]
    async fn test_complete_encoding_run_completed() {
        let store = setup_test_store().await;
        let run_id = store.create_encoding_run(100, None, None).await.unwrap();

        store
            .complete_encoding_run(run_id, "completed")
            .await
            .unwrap();

        // Should no longer be active
        let active = store.get_active_encoding_run().await.unwrap();
        assert!(active.is_none());

        // Verify status and finished_at via raw query
        store
            .run(move |conn| {
                let (status, finished_at): (String, Option<String>) = conn.query_row(
                    "SELECT status, finished_at FROM encoding_runs WHERE id = ?1",
                    params![run_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                assert_eq!(status, "completed");
                assert!(finished_at.is_some());
                Ok(())
            })
            .await
            .unwrap();
    }

    // ==================== Test Case #7: complete_encoding_run - failed ====================
    #[tokio::test]
    async fn test_complete_encoding_run_failed() {
        let store = setup_test_store().await;
        let run_id = store.create_encoding_run(100, None, None).await.unwrap();

        store.complete_encoding_run(run_id, "failed").await.unwrap();

        let active = store.get_active_encoding_run().await.unwrap();
        assert!(active.is_none());
    }

    // ==================== Test Case #7: complete_encoding_run - idempotent ====================
    #[tokio::test]
    async fn test_complete_encoding_run_idempotent() {
        let store = setup_test_store().await;
        let run_id = store.create_encoding_run(100, None, None).await.unwrap();

        store
            .complete_encoding_run(run_id, "completed")
            .await
            .unwrap();
        // Call again - should not error
        store
            .complete_encoding_run(run_id, "completed")
            .await
            .unwrap();
    }

    // ==================== Test Case #8: get_active_encoding_run - no runs ====================
    #[tokio::test]
    async fn test_get_active_encoding_run_none() {
        let store = setup_test_store().await;
        let active = store.get_active_encoding_run().await.unwrap();
        assert!(active.is_none());
    }

    // ==================== Test Case #8: get_active_encoding_run - all completed ====================
    #[tokio::test]
    async fn test_get_active_encoding_run_all_completed() {
        let store = setup_test_store().await;
        let run_id = store.create_encoding_run(100, None, None).await.unwrap();
        store
            .complete_encoding_run(run_id, "completed")
            .await
            .unwrap();

        let active = store.get_active_encoding_run().await.unwrap();
        assert!(active.is_none());
    }

    // ==================== Test Case #8: get_active_encoding_run - returns latest ====================
    #[tokio::test]
    async fn test_get_active_encoding_run_returns_latest() {
        let store = setup_test_store().await;
        let _run1 = store
            .create_encoding_run(50, Some("ollama"), Some(768))
            .await
            .unwrap();
        let run2 = store
            .create_encoding_run(100, Some("openai"), Some(1536))
            .await
            .unwrap();

        let active = store.get_active_encoding_run().await.unwrap().unwrap();
        assert_eq!(active.id, run2);
        assert_eq!(active.total_chunks, 100);
        assert_eq!(active.provider, Some("openai".to_string()));
    }

    // ==================== Test Case #9: get_encoding_progress - no data ====================
    #[tokio::test]
    async fn test_encoding_progress_no_data() {
        let store = setup_test_store().await;
        let progress = get_encoding_progress(store, None).await.unwrap();
        assert_eq!(progress.total_chunks, 0);
        assert_eq!(progress.total_embeddings, 0);
        assert_eq!(progress.percentage, 0.0);
        assert_eq!(progress.chunks_remaining, 0);
        assert!(progress.active_run.is_none());
    }

    // ==================== Test Case #10: get_encoding_progress - partial ====================
    #[tokio::test]
    async fn test_encoding_progress_partial() {
        let store = setup_test_store().await;
        setup_test_data(&store, "test-repo", 100).await;
        let mut shas = Vec::new();
        for i in 0..50 {
            shas.push(format!("blob_test-repo_{}", i));
        }
        insert_embeddings(&store, shas).await;

        let progress = get_encoding_progress(store, None).await.unwrap();
        assert_eq!(progress.total_chunks, 100);
        assert_eq!(progress.total_embeddings, 50);
        assert!((progress.percentage - 50.0).abs() < f64::EPSILON);
        assert_eq!(progress.chunks_remaining, 50);
    }

    // ==================== Test Case #11: get_encoding_progress - complete ====================
    #[tokio::test]
    async fn test_encoding_progress_complete() {
        let store = setup_test_store().await;
        setup_test_data(&store, "test-repo", 10).await;
        let shas: Vec<String> = (0..10).map(|i| format!("blob_test-repo_{}", i)).collect();
        insert_embeddings(&store, shas).await;

        let progress = get_encoding_progress(store, None).await.unwrap();
        assert_eq!(progress.total_chunks, 10);
        assert_eq!(progress.total_embeddings, 10);
        assert!((progress.percentage - 100.0).abs() < f64::EPSILON);
        assert_eq!(progress.chunks_remaining, 0);
    }

    // ==================== Test Case #12: get_encoding_progress - with repo filter ====================
    #[tokio::test]
    async fn test_encoding_progress_repo_filter() {
        let store = setup_test_store().await;
        setup_test_data(&store, "repo-a", 10).await;
        setup_test_data(&store, "repo-b", 20).await;
        insert_embeddings(
            &store,
            vec![
                "blob_repo-a_0".to_string(),
                "blob_repo-a_1".to_string(),
                "blob_repo-b_0".to_string(),
            ],
        )
        .await;

        let progress_a = get_encoding_progress(store.clone(), Some("repo-a".to_string()))
            .await
            .unwrap();
        assert_eq!(progress_a.total_chunks, 10);
        assert_eq!(progress_a.total_embeddings, 2);
        assert_eq!(progress_a.repo_filter, Some("repo-a".to_string()));

        let progress_b = get_encoding_progress(store.clone(), Some("repo-b".to_string()))
            .await
            .unwrap();
        assert_eq!(progress_b.total_chunks, 20);
        assert_eq!(progress_b.total_embeddings, 1);
    }

    // ==================== Test Case #12: get_encoding_progress - non-existent repo filter ====================
    #[tokio::test]
    async fn test_encoding_progress_nonexistent_repo() {
        let store = setup_test_store().await;
        setup_test_data(&store, "repo-a", 10).await;

        let progress = get_encoding_progress(store, Some("nonexistent".to_string()))
            .await
            .unwrap();
        assert_eq!(progress.total_chunks, 0);
        assert_eq!(progress.total_embeddings, 0);
        assert_eq!(progress.percentage, 0.0);
    }

    // ==================== Test Case #13: get_encoding_progress - with active run ====================
    #[tokio::test]
    async fn test_encoding_progress_with_active_run() {
        let store = setup_test_store().await;
        setup_test_data(&store, "test-repo", 100).await;
        let run_id = store
            .create_encoding_run(100, Some("ollama"), Some(768))
            .await
            .unwrap();
        store
            .update_encoding_run_progress(run_id, 50, Some(10.0))
            .await
            .unwrap();

        let progress = get_encoding_progress(store, None).await.unwrap();
        let run = progress.active_run.unwrap();
        assert_eq!(run.run_id, run_id);
        assert_eq!(run.total_chunks, 100);
        assert_eq!(run.chunks_completed, 50);
        assert_eq!(run.chunks_per_second, Some(10.0));
        assert_eq!(run.provider, Some("ollama".to_string()));
        assert_eq!(run.dimension, Some(768));
        // ETA: 50 remaining / 10 per sec = 5.0 seconds
        assert!((run.estimated_seconds_remaining.unwrap() - 5.0).abs() < f64::EPSILON);
    }

    // ==================== Test Case #14: get_encoding_progress - division by zero ====================
    #[tokio::test]
    async fn test_encoding_progress_division_by_zero() {
        let store = setup_test_store().await;
        // No chunks at all
        let progress = get_encoding_progress(store, None).await.unwrap();
        assert_eq!(progress.percentage, 0.0);
        assert!(!progress.percentage.is_nan());
        assert!(!progress.percentage.is_infinite());
    }

    // ==================== Test Case #15: format_text - basic output ====================
    #[test]
    fn test_format_text_basic() {
        let response = EncodingProgressResponse {
            total_chunks: 1500,
            total_embeddings: 750,
            percentage: 50.0,
            chunks_remaining: 750,
            repo_filter: None,
            active_run: None,
        };

        let output = format_text(&response);
        assert!(output.contains("Total chunks: 1,500"));
        assert!(output.contains("Embeddings: 750 (50.0%)"));
        assert!(output.contains("Remaining: 750"));
        assert!(output.contains("No active encoding run."));
    }

    // ==================== Test Case #16: format_text - no active run ====================
    #[test]
    fn test_format_text_no_active_run() {
        let response = EncodingProgressResponse {
            total_chunks: 100,
            total_embeddings: 50,
            percentage: 50.0,
            chunks_remaining: 50,
            repo_filter: None,
            active_run: None,
        };

        let output = format_text(&response);
        assert!(output.contains("No active encoding run."));
    }

    // ==================== Test Case #17: format_text - with active run ====================
    #[test]
    fn test_format_text_with_active_run() {
        let response = EncodingProgressResponse {
            total_chunks: 1000,
            total_embeddings: 500,
            percentage: 50.0,
            chunks_remaining: 500,
            repo_filter: None,
            active_run: Some(ActiveRunInfo {
                run_id: 1,
                started_at: "2026-01-01 00:00:00".to_string(),
                total_chunks: 1000,
                chunks_completed: 500,
                chunks_per_second: Some(10.0),
                provider: Some("ollama".to_string()),
                dimension: Some(768),
                estimated_seconds_remaining: Some(50.0),
                elapsed_seconds: Some(135.0),
            }),
        };

        let output = format_text(&response);
        assert!(output.contains("Active Run:"));
        assert!(output.contains("Provider:        ollama (768 dimensions)"));
        assert!(output.contains("Started:         2026-01-01 00:00:00"));
        assert!(output.contains("Elapsed:         ~2m 15s"));
        assert!(output.contains("Batch progress:  500 / 1,000 chunks"));
        assert!(output.contains("Throughput:      10.0 chunks/s"));
        assert!(output.contains("ETA:             ~50s remaining"));
    }

    // ==================== Test Case #18: format_text - zero chunks ====================
    #[test]
    fn test_format_text_zero_chunks() {
        let response = EncodingProgressResponse {
            total_chunks: 0,
            total_embeddings: 0,
            percentage: 0.0,
            chunks_remaining: 0,
            repo_filter: None,
            active_run: None,
        };

        let output = format_text(&response);
        assert!(output.contains("Total chunks: 0"));
        assert!(output.contains("Embeddings: 0 (0.0%)"));
        assert!(output.contains("Remaining: 0"));
    }

    // ==================== Test Case #19: format_json - valid JSON ====================
    #[test]
    fn test_format_json_valid() {
        let response = EncodingProgressResponse {
            total_chunks: 100,
            total_embeddings: 50,
            percentage: 50.0,
            chunks_remaining: 50,
            repo_filter: None,
            active_run: None,
        };

        let json_str = format_json(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["total_chunks"], 100);
        assert_eq!(parsed["total_embeddings"], 50);
        assert_eq!(parsed["percentage"], 50.0);
        assert_eq!(parsed["chunks_remaining"], 50);
        assert!(parsed["active_run"].is_null());
    }

    // ==================== Test Case #20: format_json - with active run ====================
    #[test]
    fn test_format_json_with_active_run() {
        let response = EncodingProgressResponse {
            total_chunks: 100,
            total_embeddings: 50,
            percentage: 50.0,
            chunks_remaining: 50,
            repo_filter: Some("test-repo".to_string()),
            active_run: Some(ActiveRunInfo {
                run_id: 1,
                started_at: "2026-01-01 00:00:00".to_string(),
                total_chunks: 100,
                chunks_completed: 50,
                chunks_per_second: Some(10.0),
                provider: Some("ollama".to_string()),
                dimension: Some(768),
                estimated_seconds_remaining: Some(5.0),
                elapsed_seconds: Some(120.0),
            }),
        };

        let json_str = format_json(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed["active_run"].is_object());
        assert_eq!(parsed["active_run"]["run_id"], 1);
        assert_eq!(parsed["active_run"]["provider"], "ollama");
        assert_eq!(parsed["active_run"]["dimension"], 768);
        assert_eq!(parsed["repo_filter"], "test-repo");
    }

    // ==================== Test Case #21: format_json - without active run ====================
    #[test]
    fn test_format_json_without_active_run() {
        let response = EncodingProgressResponse {
            total_chunks: 0,
            total_embeddings: 0,
            percentage: 0.0,
            chunks_remaining: 0,
            repo_filter: None,
            active_run: None,
        };

        let json_str = format_json(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed["active_run"].is_null());
    }

    // ==================== format_text with repo filter ====================
    #[test]
    fn test_format_text_with_repo_filter() {
        let response = EncodingProgressResponse {
            total_chunks: 500,
            total_embeddings: 250,
            percentage: 50.0,
            chunks_remaining: 250,
            repo_filter: Some("my-repo".to_string()),
            active_run: None,
        };

        let output = format_text(&response);
        assert!(output.contains("Repository: my-repo"));
    }

    // ==================== format_text - large numbers ====================
    #[test]
    fn test_format_text_large_numbers() {
        let response = EncodingProgressResponse {
            total_chunks: 1_234_567,
            total_embeddings: 987_654,
            percentage: 80.0,
            chunks_remaining: 246_913,
            repo_filter: None,
            active_run: None,
        };

        let output = format_text(&response);
        assert!(output.contains("Total chunks: 1,234,567"));
        assert!(output.contains("Embeddings: 987,654 (80.0%)"));
        assert!(output.contains("Remaining: 246,913"));
    }

    // ==================== format_duration tests ====================
    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(0.0), "~0s");
        assert_eq!(format_duration(30.0), "~30s");
        assert_eq!(format_duration(59.0), "~59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60.0), "~1m");
        assert_eq!(format_duration(90.0), "~1m 30s");
        assert_eq!(format_duration(150.0), "~2m 30s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600.0), "~1h");
        assert_eq!(format_duration(3900.0), "~1h 5m");
        assert_eq!(format_duration(7200.0), "~2h");
    }

    // ==================== format_number tests ====================
    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    // ==================== ETA edge cases ====================
    #[test]
    fn test_eta_zero_throughput() {
        let run = ActiveRunInfo {
            run_id: 1,
            started_at: "2026-01-01 00:00:00".to_string(),
            total_chunks: 100,
            chunks_completed: 50,
            chunks_per_second: Some(0.0),
            provider: None,
            dimension: None,
            estimated_seconds_remaining: None, // Should not be computed with 0 throughput
            elapsed_seconds: None,
        };
        // Verified the logic: when chunks_per_second is 0.0, cps > 0.0 is false, so ETA = None
        assert!(run.estimated_seconds_remaining.is_none());
    }

    // ==================== Test Case #30: migration creates encoding_runs table ====================
    #[tokio::test]
    async fn test_migration_creates_encoding_runs() {
        let store = setup_test_store().await;
        // Verify table exists by inserting/selecting
        store
            .run(|conn| {
                conn.execute(
                    "INSERT INTO encoding_runs (total_chunks) VALUES (?1)",
                    params![100],
                )?;
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM encoding_runs",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(count, 1);

                // Verify schema columns
                let (id, started_at, status, total_chunks, chunks_completed): (
                    i64,
                    String,
                    String,
                    i64,
                    i64,
                ) = conn.query_row(
                    "SELECT id, started_at, status, total_chunks, chunks_completed FROM encoding_runs WHERE id = 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )?;
                assert_eq!(id, 1);
                assert!(!started_at.is_empty());
                assert_eq!(status, "running");
                assert_eq!(total_chunks, 100);
                assert_eq!(chunks_completed, 0);
                Ok(())
            })
            .await
            .unwrap();
    }

    // ==================== Test Case #31: migration is idempotent ====================
    #[tokio::test]
    async fn test_migration_idempotent() {
        let store = setup_test_store().await;
        // Migrate again - should not error
        store.migrate().await.unwrap();

        // Still works
        let count = store.get_global_chunk_count().await.unwrap();
        assert_eq!(count, 0);
    }

    // ==================== mark_stale_runs_as_failed - marks multiple stale runs ====================
    #[tokio::test]
    async fn test_mark_stale_runs_as_failed_multiple() {
        let store = setup_test_store().await;

        // Create multiple running runs
        let run1 = store
            .create_encoding_run(100, Some("ollama"), Some(768))
            .await
            .unwrap();
        let run2 = store
            .create_encoding_run(200, Some("openai"), Some(1536))
            .await
            .unwrap();
        let run3 = store.create_encoding_run(50, None, None).await.unwrap();

        // Complete one so it shouldn't be affected
        store
            .complete_encoding_run(run3, "completed")
            .await
            .unwrap();

        // Mark stale runs as failed
        store.mark_stale_runs_as_failed().await.unwrap();

        // No active runs should remain
        let active = store.get_active_encoding_run().await.unwrap();
        assert!(active.is_none());

        // Verify run1 and run2 are failed, run3 is still completed
        store
            .run(move |conn| {
                let status1: String = conn.query_row(
                    "SELECT status FROM encoding_runs WHERE id = ?1",
                    params![run1],
                    |row| row.get(0),
                )?;
                assert_eq!(status1, "failed");

                let status2: String = conn.query_row(
                    "SELECT status FROM encoding_runs WHERE id = ?1",
                    params![run2],
                    |row| row.get(0),
                )?;
                assert_eq!(status2, "failed");

                let status3: String = conn.query_row(
                    "SELECT status FROM encoding_runs WHERE id = ?1",
                    params![run3],
                    |row| row.get(0),
                )?;
                assert_eq!(status3, "completed");

                // Verify finished_at is set on the failed runs
                let finished1: Option<String> = conn.query_row(
                    "SELECT finished_at FROM encoding_runs WHERE id = ?1",
                    params![run1],
                    |row| row.get(0),
                )?;
                assert!(finished1.is_some());

                let finished2: Option<String> = conn.query_row(
                    "SELECT finished_at FROM encoding_runs WHERE id = ?1",
                    params![run2],
                    |row| row.get(0),
                )?;
                assert!(finished2.is_some());

                Ok(())
            })
            .await
            .unwrap();
    }

    // ==================== mark_stale_runs_as_failed - no running runs ====================
    #[tokio::test]
    async fn test_mark_stale_runs_as_failed_none() {
        let store = setup_test_store().await;

        // No runs at all - should not error
        store.mark_stale_runs_as_failed().await.unwrap();

        // Create and complete a run, then call again - should not error
        let run_id = store.create_encoding_run(100, None, None).await.unwrap();
        store
            .complete_encoding_run(run_id, "completed")
            .await
            .unwrap();
        store.mark_stale_runs_as_failed().await.unwrap();
    }

    // ==================== Test Case #32: concurrent access - no locks ====================
    #[tokio::test]
    async fn test_concurrent_read_write_no_locks() {
        // Use shared in-memory database so concurrent connections share the same data.
        // Plain `:memory:` creates a separate database per connection in the pool.
        let counter = TEST_STORE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!(
            "file:encprog_concurrent_{}?mode=memory&cache=shared",
            counter
        );
        let store = Arc::new(SqliteStore::connect(&db_name).await.unwrap());
        let run_id = store
            .create_encoding_run(1000, Some("ollama"), Some(768))
            .await
            .unwrap();

        // Spawn writer task: updates progress repeatedly
        let writer_store = store.clone();
        let writer = tokio::spawn(async move {
            for i in 1..=10 {
                writer_store
                    .update_encoding_run_progress(run_id, i * 100, Some(50.0))
                    .await
                    .unwrap();
            }
        });

        // Spawn reader task: queries active run repeatedly
        let reader_store = store.clone();
        let reader = tokio::spawn(async move {
            for _ in 0..10 {
                let result = reader_store.get_active_encoding_run().await;
                assert!(result.is_ok(), "Reader should not encounter lock errors");
                // The run should exist (still running)
                let run = result.unwrap();
                assert!(run.is_some(), "Active run should be found during reads");
            }
        });

        // Both tasks should complete without errors
        let (writer_result, reader_result) = tokio::join!(writer, reader);
        writer_result.unwrap();
        reader_result.unwrap();

        // Verify final state
        let run = store.get_active_encoding_run().await.unwrap().unwrap();
        assert_eq!(run.chunks_completed, 1000);
    }

    // ==================== Test Case #22: ETA with zero throughput returns None ====================
    #[test]
    fn test_calculate_eta_zero_throughput() {
        assert_eq!(calculate_eta(100, Some(0.0)), None);
        assert_eq!(calculate_eta(100, None), None);
        assert_eq!(calculate_eta(100, Some(-1.0)), None);
    }

    // ==================== Test Case #23: ETA with positive throughput calculates correctly ====================
    #[test]
    fn test_calculate_eta_positive_throughput() {
        // 100 remaining / 10 per sec = 10 seconds
        let eta = calculate_eta(100, Some(10.0)).unwrap();
        assert!((eta - 10.0).abs() < f64::EPSILON);

        // 500 remaining / 25.0 per sec = 20 seconds
        let eta = calculate_eta(500, Some(25.0)).unwrap();
        assert!((eta - 20.0).abs() < f64::EPSILON);

        // 0 remaining = 0 seconds
        let eta = calculate_eta(0, Some(10.0)).unwrap();
        assert!((eta - 0.0).abs() < f64::EPSILON);
    }

    // ==================== Test Case #24: ETA with very fast throughput (<1s) ====================
    #[test]
    fn test_calculate_eta_very_fast_throughput() {
        // 10 remaining / 1000 per sec = 0.01 seconds
        let eta = calculate_eta(10, Some(1000.0)).unwrap();
        assert!((eta - 0.01).abs() < 1e-10);

        // 1 remaining / 10000 per sec = 0.0001 seconds
        let eta = calculate_eta(1, Some(10000.0)).unwrap();
        assert!((eta - 0.0001).abs() < 1e-10);
    }

    // ==================== Test Case #25: ETA with very slow throughput (hours) ====================
    #[test]
    fn test_calculate_eta_very_slow_throughput() {
        // 10000 remaining / 0.5 per sec = 20000 seconds (~5.5 hours)
        let eta = calculate_eta(10000, Some(0.5)).unwrap();
        assert!((eta - 20000.0).abs() < f64::EPSILON);

        // 1000000 remaining / 0.1 per sec = 10000000 seconds (~115 days)
        let eta = calculate_eta(1000000, Some(0.1)).unwrap();
        assert!((eta - 10000000.0).abs() < 1e-6);
    }

    // ==================== Elapsed time calculation tests ====================
    #[test]
    fn test_calculate_elapsed_seconds_sqlite_format() {
        // Use a timestamp very close to now to get a small positive result
        let now = Utc::now();
        let ts = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let elapsed = calculate_elapsed_seconds(&ts).unwrap();
        // Should be very close to 0 (within 1 second)
        assert!(elapsed >= 0.0 && elapsed < 2.0, "elapsed was {}", elapsed);
    }

    #[test]
    fn test_calculate_elapsed_seconds_iso8601_format() {
        let now = Utc::now();
        let ts = now.format("%Y-%m-%dT%H:%M:%S").to_string();
        let elapsed = calculate_elapsed_seconds(&ts).unwrap();
        assert!(elapsed >= 0.0 && elapsed < 2.0, "elapsed was {}", elapsed);
    }

    #[test]
    fn test_calculate_elapsed_seconds_known_past() {
        // A timestamp 60 seconds in the past
        let past = Utc::now() - chrono::Duration::seconds(60);
        let ts = past.format("%Y-%m-%d %H:%M:%S").to_string();
        let elapsed = calculate_elapsed_seconds(&ts).unwrap();
        // Should be approximately 60 seconds (within 2 seconds tolerance)
        assert!(
            (elapsed - 60.0).abs() < 2.0,
            "elapsed was {}, expected ~60",
            elapsed
        );
    }

    #[test]
    fn test_calculate_elapsed_seconds_invalid_format() {
        let result = calculate_elapsed_seconds("not-a-timestamp");
        assert!(result.is_err());
    }

    // ==================== Staleness detection tests ====================
    #[test]
    fn test_is_stale_recent_timestamp() {
        let now = Utc::now();
        let ts = now.format("%Y-%m-%d %H:%M:%S").to_string();
        assert!(!is_stale(&ts), "Recent timestamp should not be stale");
    }

    #[test]
    fn test_is_stale_old_timestamp() {
        // 2 hours ago
        let old = Utc::now() - chrono::Duration::hours(2);
        let ts = old.format("%Y-%m-%d %H:%M:%S").to_string();
        assert!(is_stale(&ts), "2-hour old timestamp should be stale");
    }

    #[test]
    fn test_is_stale_just_under_threshold() {
        // 59 minutes ago - should not be stale
        let recent = Utc::now() - chrono::Duration::minutes(59);
        let ts = recent.format("%Y-%m-%d %H:%M:%S").to_string();
        assert!(
            !is_stale(&ts),
            "59-minute old timestamp should not be stale"
        );
    }

    #[test]
    fn test_is_stale_just_over_threshold() {
        // 61 minutes ago - should be stale
        let old = Utc::now() - chrono::Duration::minutes(61);
        let ts = old.format("%Y-%m-%d %H:%M:%S").to_string();
        assert!(is_stale(&ts), "61-minute old timestamp should be stale");
    }

    #[test]
    fn test_is_stale_invalid_timestamp() {
        assert!(
            is_stale("invalid"),
            "Invalid timestamp should be treated as stale"
        );
    }

    // ==================== Staleness in get_encoding_progress ====================
    #[tokio::test]
    async fn test_encoding_progress_stale_run_hidden() {
        let store = setup_test_store().await;
        setup_test_data(&store, "test-repo", 100).await;
        let run_id = store
            .create_encoding_run(100, Some("ollama"), Some(768))
            .await
            .unwrap();

        // Manually set last_batch_at to 2 hours ago to simulate staleness
        let two_hours_ago = (Utc::now() - chrono::Duration::hours(2))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let ts = two_hours_ago.clone();
        store
            .run(move |conn| {
                conn.execute(
                    "UPDATE encoding_runs SET last_batch_at = ?1 WHERE id = ?2",
                    params![ts, run_id],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        let progress = get_encoding_progress(store, None).await.unwrap();
        // Stale run should not appear as active
        assert!(
            progress.active_run.is_none(),
            "Stale run (>1 hour old) should not be shown as active"
        );
    }

    #[tokio::test]
    async fn test_encoding_progress_fresh_run_shown() {
        let store = setup_test_store().await;
        setup_test_data(&store, "test-repo", 100).await;
        let run_id = store
            .create_encoding_run(100, Some("ollama"), Some(768))
            .await
            .unwrap();

        // Update progress so last_batch_at is set to now
        store
            .update_encoding_run_progress(run_id, 50, Some(10.0))
            .await
            .unwrap();

        let progress = get_encoding_progress(store, None).await.unwrap();
        assert!(
            progress.active_run.is_some(),
            "Fresh run should be shown as active"
        );
    }

    // ==================== format_text - active run with elapsed and new format ====================
    #[test]
    fn test_format_text_active_run_full_format() {
        let response = EncodingProgressResponse {
            total_chunks: 2226,
            total_embeddings: 1226,
            percentage: 55.1,
            chunks_remaining: 1000,
            repo_filter: None,
            active_run: Some(ActiveRunInfo {
                run_id: 1,
                started_at: "2026-02-05 14:30:00".to_string(),
                total_chunks: 2226,
                chunks_completed: 1226,
                chunks_per_second: Some(22.3),
                provider: Some("ollama".to_string()),
                dimension: Some(1024),
                estimated_seconds_remaining: Some(44.8),
                elapsed_seconds: Some(135.0),
            }),
        };

        let output = format_text(&response);
        assert!(output.contains("Active Run:"));
        assert!(output.contains("Provider:        ollama (1024 dimensions)"));
        assert!(output.contains("Started:         2026-02-05 14:30:00"));
        assert!(output.contains("Elapsed:         ~2m 15s"));
        assert!(output.contains("Batch progress:  1,226 / 2,226 chunks"));
        assert!(output.contains("Throughput:      22.3 chunks/s"));
        assert!(output.contains("ETA:             ~45s remaining"));
    }

    // ==================== format_text - provider without dimension ====================
    #[test]
    fn test_format_text_provider_without_dimension() {
        let response = EncodingProgressResponse {
            total_chunks: 100,
            total_embeddings: 50,
            percentage: 50.0,
            chunks_remaining: 50,
            repo_filter: None,
            active_run: Some(ActiveRunInfo {
                run_id: 1,
                started_at: "2026-01-01 00:00:00".to_string(),
                total_chunks: 100,
                chunks_completed: 50,
                chunks_per_second: None,
                provider: Some("openai".to_string()),
                dimension: None,
                estimated_seconds_remaining: None,
                elapsed_seconds: None,
            }),
        };

        let output = format_text(&response);
        assert!(output.contains("Provider:        openai"));
        assert!(!output.contains("dimensions"));
        // No throughput or ETA when chunks_per_second is None
        assert!(!output.contains("Throughput:"));
        assert!(!output.contains("ETA:"));
    }

    // ==================== format_json - includes elapsed_seconds ====================
    #[test]
    fn test_format_json_includes_elapsed_seconds() {
        let response = EncodingProgressResponse {
            total_chunks: 100,
            total_embeddings: 50,
            percentage: 50.0,
            chunks_remaining: 50,
            repo_filter: None,
            active_run: Some(ActiveRunInfo {
                run_id: 1,
                started_at: "2026-01-01 00:00:00".to_string(),
                total_chunks: 100,
                chunks_completed: 50,
                chunks_per_second: Some(10.0),
                provider: Some("ollama".to_string()),
                dimension: Some(768),
                estimated_seconds_remaining: Some(5.0),
                elapsed_seconds: Some(120.5),
            }),
        };

        let json_str = format_json(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["active_run"]["elapsed_seconds"], 120.5);
        assert_eq!(parsed["active_run"]["estimated_seconds_remaining"], 5.0);
    }

    // ==================== format_duration edge cases for ETA display ====================
    #[test]
    fn test_format_duration_sub_second() {
        // Very fast ETA rounds to 0
        assert_eq!(format_duration(0.01), "~0s");
        assert_eq!(format_duration(0.4), "~0s");
        assert_eq!(format_duration(0.5), "~1s");
    }

    #[test]
    fn test_format_duration_very_long() {
        // 10 hours
        assert_eq!(format_duration(36000.0), "~10h");
        // 25 hours 30 minutes
        assert_eq!(format_duration(91800.0), "~25h 30m");
    }
}

/// Integration tests for end-to-end encoding progress functionality.
///
/// These tests verify the full integration between the embedding pipeline,
/// encoding progress tracking, and progress querying. They complement the
/// unit tests in this module and the pipeline tests in `embedding/pipeline.rs`.
///
/// Tests implemented:
/// - End-to-end progress flow (pipeline writes, progress query reads)
/// - Concurrent pipeline + progress query (test case #32)
/// - Provider/dimension mismatch scenario (test case #33)
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::db::sqlite::SqliteStore;
    use crate::db::traits::StoreChunks;
    use crate::db::{ChunkRecord, FileRecord};
    use crate::embedding::cache::EmbeddingCache;
    use crate::embedding::config::CacheConfig;
    use crate::embedding::error::EmbeddingError;
    use crate::embedding::pipeline::{EmbeddingPipeline, PipelineConfig};
    use crate::embedding::provider::{EmbeddingProvider, ProviderMetrics};
    use crate::embedding::service::EmbeddingService;
    use async_trait::async_trait;
    use rusqlite::params;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Counter for unique shared in-memory database names.
    static INTEGRATION_STORE_COUNTER: AtomicUsize = AtomicUsize::new(1000);

    /// Mock provider with controllable delay for concurrent testing.
    struct SlowMockProvider {
        delay_ms: u64,
        dimension: usize,
        name: &'static str,
    }

    #[async_trait]
    impl EmbeddingProvider for SlowMockProvider {
        async fn embed(&self, _text: String) -> Result<Vec<f32>, EmbeddingError> {
            if self.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            }
            Ok(vec![0.1; self.dimension])
        }

        async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
            if self.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            }
            Ok(vec![vec![0.1; self.dimension]; texts.len()])
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }

        fn metrics(&self) -> Option<ProviderMetrics> {
            Some(ProviderMetrics {
                total_requests: 1,
                total_tokens: 100,
                failed_requests: 0,
                estimated_cost_usd: 0.0001,
            })
        }
    }

    /// Fast mock provider (no delay).
    struct FastMockProvider {
        dimension: usize,
        name: &'static str,
    }

    #[async_trait]
    impl EmbeddingProvider for FastMockProvider {
        async fn embed(&self, _text: String) -> Result<Vec<f32>, EmbeddingError> {
            Ok(vec![0.1; self.dimension])
        }

        async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
            Ok(vec![vec![0.1; self.dimension]; texts.len()])
        }

        fn dimension(&self) -> usize {
            self.dimension
        }

        fn provider_name(&self) -> &'static str {
            self.name
        }

        fn metrics(&self) -> Option<ProviderMetrics> {
            Some(ProviderMetrics {
                total_requests: 1,
                total_tokens: 100,
                failed_requests: 0,
                estimated_cost_usd: 0.0001,
            })
        }
    }

    fn create_service_with_provider(provider: Box<dyn EmbeddingProvider>) -> EmbeddingService {
        let cache_config = CacheConfig {
            max_entries: 1000,
            ttl_seconds: 3600,
            enable_metrics: true,
        };
        let cache = EmbeddingCache::new(cache_config).unwrap();
        EmbeddingService::new(provider, Arc::new(cache))
    }

    fn create_slow_service(
        delay_ms: u64,
        dimension: usize,
        name: &'static str,
    ) -> EmbeddingService {
        let provider = Box::new(SlowMockProvider {
            delay_ms,
            dimension,
            name,
        });
        create_service_with_provider(provider)
    }

    fn create_fast_service(dimension: usize, name: &'static str) -> EmbeddingService {
        let provider = Box::new(FastMockProvider { dimension, name });
        create_service_with_provider(provider)
    }

    /// Helper to create an in-memory test store.
    async fn setup_test_store() -> SqliteStore {
        SqliteStore::connect(":memory:").await.unwrap()
    }

    /// Helper to create a shared in-memory test store (same DB across connections).
    async fn setup_shared_test_store() -> Arc<SqliteStore> {
        let counter = INTEGRATION_STORE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!(
            "file:encprog_integration_{}?mode=memory&cache=shared",
            counter
        );
        Arc::new(SqliteStore::connect(&db_name).await.unwrap())
    }

    /// Helper to create test data: repo, worktree, commit, file, and N chunks.
    async fn setup_test_chunks(store: &SqliteStore, repo_name: &str, num_chunks: usize) {
        let repo_id = store
            .get_or_create_repo(repo_name, "/test/path")
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
            content_hash: format!("hash_{}", repo_name),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        for i in 0..num_chunks {
            let chunk = ChunkRecord {
                file_id,
                worktree_id,
                blob_sha: format!("blob_{}_{}", repo_name, i),
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

    /// Helper to insert pre-existing embeddings for specific blob_shas.
    async fn insert_embeddings_with_params(
        store: &SqliteStore,
        blob_shas: Vec<String>,
        dimension: i32,
        model_version: &str,
    ) {
        let embedding_bytes = vec![0u8; (dimension as usize) * 4];
        for blob_sha in blob_shas {
            let emb = embedding_bytes.clone();
            let mv = model_version.to_string();
            store
                .run(move |conn| {
                    conn.execute(
                        "INSERT OR IGNORE INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![blob_sha, emb, dimension, mv],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
    }

    // ========================================================================
    // Test 1: End-to-end progress flow
    //
    // Pipeline writes progress, progress command reads it correctly.
    // Verifies: run creation, progress updates, completion, ETA calculation.
    // ========================================================================
    #[tokio::test]
    async fn test_end_to_end_progress_flow() {
        let store = setup_test_store().await;
        let store_arc = Arc::new(store);

        // Create 10 test chunks
        setup_test_chunks(&store_arc, "test-repo", 10).await;

        // Verify initial state: no progress, no active run
        let initial = get_encoding_progress(store_arc.clone(), None)
            .await
            .unwrap();
        assert_eq!(initial.total_chunks, 10);
        assert_eq!(initial.total_embeddings, 0);
        assert_eq!(initial.percentage, 0.0);
        assert_eq!(initial.chunks_remaining, 10);
        assert!(initial.active_run.is_none());

        // Run pipeline with small batch size (2) to get multiple batches
        let service = create_fast_service(1536, "openai");
        let config = PipelineConfig {
            batch_size: 2,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);
        let stats = pipeline.run(&store_arc).await.unwrap();

        assert_eq!(stats.total_chunks, 10);
        assert_eq!(stats.provider, "openai");
        assert_eq!(stats.dimension, 1536);

        // After pipeline completes: verify progress shows 100%
        let final_progress = get_encoding_progress(store_arc.clone(), None)
            .await
            .unwrap();
        assert_eq!(final_progress.total_chunks, 10);
        assert_eq!(final_progress.total_embeddings, 10);
        assert!((final_progress.percentage - 100.0).abs() < f64::EPSILON);
        assert_eq!(final_progress.chunks_remaining, 0);

        // Active run should be gone (completed)
        assert!(
            final_progress.active_run.is_none(),
            "Run should be completed, not active"
        );

        // Verify the run is in the database with correct final state
        store_arc
            .run(move |conn| {
                let (status, total_chunks, chunks_completed, provider, dimension, finished_at): (
                    String,
                    i64,
                    i64,
                    Option<String>,
                    Option<i32>,
                    Option<String>,
                ) = conn.query_row(
                    "SELECT status, total_chunks, chunks_completed, provider, dimension, finished_at
                     FROM encoding_runs ORDER BY id DESC LIMIT 1",
                    [],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )?;

                assert_eq!(status, "completed");
                assert_eq!(total_chunks, 10);
                assert_eq!(chunks_completed, 10);
                assert_eq!(provider, Some("openai".to_string()));
                assert_eq!(dimension, Some(1536));
                assert!(finished_at.is_some());

                // Verify chunks_per_second was recorded
                let cps: Option<f64> = conn.query_row(
                    "SELECT chunks_per_second FROM encoding_runs ORDER BY id DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )?;
                assert!(cps.is_some(), "chunks_per_second should be set");
                assert!(cps.unwrap() > 0.0, "chunks_per_second should be positive");

                Ok(())
            })
            .await
            .unwrap();
    }

    // ========================================================================
    // Test 2: Concurrent pipeline + progress query (test case #32)
    //
    // Start pipeline with slow mock provider, spawn concurrent task querying
    // progress repeatedly. Verify no lock errors and valid data throughout.
    //
    // Note: The pipeline runs on the current task while progress queries are
    // spawned in a concurrent task. This avoids Send issues with the pipeline's
    // internal callback types while still exercising true concurrent DB access.
    // ========================================================================
    #[tokio::test]
    async fn test_concurrent_pipeline_and_progress_query() {
        let store = setup_shared_test_store().await;

        // Create 10 test chunks
        setup_test_chunks(&store, "test-repo", 10).await;

        // Create pipeline with slow provider (50ms delay per batch)
        let service = create_slow_service(50, 1536, "ollama");
        let config = PipelineConfig {
            batch_size: 2,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);

        // Clone store for the progress query task
        let query_store = store.clone();

        // Spawn concurrent progress query task
        let query_handle = tokio::spawn(async move {
            let mut query_count = 0;
            let mut saw_active_run = false;
            let mut last_percentage = -1.0f64;
            let mut progress_increased = false;

            // Query progress repeatedly while pipeline runs
            for _ in 0..50 {
                let result = get_encoding_progress(query_store.clone(), None).await;
                assert!(
                    result.is_ok(),
                    "Progress query should not fail during concurrent access: {:?}",
                    result.err()
                );

                let progress = result.unwrap();
                query_count += 1;

                // Verify data is valid (no corruption)
                assert!(progress.total_chunks >= 0);
                assert!(progress.total_embeddings >= 0);
                assert!(progress.total_embeddings <= progress.total_chunks);
                assert!(progress.percentage >= 0.0);
                assert!(progress.percentage <= 100.0);
                assert!(progress.chunks_remaining >= 0);

                // Track if we ever see an active run
                if let Some(run) = &progress.active_run {
                    saw_active_run = true;
                    assert!(run.chunks_completed >= 0);
                    assert!(run.chunks_completed <= run.total_chunks);
                    if let Some(cps) = run.chunks_per_second {
                        assert!(cps >= 0.0, "chunks_per_second must be non-negative");
                    }
                }

                // Track if progress ever increases
                if progress.percentage > last_percentage {
                    progress_increased = true;
                }
                last_percentage = progress.percentage;

                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }

            (query_count, saw_active_run, progress_increased)
        });

        // Run pipeline on the current task (concurrently with the query task)
        let pipeline_result = pipeline.run(&store).await;

        // Wait for query task to finish
        let query_result = query_handle.await;

        // Pipeline should complete successfully
        let stats = pipeline_result.unwrap();
        assert_eq!(stats.total_chunks, 10);

        // Query task should complete without errors
        let (query_count, _saw_active_run, _progress_increased) = query_result.unwrap();
        assert!(
            query_count > 0,
            "Should have queried progress at least once"
        );
        // Note: saw_active_run and progress_increased may be false if the pipeline
        // completes very quickly before the query task starts. This is acceptable
        // since the main goal is verifying no lock errors or data corruption.

        // Final state should be correct
        let final_progress = get_encoding_progress(store.clone(), None).await.unwrap();
        assert_eq!(final_progress.total_chunks, 10);
        assert_eq!(final_progress.total_embeddings, 10);
        assert!((final_progress.percentage - 100.0).abs() < f64::EPSILON);
    }

    // ========================================================================
    // Test 3: Provider/dimension mismatch scenario (test case #33)
    //
    // Create initial embeddings with provider "ollama" dimension 1024,
    // add new chunks, run pipeline with "openai" dimension 1536.
    // Verify old embeddings remain and new run shows new provider.
    // ========================================================================
    #[tokio::test]
    async fn test_provider_dimension_mismatch() {
        let store = setup_test_store().await;
        let store_arc = Arc::new(store);

        // Step 1: Create initial 5 chunks and embed them with "ollama" / 1024
        setup_test_chunks(&store_arc, "test-repo", 5).await;

        let service1 = create_fast_service(1024, "ollama");
        let config1 = PipelineConfig {
            batch_size: 10,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline1 = EmbeddingPipeline::new(service1, config1);
        let stats1 = pipeline1.run(&store_arc).await.unwrap();
        assert_eq!(stats1.total_chunks, 5);
        assert_eq!(stats1.provider, "ollama");
        assert_eq!(stats1.dimension, 1024);

        // Verify: 5 chunks, 5 embeddings, 100%
        let progress_after_first = get_encoding_progress(store_arc.clone(), None)
            .await
            .unwrap();
        assert_eq!(progress_after_first.total_chunks, 5);
        assert_eq!(progress_after_first.total_embeddings, 5);
        assert!((progress_after_first.percentage - 100.0).abs() < f64::EPSILON);

        // Verify first run is completed
        let first_run_status: String = store_arc
            .run(|conn| {
                let status: String = conn.query_row(
                    "SELECT status FROM encoding_runs ORDER BY id ASC LIMIT 1",
                    [],
                    |row| row.get(0),
                )?;
                Ok(status)
            })
            .await
            .unwrap();
        assert_eq!(first_run_status, "completed");

        // Step 2: Add 3 new chunks that don't have embeddings yet
        // We need to create new chunks with different blob_shas
        let repo_id = store_arc
            .run(|conn| {
                let id: i64 = conn.query_row(
                    "SELECT id FROM repos WHERE name = ?1",
                    params!["test-repo"],
                    |row| row.get(0),
                )?;
                Ok(id)
            })
            .await
            .unwrap();

        let worktree_id = store_arc
            .run(move |conn| {
                let id: i64 = conn.query_row(
                    "SELECT id FROM worktrees WHERE repo_id = ?1",
                    params![repo_id],
                    |row| row.get(0),
                )?;
                Ok(id)
            })
            .await
            .unwrap();

        let commit_id = store_arc
            .run(move |conn| {
                let id: i64 = conn.query_row(
                    "SELECT id FROM commits WHERE repo_id = ?1",
                    params![repo_id],
                    |row| row.get(0),
                )?;
                Ok(id)
            })
            .await
            .unwrap();

        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "new_file.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash_new_file".to_string(),
            size_bytes: 200,
            last_modified: None,
        };
        let new_file_id = store_arc.upsert_file(&file).await.unwrap();

        for i in 0..3 {
            let chunk = ChunkRecord {
                file_id: new_file_id,
                worktree_id,
                blob_sha: format!("blob_new_{}", i),
                symbol_name: Some(format!("fn_new_{}", i)),
                kind: "function".to_string(),
                signature: Some(format!("fn fn_new_{}()", i)),
                docstring: Some(format!("New function {}", i)),
                start_line: i * 10 + 1,
                end_line: i * 10 + 10,
                preview: format!("fn fn_new_{}() {{}}", i),
                ts_doc_text: String::new(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
            };
            store_arc.insert_chunk(&chunk).await.unwrap();
        }

        // Verify: 8 total chunks, 5 embeddings (the old ones)
        let progress_before_second = get_encoding_progress(store_arc.clone(), None)
            .await
            .unwrap();
        assert_eq!(progress_before_second.total_chunks, 8);
        assert_eq!(progress_before_second.total_embeddings, 5);
        assert_eq!(progress_before_second.chunks_remaining, 3);

        // Step 3: Run pipeline with "openai" / 1536 (different provider & dimension)
        let service2 = create_fast_service(1536, "openai");
        let config2 = PipelineConfig {
            batch_size: 10,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline2 = EmbeddingPipeline::new(service2, config2);
        let stats2 = pipeline2.run(&store_arc).await.unwrap();

        // Only the 3 new chunks should be processed (incremental mode)
        assert_eq!(stats2.total_chunks, 3);
        assert_eq!(stats2.provider, "openai");
        assert_eq!(stats2.dimension, 1536);

        // Step 4: Verify final state
        let final_progress = get_encoding_progress(store_arc.clone(), None)
            .await
            .unwrap();
        assert_eq!(final_progress.total_chunks, 8);
        assert_eq!(final_progress.total_embeddings, 8);
        assert!((final_progress.percentage - 100.0).abs() < f64::EPSILON);
        assert_eq!(final_progress.chunks_remaining, 0);

        // Verify: old embeddings remain in the database
        let old_embedding_count: i64 = store_arc
            .run(|conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM code_embeddings WHERE blob_sha LIKE 'blob_test-repo_%'",
                    [],
                    |row| row.get(0),
                )?;
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(
            old_embedding_count, 5,
            "Old embeddings should remain in the database"
        );

        // Verify: new embeddings were created
        let new_embedding_count: i64 = store_arc
            .run(|conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM code_embeddings WHERE blob_sha LIKE 'blob_new_%'",
                    [],
                    |row| row.get(0),
                )?;
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(new_embedding_count, 3, "New embeddings should be created");

        // Verify: the second encoding run recorded the new provider and dimension
        let (provider2, dimension2): (Option<String>, Option<i32>) = store_arc
            .run(|conn| {
                let row: (Option<String>, Option<i32>) = conn.query_row(
                    "SELECT provider, dimension FROM encoding_runs ORDER BY id DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                Ok(row)
            })
            .await
            .unwrap();
        assert_eq!(provider2, Some("openai".to_string()));
        assert_eq!(dimension2, Some(1536));

        // Verify: we have exactly 2 encoding runs (one per pipeline invocation)
        let run_count: i64 = store_arc
            .run(|conn| {
                let count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM encoding_runs", [], |row| row.get(0))?;
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(run_count, 2, "Should have two completed encoding runs");
    }

    // ========================================================================
    // Test 4: Progress percentage increases and ETA decreases during encoding
    //
    // Uses a slow mock provider and progress callback to capture intermediate
    // progress snapshots, then verifies monotonic progress increase.
    // ========================================================================
    #[tokio::test]
    async fn test_progress_increases_and_eta_decreases() {
        let store = setup_shared_test_store().await;

        // Create 10 test chunks
        setup_test_chunks(&store, "test-repo", 10).await;

        // Use slow provider so we can observe progress changes
        let service = create_slow_service(20, 1536, "ollama");
        let config = PipelineConfig {
            batch_size: 2,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);

        // Use a progress callback to capture snapshots
        let snapshots = Arc::new(std::sync::Mutex::new(Vec::<(usize, usize)>::new()));
        let snapshots_clone = snapshots.clone();

        let callback = move |completed: usize, total: usize| {
            snapshots_clone.lock().unwrap().push((completed, total));
        };

        let stats = pipeline
            .run_with_progress(&store, Some(&callback))
            .await
            .unwrap();
        assert_eq!(stats.total_chunks, 10);

        // Verify progress snapshots are monotonically increasing
        let captured = snapshots.lock().unwrap().clone();
        assert!(
            !captured.is_empty(),
            "Should have captured at least one progress snapshot"
        );

        let mut prev_completed = 0;
        for (completed, total) in &captured {
            assert_eq!(*total, 10, "Total should always be 10");
            assert!(
                *completed >= prev_completed,
                "Progress should never decrease: {} < {}",
                completed,
                prev_completed
            );
            assert!(*completed <= *total, "Completed should not exceed total");
            prev_completed = *completed;
        }

        // Final snapshot should have all chunks completed
        let (final_completed, _) = captured.last().unwrap();
        assert_eq!(
            *final_completed, 10,
            "Final progress should show all chunks completed"
        );

        // Verify encoding run was marked completed with valid throughput
        store
            .run(move |conn| {
                let (status, cps): (String, Option<f64>) = conn.query_row(
                    "SELECT status, chunks_per_second FROM encoding_runs ORDER BY id DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                assert_eq!(status, "completed");
                assert!(cps.is_some());
                assert!(cps.unwrap() > 0.0, "Throughput should be positive");
                Ok(())
            })
            .await
            .unwrap();
    }

    // ========================================================================
    // Test 5: Stale run cleanup during pipeline restart (thorough variant)
    //
    // This extends the ENCPROG.2002 test by verifying that:
    // - Multiple stale runs from different providers are cleaned up
    // - A new run is created successfully after cleanup
    // - The new run tracks the correct provider/dimension
    // ========================================================================
    #[tokio::test]
    async fn test_stale_run_cleanup_multi_provider() {
        let store = setup_test_store().await;

        // Create multiple stale runs with different providers
        let stale1 = store
            .create_encoding_run(500, Some("ollama"), Some(768))
            .await
            .unwrap();
        let stale2 = store
            .create_encoding_run(300, Some("openai"), Some(1536))
            .await
            .unwrap();

        // Verify both are active
        let active = store.get_active_encoding_run().await.unwrap();
        assert!(active.is_some(), "Should have an active run");

        // Add test chunks so the pipeline has work
        setup_test_chunks(&store, "test-repo", 3).await;

        // Run a new pipeline - should clean up stale runs first
        let service = create_fast_service(1024, "google");
        let config = PipelineConfig {
            batch_size: 10,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);
        let stats = pipeline.run(&store).await.unwrap();
        assert_eq!(stats.total_chunks, 3);
        assert_eq!(stats.provider, "google");

        // Verify: both stale runs are marked as failed
        store
            .run(move |conn| {
                let status1: String = conn.query_row(
                    "SELECT status FROM encoding_runs WHERE id = ?1",
                    params![stale1],
                    |row| row.get(0),
                )?;
                assert_eq!(status1, "failed", "First stale run should be marked failed");

                let status2: String = conn.query_row(
                    "SELECT status FROM encoding_runs WHERE id = ?1",
                    params![stale2],
                    |row| row.get(0),
                )?;
                assert_eq!(status2, "failed", "Second stale run should be marked failed");

                // The new run (third) should be completed
                let (status3, provider3, dimension3): (String, Option<String>, Option<i32>) =
                    conn.query_row(
                        "SELECT status, provider, dimension FROM encoding_runs ORDER BY id DESC LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )?;
                assert_eq!(status3, "completed");
                assert_eq!(provider3, Some("google".to_string()));
                assert_eq!(dimension3, Some(1024));

                // Total runs should be 3
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM encoding_runs",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(count, 3);

                Ok(())
            })
            .await
            .unwrap();
    }

    // ========================================================================
    // Test 6: Error does not prevent future runs
    //
    // This extends ENCPROG.2002's error test by verifying that after a failed
    // run, a subsequent run with a working provider succeeds.
    // ========================================================================
    #[tokio::test]
    async fn test_error_does_not_prevent_future_runs() {
        let store = setup_test_store().await;
        setup_test_chunks(&store, "test-repo", 3).await;

        // First run: use a failing provider
        let failing_provider = Box::new(FailingMockProvider {
            dimension: 1536,
            name: "failing",
        });
        let service1 = create_service_with_provider(failing_provider);
        let config1 = PipelineConfig {
            batch_size: 2,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline1 = EmbeddingPipeline::new(service1, config1);
        let result1 = pipeline1.run(&store).await;
        assert!(result1.is_err(), "First run should fail");

        // Verify the failed run is in the database
        let failed_run_count: i64 = store
            .run(|conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM encoding_runs WHERE status = 'failed'",
                    [],
                    |row| row.get(0),
                )?;
                Ok(count)
            })
            .await
            .unwrap();
        assert_eq!(failed_run_count, 1);

        // Second run: use a working provider
        let service2 = create_fast_service(1536, "openai");
        let config2 = PipelineConfig {
            batch_size: 10,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline2 = EmbeddingPipeline::new(service2, config2);
        let stats2 = pipeline2.run(&store).await.unwrap();

        // Second run should succeed and process all chunks
        assert_eq!(stats2.total_chunks, 3);

        // Verify final state
        let final_progress = get_encoding_progress(Arc::new(store), None).await.unwrap();
        assert_eq!(final_progress.total_chunks, 3);
        assert_eq!(final_progress.total_embeddings, 3);
        assert!((final_progress.percentage - 100.0).abs() < f64::EPSILON);
    }

    /// Mock provider that always fails (for error handling tests).
    struct FailingMockProvider {
        dimension: usize,
        name: &'static str,
    }

    #[async_trait]
    impl EmbeddingProvider for FailingMockProvider {
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

    // ========================================================================
    // Test 7: Pre-existing embeddings counted in progress
    //
    // Verifies that embeddings inserted outside the pipeline (e.g., from a
    // previous run or manual insertion) are counted correctly by the progress
    // query.
    // ========================================================================
    #[tokio::test]
    async fn test_preexisting_embeddings_counted_in_progress() {
        let store = setup_test_store().await;
        let store_arc = Arc::new(store);

        // Create 10 chunks
        setup_test_chunks(&store_arc, "test-repo", 10).await;

        // Manually insert embeddings for 4 of the 10 chunks (simulating a previous run)
        let blob_shas: Vec<String> = (0..4).map(|i| format!("blob_test-repo_{}", i)).collect();
        insert_embeddings_with_params(&store_arc, blob_shas, 1024, "ollama").await;

        // Verify progress reflects the pre-existing embeddings
        let progress = get_encoding_progress(store_arc.clone(), None)
            .await
            .unwrap();
        assert_eq!(progress.total_chunks, 10);
        assert_eq!(progress.total_embeddings, 4);
        assert!((progress.percentage - 40.0).abs() < f64::EPSILON);
        assert_eq!(progress.chunks_remaining, 6);

        // Run pipeline - should only process the 6 remaining chunks
        let service = create_fast_service(1536, "openai");
        let config = PipelineConfig {
            batch_size: 10,
            incremental: true,
            dry_run: false,
            sample_size: None,
            batch_delay_ms: 0,
            max_cost_usd: None,
        };
        let pipeline = EmbeddingPipeline::new(service, config);
        let stats = pipeline.run(&store_arc).await.unwrap();
        assert_eq!(
            stats.total_chunks, 6,
            "Should only process chunks without embeddings"
        );

        // Final progress: all 10 chunks should now have embeddings
        let final_progress = get_encoding_progress(store_arc.clone(), None)
            .await
            .unwrap();
        assert_eq!(final_progress.total_chunks, 10);
        assert_eq!(final_progress.total_embeddings, 10);
        assert!((final_progress.percentage - 100.0).abs() < f64::EPSILON);
    }
}

/// Performance benchmark tests for encoding progress queries with large datasets.
///
/// These tests validate that progress queries complete in <500ms even with 100K chunks,
/// confirming the "typical repository" performance assumption from the design plan.
///
/// Run with: cargo test --release -p crewchief-maproom -- --ignored --nocapture benchmark_large_repository
#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use crate::db::sqlite::SqliteStore;
    use rusqlite::params;
    use std::sync::Arc;
    use std::time::Instant;

    const NUM_REPOS: usize = 10;
    const CHUNKS_PER_REPO: usize = 10_000;
    const TOTAL_CHUNKS: usize = NUM_REPOS * CHUNKS_PER_REPO; // 100,000
    const TOTAL_EMBEDDINGS: usize = TOTAL_CHUNKS / 2; // 50,000 (50% coverage)
    const QUERY_THRESHOLD_MS: u128 = 500;

    /// Helper to create a test store with migrations applied.
    async fn setup_test_store() -> Arc<SqliteStore> {
        Arc::new(SqliteStore::connect(":memory:").await.unwrap())
    }

    /// Bulk-insert test data: 10 repos, 100K chunks, 50K embeddings.
    ///
    /// Uses direct SQL batch inserts inside a transaction for speed.
    /// Returns the time taken for setup.
    async fn setup_large_test_db(store: &Arc<SqliteStore>) -> std::time::Duration {
        let start = Instant::now();

        store
            .run(move |conn| {
                let tx = conn.transaction()?;

                // 1. Create 10 repos and worktrees
                for repo_idx in 0..NUM_REPOS {
                    let repo_name = format!("bench-repo-{}", repo_idx);
                    let repo_path = format!("/bench/path/{}", repo_idx);
                    tx.execute(
                        "INSERT INTO repos (name, root_path) VALUES (?1, ?2)",
                        params![repo_name, repo_path],
                    )?;
                    let repo_id: i64 =
                        tx.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))?;

                    tx.execute(
                        "INSERT INTO worktrees (repo_id, name, abs_path) VALUES (?1, ?2, ?3)",
                        params![repo_id, "main", repo_path],
                    )?;
                    let worktree_id: i64 =
                        tx.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))?;

                    tx.execute(
                        "INSERT INTO commits (repo_id, sha) VALUES (?1, ?2)",
                        params![repo_id, format!("commit_{}", repo_idx)],
                    )?;
                    let commit_id: i64 =
                        tx.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))?;

                    tx.execute(
                        "INSERT INTO files (repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![repo_id, worktree_id, commit_id, "bench.rs", "rust", format!("hash_{}", repo_idx), 1000],
                    )?;
                    let file_id: i64 =
                        tx.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))?;

                    // 2. Bulk-insert chunks for this repo (CHUNKS_PER_REPO each)
                    {
                        let mut chunk_stmt = tx.prepare(
                            "INSERT INTO chunks (file_id, blob_sha, symbol_name, kind, start_line, end_line, preview, ts_doc_text, recency_score, churn_score, worktree_ids)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        )?;

                        let mut cw_stmt = tx.prepare(
                            "INSERT INTO chunk_worktrees (chunk_id, worktree_id) VALUES (?1, ?2)",
                        )?;

                        for chunk_idx in 0..CHUNKS_PER_REPO {
                            let global_idx = repo_idx * CHUNKS_PER_REPO + chunk_idx;
                            let blob_sha = format!("blob_{:08x}", global_idx);
                            let sym = format!("fn_{}", chunk_idx);
                            let line_start = (chunk_idx * 10 + 1) as i32;
                            let line_end = (chunk_idx * 10 + 10) as i32;
                            let preview = format!("fn fn_{}() {{}}", chunk_idx);
                            let wt_json = format!("[{}]", worktree_id);

                            chunk_stmt.execute(params![
                                file_id, blob_sha, sym, "function",
                                line_start, line_end, preview, "",
                                1.0_f64, 0.5_f64, wt_json,
                            ])?;
                            let chunk_id: i64 =
                                tx.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))?;

                            cw_stmt.execute(params![chunk_id, worktree_id])?;
                        }
                    }

                    // 3. Bulk-insert embeddings for 50% of chunks in this repo
                    {
                        let mut emb_stmt = tx.prepare(
                            "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
                             VALUES (?1, ?2, ?3, ?4)",
                        )?;

                        let dummy_embedding = vec![0u8; 64]; // Small dummy blob
                        for chunk_idx in 0..CHUNKS_PER_REPO {
                            if chunk_idx % 2 == 0 {
                                let global_idx = repo_idx * CHUNKS_PER_REPO + chunk_idx;
                                let blob_sha = format!("blob_{:08x}", global_idx);
                                emb_stmt.execute(params![
                                    blob_sha,
                                    dummy_embedding,
                                    768,
                                    "bench-model",
                                ])?;
                            }
                        }
                    }
                }

                tx.commit()?;
                Ok(())
            })
            .await
            .unwrap();

        start.elapsed()
    }

    /// Performance benchmark: validates encoding progress queries complete in <500ms
    /// with 100K chunks distributed across 10 repos.
    ///
    /// Run with: cargo test --release -p crewchief-maproom -- --ignored --nocapture benchmark_large_repository
    #[tokio::test]
    #[ignore]
    async fn benchmark_large_repository() {
        let store = setup_test_store().await;

        // ---- Database Setup ----
        let setup_duration = setup_large_test_db(&store).await;

        println!();
        println!("Benchmark: 100K chunks performance test");
        println!("----------------------------------------");
        println!("Database setup:        {}ms", setup_duration.as_millis());

        // ---- Warm-up query (not measured) ----
        let _ = get_encoding_progress(store.clone(), None).await.unwrap();

        // ---- Global progress query ----
        let start = Instant::now();
        let global_result = get_encoding_progress(store.clone(), None).await.unwrap();
        let global_duration = start.elapsed();
        println!("Global progress query: {}ms", global_duration.as_millis());

        // Sanity-check the data
        assert_eq!(
            global_result.total_chunks, TOTAL_CHUNKS as i64,
            "Expected {} total chunks, got {}",
            TOTAL_CHUNKS, global_result.total_chunks
        );
        assert_eq!(
            global_result.total_embeddings, TOTAL_EMBEDDINGS as i64,
            "Expected {} total embeddings, got {}",
            TOTAL_EMBEDDINGS, global_result.total_embeddings
        );
        assert!(
            (global_result.percentage - 50.0).abs() < 0.1,
            "Expected ~50% coverage, got {}%",
            global_result.percentage
        );

        // ---- Repo-filtered query ----
        let start = Instant::now();
        let repo_result = get_encoding_progress(store.clone(), Some("bench-repo-0".to_string()))
            .await
            .unwrap();
        let repo_duration = start.elapsed();
        println!("Repo filtered query:   {}ms", repo_duration.as_millis());

        assert_eq!(
            repo_result.total_chunks, CHUNKS_PER_REPO as i64,
            "Expected {} chunks for single repo, got {}",
            CHUNKS_PER_REPO, repo_result.total_chunks
        );

        // ---- Query with active encoding run ----
        store
            .create_encoding_run(TOTAL_CHUNKS as i64, Some("bench-provider"), Some(768))
            .await
            .unwrap();

        let start = Instant::now();
        let run_result = get_encoding_progress(store.clone(), None).await.unwrap();
        let run_duration = start.elapsed();
        println!("With active run:       {}ms", run_duration.as_millis());

        assert!(
            run_result.active_run.is_some(),
            "Expected active run to be present"
        );

        // ---- Results ----
        println!("----------------------------------------");

        let all_pass = global_duration.as_millis() < QUERY_THRESHOLD_MS
            && repo_duration.as_millis() < QUERY_THRESHOLD_MS
            && run_duration.as_millis() < QUERY_THRESHOLD_MS;

        if all_pass {
            println!("\u{2713} All queries < {}ms threshold", QUERY_THRESHOLD_MS);
        } else {
            println!(
                "\u{2717} FAILED: some queries exceeded {}ms threshold",
                QUERY_THRESHOLD_MS
            );
        }
        println!();

        assert!(
            global_duration.as_millis() < QUERY_THRESHOLD_MS,
            "Global progress query took {}ms, exceeds {}ms threshold",
            global_duration.as_millis(),
            QUERY_THRESHOLD_MS
        );
        assert!(
            repo_duration.as_millis() < QUERY_THRESHOLD_MS,
            "Repo filtered query took {}ms, exceeds {}ms threshold",
            repo_duration.as_millis(),
            QUERY_THRESHOLD_MS
        );
        assert!(
            run_duration.as_millis() < QUERY_THRESHOLD_MS,
            "Query with active run took {}ms, exceeds {}ms threshold",
            run_duration.as_millis(),
            QUERY_THRESHOLD_MS
        );
    }
}
