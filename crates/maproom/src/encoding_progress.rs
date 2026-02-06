//! Encoding progress module for querying chunk/embedding counts and active encoding runs.
//!
//! This module mirrors the pattern established in `status.rs`:
//! query function + response structs + formatters.

use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
