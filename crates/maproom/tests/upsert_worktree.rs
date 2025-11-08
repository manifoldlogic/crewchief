//! Integration tests for worktree-aware chunk upsert.
//!
//! Tests verify:
//! - INSERT creates chunk with single worktree_id in array
//! - UPDATE (ON CONFLICT) appends worktree_id if not present
//! - Idempotency: same worktree_id twice doesn't create duplicates
//! - Multi-worktree: same content tracked across multiple worktrees
//!
//! Requirements:
//! - PostgreSQL with DATABASE_URL environment variable
//! - Migration 004 applied (worktree_ids column exists)
//!
//! Run with: cargo test --test upsert_worktree -- --ignored --nocapture

use crewchief_maproom::{db, metrics::CacheMetrics, upsert::{ParsedChunk, upsert_chunk_with_worktree}};

/// Helper function to connect to the test database.
async fn test_db() -> Option<tokio_postgres::Client> {
    dotenvy::dotenv().ok();
    db::connect().await.ok()
}

/// Skip test if database is not available.
macro_rules! skip_if_no_db {
    () => {
        match test_db().await {
            Some(client) => client,
            None => {
                eprintln!("Skipping test: DATABASE_URL not set or connection failed");
                return;
            }
        }
    };
}

/// Helper to create a test worktree in the database.
async fn create_test_worktree(client: &tokio_postgres::Client, branch: &str) -> anyhow::Result<i64> {
    // First, ensure we have a repo
    let repo_id: i64 = client
        .query_one(
            r#"
            INSERT INTO maproom.repos (url, name, primary_language)
            VALUES ($1, $2, 'rust')
            ON CONFLICT (url) DO UPDATE SET url = EXCLUDED.url
            RETURNING id
            "#,
            &[&format!("test://repo-{}", branch), &format!("test_repo_{}", branch)],
        )
        .await?
        .get(0);

    // Create a worktree
    let worktree_id: i64 = client
        .query_one(
            r#"
            INSERT INTO maproom.worktrees (repo_id, branch, root_path)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            &[&repo_id, &branch, &format!("/tmp/test-{}", branch)],
        )
        .await?
        .get(0);

    Ok(worktree_id)
}

// ============================================================================
// WORKTREE UPSERT TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires database
async fn test_insert_creates_single_worktree_array() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client).await.expect("Failed to run migrations");

    // Create test worktree
    let worktree_id = create_test_worktree(&client, "test-insert")
        .await
        .expect("Failed to create test worktree");

    // Create test chunk
    let chunk = ParsedChunk {
        relpath: "test/insert.rs".to_string(),
        symbol_name: Some("test_insert".to_string()),
        content: "fn test_insert() { println!(\"test\"); }".to_string(),
        start_line: 1,
        end_line: 1,
        kind: "function".to_string(),
    };

    let metrics = CacheMetrics::new();

    // Insert chunk
    let chunk_id = upsert_chunk_with_worktree(&client, &chunk, worktree_id, &metrics)
        .await
        .expect("Failed to upsert chunk");

    // Verify chunk was inserted with worktree_id in array
    let row = client
        .query_one(
            "SELECT worktree_ids FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await
        .expect("Failed to query chunk");

    let worktree_ids: serde_json::Value = row.get(0);
    let ids_array = worktree_ids.as_array().expect("worktree_ids should be array");

    assert_eq!(ids_array.len(), 1, "Should have exactly one worktree_id");
    assert_eq!(
        ids_array[0].as_i64().unwrap(),
        worktree_id,
        "worktree_ids array should contain the inserted worktree_id"
    );
}

#[tokio::test]
#[ignore] // Requires database
async fn test_upsert_is_idempotent() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client).await.expect("Failed to run migrations");

    // Create test worktree
    let worktree_id = create_test_worktree(&client, "test-idempotent")
        .await
        .expect("Failed to create test worktree");

    // Create test chunk
    let chunk = ParsedChunk {
        relpath: "test/idempotent.rs".to_string(),
        symbol_name: Some("test_fn".to_string()),
        content: "fn test_fn() { }".to_string(),
        start_line: 5,
        end_line: 5,
        kind: "function".to_string(),
    };

    let metrics = CacheMetrics::new();

    // First upsert
    let chunk_id_1 = upsert_chunk_with_worktree(&client, &chunk, worktree_id, &metrics)
        .await
        .expect("Failed to upsert chunk first time");

    // Second upsert with SAME worktree_id (should be idempotent)
    let chunk_id_2 = upsert_chunk_with_worktree(&client, &chunk, worktree_id, &metrics)
        .await
        .expect("Failed to upsert chunk second time");

    // Should return same chunk_id
    assert_eq!(chunk_id_1, chunk_id_2, "Chunk ID should be same for idempotent upsert");

    // Verify worktree_ids array has NO duplicates
    let row = client
        .query_one(
            "SELECT worktree_ids FROM maproom.chunks WHERE id = $1",
            &[&chunk_id_1],
        )
        .await
        .expect("Failed to query chunk");

    let worktree_ids: serde_json::Value = row.get(0);
    let ids_array = worktree_ids.as_array().expect("worktree_ids should be array");

    assert_eq!(
        ids_array.len(),
        1,
        "Array should still have exactly one entry (no duplicate)"
    );
    assert_eq!(
        ids_array[0].as_i64().unwrap(),
        worktree_id,
        "Array should contain the worktree_id"
    );
}

#[tokio::test]
#[ignore] // Requires database
async fn test_multi_worktree_scenario() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client).await.expect("Failed to run migrations");

    // Create three test worktrees (simulating main, feature-a, feature-b)
    let wt_main = create_test_worktree(&client, "main")
        .await
        .expect("Failed to create main worktree");
    let wt_feature_a = create_test_worktree(&client, "feature-a")
        .await
        .expect("Failed to create feature-a worktree");
    let wt_feature_b = create_test_worktree(&client, "feature-b")
        .await
        .expect("Failed to create feature-b worktree");

    // Create chunk with SAME content across all worktrees
    let chunk = ParsedChunk {
        relpath: "src/common.rs".to_string(),
        symbol_name: Some("common_fn".to_string()),
        content: "fn common_fn() { println!(\"shared code\"); }".to_string(),
        start_line: 10,
        end_line: 12,
        kind: "function".to_string(),
    };

    let metrics = CacheMetrics::new();

    // Upsert from main worktree
    let chunk_id_main = upsert_chunk_with_worktree(&client, &chunk, wt_main, &metrics)
        .await
        .expect("Failed to upsert from main");

    // Upsert from feature-a worktree (same content, different worktree)
    let chunk_id_a = upsert_chunk_with_worktree(&client, &chunk, wt_feature_a, &metrics)
        .await
        .expect("Failed to upsert from feature-a");

    // Upsert from feature-b worktree (same content, different worktree)
    let chunk_id_b = upsert_chunk_with_worktree(&client, &chunk, wt_feature_b, &metrics)
        .await
        .expect("Failed to upsert from feature-b");

    // All should return SAME chunk_id (blob_sha + relpath conflict)
    assert_eq!(chunk_id_main, chunk_id_a, "main and feature-a should get same chunk");
    assert_eq!(chunk_id_main, chunk_id_b, "main and feature-b should get same chunk");

    // Verify worktree_ids array contains all three worktrees
    let row = client
        .query_one(
            "SELECT worktree_ids FROM maproom.chunks WHERE id = $1",
            &[&chunk_id_main],
        )
        .await
        .expect("Failed to query chunk");

    let worktree_ids: serde_json::Value = row.get(0);
    let ids_array = worktree_ids.as_array().expect("worktree_ids should be array");

    assert_eq!(
        ids_array.len(),
        3,
        "Array should have all three worktree_ids"
    );

    // Convert to i64 vec for easier checking
    let ids: Vec<i64> = ids_array
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();

    assert!(ids.contains(&wt_main), "Array should contain main worktree");
    assert!(ids.contains(&wt_feature_a), "Array should contain feature-a worktree");
    assert!(ids.contains(&wt_feature_b), "Array should contain feature-b worktree");
}

#[tokio::test]
#[ignore] // Requires database
async fn test_different_content_creates_separate_chunks() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client).await.expect("Failed to run migrations");

    // Create test worktree
    let worktree_id = create_test_worktree(&client, "test-different")
        .await
        .expect("Failed to create test worktree");

    let metrics = CacheMetrics::new();

    // Create two chunks with DIFFERENT content but SAME relpath
    let chunk1 = ParsedChunk {
        relpath: "src/file.rs".to_string(),
        symbol_name: Some("fn_v1".to_string()),
        content: "fn fn_v1() { return 1; }".to_string(),
        start_line: 1,
        end_line: 1,
        kind: "function".to_string(),
    };

    let chunk2 = ParsedChunk {
        relpath: "src/file.rs".to_string(), // SAME relpath
        symbol_name: Some("fn_v2".to_string()),
        content: "fn fn_v2() { return 2; }".to_string(), // DIFFERENT content
        start_line: 5,
        end_line: 5,
        kind: "function".to_string(),
    };

    // Upsert both chunks
    let chunk_id_1 = upsert_chunk_with_worktree(&client, &chunk1, worktree_id, &metrics)
        .await
        .expect("Failed to upsert chunk1");

    let chunk_id_2 = upsert_chunk_with_worktree(&client, &chunk2, worktree_id, &metrics)
        .await
        .expect("Failed to upsert chunk2");

    // Different content = different blob_sha = different chunks
    assert_ne!(
        chunk_id_1, chunk_id_2,
        "Different content should create separate chunks"
    );

    // Verify both chunks have the worktree_id
    for chunk_id in [chunk_id_1, chunk_id_2] {
        let row = client
            .query_one(
                "SELECT worktree_ids FROM maproom.chunks WHERE id = $1",
                &[&chunk_id],
            )
            .await
            .expect("Failed to query chunk");

        let worktree_ids: serde_json::Value = row.get(0);
        let ids_array = worktree_ids.as_array().expect("worktree_ids should be array");

        assert_eq!(ids_array.len(), 1, "Each chunk should have one worktree_id");
        assert_eq!(
            ids_array[0].as_i64().unwrap(),
            worktree_id,
            "Chunk should contain the worktree_id"
        );
    }
}

#[tokio::test]
#[ignore] // Requires database
async fn test_cache_metrics_integration() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client).await.expect("Failed to run migrations");

    // Create test worktree
    let worktree_id = create_test_worktree(&client, "test-metrics")
        .await
        .expect("Failed to create test worktree");

    let metrics = CacheMetrics::new();

    let chunk = ParsedChunk {
        relpath: "src/metrics.rs".to_string(),
        symbol_name: Some("test_metrics".to_string()),
        content: "fn test_metrics() { }".to_string(),
        start_line: 1,
        end_line: 1,
        kind: "function".to_string(),
    };

    // First upsert: should be cache miss (new blob_sha)
    upsert_chunk_with_worktree(&client, &chunk, worktree_id, &metrics)
        .await
        .expect("Failed first upsert");

    assert_eq!(metrics.hits(), 0, "First upsert should be cache miss");
    assert_eq!(metrics.misses(), 1, "First upsert should record miss");

    // Note: Since we don't actually generate embeddings in this test,
    // the second upsert will still be a cache miss. In production with
    // full embedding pipeline, this would be a cache hit.
}
