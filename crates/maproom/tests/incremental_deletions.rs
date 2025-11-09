//! Integration tests for file deletion handling in incremental updates.
//!
//! Tests verify:
//! - remove_worktree_from_chunks removes worktree_id from JSONB array
//! - Idempotency: removing same worktree twice is safe (no-op)
//! - Multi-worktree: only removes specified worktree, leaves others intact
//! - Garbage collection: deletes chunks with empty worktree_ids arrays
//! - Non-existent file paths are handled gracefully (no error)
//!
//! Requirements:
//! - PostgreSQL with DATABASE_URL environment variable
//! - Migration 004 applied (worktree_ids column exists)
//!
//! Run with: cargo test --test incremental_deletions -- --ignored --nocapture

use crewchief_maproom::{
    db,
    incremental::remove_worktree_from_chunks,
    metrics::CacheMetrics,
    upsert::{upsert_chunk_with_worktree, ParsedChunk},
};

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
async fn create_test_worktree(
    client: &tokio_postgres::Client,
    branch: &str,
) -> anyhow::Result<i64> {
    // First, ensure we have a repo
    let repo_id: i64 = client
        .query_one(
            r#"
            INSERT INTO maproom.repos (url, name, primary_language)
            VALUES ($1, $2, 'rust')
            ON CONFLICT (url) DO UPDATE SET url = EXCLUDED.url
            RETURNING id
            "#,
            &[
                &format!("test://repo-{}", branch),
                &format!("test_repo_{}", branch),
            ],
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
// FILE DELETION TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires database
async fn test_remove_worktree_from_single_chunk() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    // Create test worktree
    let worktree_id = create_test_worktree(&client, "test-remove-single")
        .await
        .expect("Failed to create test worktree");

    let metrics = CacheMetrics::new();

    // Create and insert a test chunk
    let chunk = ParsedChunk {
        relpath: "src/to_delete.rs".to_string(),
        symbol_name: Some("to_delete_fn".to_string()),
        content: "fn to_delete_fn() { }".to_string(),
        start_line: 1,
        end_line: 1,
        kind: "function".to_string(),
    };

    let chunk_id = upsert_chunk_with_worktree(&client, &chunk, worktree_id, &metrics)
        .await
        .expect("Failed to insert chunk");

    // Verify chunk was inserted with worktree_id
    let row = client
        .query_one(
            "SELECT worktree_ids FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await
        .expect("Failed to query chunk");

    let worktree_ids: serde_json::Value = row.get(0);
    assert_eq!(
        worktree_ids.as_array().unwrap().len(),
        1,
        "Should have one worktree initially"
    );

    // Remove worktree from chunk
    let affected = remove_worktree_from_chunks(&client, worktree_id, &chunk.relpath)
        .await
        .expect("Failed to remove worktree");

    assert_eq!(affected, 1, "Should affect exactly one chunk");

    // Verify chunk was deleted (garbage collection)
    let count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await
        .expect("Failed to count chunks")
        .get(0);

    assert_eq!(
        count, 0,
        "Chunk should be deleted (empty worktree_ids array)"
    );
}

#[tokio::test]
#[ignore] // Requires database
async fn test_remove_worktree_is_idempotent() {
    let client = skip_if_no_db!();

    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    let worktree_id = create_test_worktree(&client, "test-remove-idempotent")
        .await
        .expect("Failed to create test worktree");

    let metrics = CacheMetrics::new();

    let chunk = ParsedChunk {
        relpath: "src/idempotent_delete.rs".to_string(),
        symbol_name: Some("idempotent_fn".to_string()),
        content: "fn idempotent_fn() { }".to_string(),
        start_line: 1,
        end_line: 1,
        kind: "function".to_string(),
    };

    upsert_chunk_with_worktree(&client, &chunk, worktree_id, &metrics)
        .await
        .expect("Failed to insert chunk");

    // First removal
    let affected1 = remove_worktree_from_chunks(&client, worktree_id, &chunk.relpath)
        .await
        .expect("Failed first remove");

    assert_eq!(affected1, 1, "First removal should affect 1 chunk");

    // Second removal (idempotent - should be no-op)
    let affected2 = remove_worktree_from_chunks(&client, worktree_id, &chunk.relpath)
        .await
        .expect("Failed second remove");

    assert_eq!(
        affected2, 0,
        "Second removal should affect 0 chunks (already removed)"
    );
}

#[tokio::test]
#[ignore] // Requires database
async fn test_remove_one_worktree_leaves_others() {
    let client = skip_if_no_db!();

    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    // Create three worktrees
    let wt_main = create_test_worktree(&client, "main-multi-delete")
        .await
        .expect("Failed to create main worktree");
    let wt_feature_a = create_test_worktree(&client, "feature-a-multi-delete")
        .await
        .expect("Failed to create feature-a worktree");
    let wt_feature_b = create_test_worktree(&client, "feature-b-multi-delete")
        .await
        .expect("Failed to create feature-b worktree");

    let metrics = CacheMetrics::new();

    // Create chunk with SAME content across all worktrees
    let chunk = ParsedChunk {
        relpath: "src/shared_delete.rs".to_string(),
        symbol_name: Some("shared_fn".to_string()),
        content: "fn shared_fn() { println!(\"shared\"); }".to_string(),
        start_line: 1,
        end_line: 1,
        kind: "function".to_string(),
    };

    // Add to all three worktrees
    let chunk_id = upsert_chunk_with_worktree(&client, &chunk, wt_main, &metrics)
        .await
        .expect("Failed to upsert from main");
    upsert_chunk_with_worktree(&client, &chunk, wt_feature_a, &metrics)
        .await
        .expect("Failed to upsert from feature-a");
    upsert_chunk_with_worktree(&client, &chunk, wt_feature_b, &metrics)
        .await
        .expect("Failed to upsert from feature-b");

    // Verify all three worktrees are in array
    let row = client
        .query_one(
            "SELECT worktree_ids FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await
        .expect("Failed to query chunk");

    let worktree_ids: serde_json::Value = row.get(0);
    assert_eq!(
        worktree_ids.as_array().unwrap().len(),
        3,
        "Should have three worktrees"
    );

    // Remove ONE worktree (feature-a)
    let affected = remove_worktree_from_chunks(&client, wt_feature_a, &chunk.relpath)
        .await
        .expect("Failed to remove feature-a");

    assert_eq!(affected, 1, "Should affect one chunk");

    // Verify chunk still exists with two worktrees
    let row = client
        .query_one(
            "SELECT worktree_ids FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await
        .expect("Failed to query chunk after removal");

    let worktree_ids: serde_json::Value = row.get(0);
    let ids_array = worktree_ids.as_array().unwrap();

    assert_eq!(
        ids_array.len(),
        2,
        "Should have two worktrees remaining after removal"
    );

    // Convert to i64 vec for easier checking
    let ids: Vec<i64> = ids_array.iter().map(|v| v.as_i64().unwrap()).collect();

    assert!(ids.contains(&wt_main), "Should still contain main");
    assert!(
        ids.contains(&wt_feature_b),
        "Should still contain feature-b"
    );
    assert!(
        !ids.contains(&wt_feature_a),
        "Should NOT contain feature-a (removed)"
    );
}

#[tokio::test]
#[ignore] // Requires database
async fn test_garbage_collection_deletes_orphans() {
    let client = skip_if_no_db!();

    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    let worktree_id = create_test_worktree(&client, "test-gc")
        .await
        .expect("Failed to create test worktree");

    let metrics = CacheMetrics::new();

    // Create chunk with ONLY one worktree
    let chunk = ParsedChunk {
        relpath: "src/orphan.rs".to_string(),
        symbol_name: Some("orphan_fn".to_string()),
        content: "fn orphan_fn() { }".to_string(),
        start_line: 1,
        end_line: 1,
        kind: "function".to_string(),
    };

    let chunk_id = upsert_chunk_with_worktree(&client, &chunk, worktree_id, &metrics)
        .await
        .expect("Failed to insert chunk");

    // Remove the ONLY worktree (should trigger garbage collection)
    let affected = remove_worktree_from_chunks(&client, worktree_id, &chunk.relpath)
        .await
        .expect("Failed to remove worktree");

    assert_eq!(affected, 1, "Should affect one chunk");

    // Verify chunk was DELETED (orphan with empty worktree_ids)
    let count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE id = $1",
            &[&chunk_id],
        )
        .await
        .expect("Failed to count chunks")
        .get(0);

    assert_eq!(count, 0, "Orphan chunk should be garbage collected");
}

#[tokio::test]
#[ignore] // Requires database
async fn test_remove_nonexistent_file_is_noop() {
    let client = skip_if_no_db!();

    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    let worktree_id = create_test_worktree(&client, "test-nonexistent")
        .await
        .expect("Failed to create test worktree");

    // Try to remove worktree from file that doesn't exist
    let result = remove_worktree_from_chunks(&client, worktree_id, "src/does_not_exist.rs").await;

    // Should succeed (no-op)
    assert!(
        result.is_ok(),
        "Removing from non-existent file should not error"
    );
    assert_eq!(result.unwrap(), 0, "Should affect 0 chunks (no such file)");
}

#[tokio::test]
#[ignore] // Requires database
async fn test_remove_multiple_chunks_in_file() {
    let client = skip_if_no_db!();

    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    let worktree_id = create_test_worktree(&client, "test-multi-chunk")
        .await
        .expect("Failed to create test worktree");

    let metrics = CacheMetrics::new();

    let file_path = "src/multi_chunk.rs";

    // Create THREE chunks in the same file
    for i in 1..=3 {
        let chunk = ParsedChunk {
            relpath: file_path.to_string(),
            symbol_name: Some(format!("fn_{}", i)),
            content: format!("fn fn_{}() {{ }}", i),
            start_line: i,
            end_line: i,
            kind: "function".to_string(),
        };

        upsert_chunk_with_worktree(&client, &chunk, worktree_id, &metrics)
            .await
            .expect("Failed to insert chunk");
    }

    // Verify 3 chunks exist for this file
    let count_before: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE relpath = $1",
            &[&file_path],
        )
        .await
        .expect("Failed to count chunks")
        .get(0);

    assert_eq!(count_before, 3, "Should have 3 chunks initially");

    // Remove worktree from ALL chunks in this file
    let affected = remove_worktree_from_chunks(&client, worktree_id, file_path)
        .await
        .expect("Failed to remove worktree");

    assert_eq!(affected, 3, "Should affect all 3 chunks");

    // Verify all chunks were deleted (garbage collection)
    let count_after: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE relpath = $1",
            &[&file_path],
        )
        .await
        .expect("Failed to count chunks")
        .get(0);

    assert_eq!(count_after, 0, "All chunks should be deleted (orphans)");
}
