//! Integration tests for worktree index state database functions.
//!
//! Tests verify:
//! - get_last_indexed_tree returns "init" for non-existent worktrees
//! - update_index_state inserts new records
//! - update_index_state updates existing records (upsert)
//! - Metrics are stored correctly
//! - Timestamps are updated on each update
//!
//! Requirements:
//! - PostgreSQL with MAPROOM_DATABASE_URL environment variable
//! - Migration 004 applied (worktree_index_state table exists)
//!
//! Run with: cargo test --test index_state -- --ignored --nocapture

use crewchief_maproom::db;
use crewchief_maproom::db::{get_last_indexed_tree, update_index_state, UpdateStats};

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
                eprintln!("Skipping test: MAPROOM_DATABASE_URL not set or connection failed");
                return;
            }
        }
    };
}

/// Helper to create a test worktree in the database.
///
/// Returns the worktree ID for testing.
async fn create_test_worktree(client: &tokio_postgres::Client) -> anyhow::Result<i64> {
    // First, ensure we have a repo
    let repo_id: i64 = client
        .query_one(
            r#"
            INSERT INTO maproom.repos (url, name, primary_language)
            VALUES ('test://repo', 'test_repo', 'rust')
            ON CONFLICT (url) DO UPDATE SET url = EXCLUDED.url
            RETURNING id
            "#,
            &[],
        )
        .await?
        .get(0);

    // Create a worktree
    let worktree_id: i64 = client
        .query_one(
            r#"
            INSERT INTO maproom.worktrees (repo_id, branch, root_path)
            VALUES ($1, 'test-branch', '/tmp/test')
            RETURNING id
            "#,
            &[&repo_id],
        )
        .await?
        .get(0);

    Ok(worktree_id)
}

// ============================================================================
// INDEX STATE TESTS
// ============================================================================

#[tokio::test]
#[ignore] // Requires database
async fn test_get_last_indexed_tree_returns_init() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    // Query for a non-existent worktree ID (very high ID unlikely to exist)
    let result = get_last_indexed_tree(&client, 999999999)
        .await
        .expect("Failed to get last indexed tree");

    assert_eq!(
        result, "init",
        "Should return 'init' for worktrees that have never been indexed"
    );
}

#[tokio::test]
#[ignore] // Requires database
async fn test_update_and_retrieve_index_state() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    // Create a test worktree
    let worktree_id = create_test_worktree(&client)
        .await
        .expect("Failed to create test worktree");

    // Create update stats
    let stats = UpdateStats {
        files_processed: 10,
        chunks_processed: 100,
        embeddings_generated: 50,
    };

    // Update index state (INSERT path)
    update_index_state(&client, worktree_id, "abc123def456", &stats)
        .await
        .expect("Failed to update index state");

    // Retrieve the state
    let retrieved = get_last_indexed_tree(&client, worktree_id)
        .await
        .expect("Failed to get last indexed tree");

    assert_eq!(
        retrieved, "abc123def456",
        "Retrieved tree SHA should match inserted value"
    );

    // Verify metrics were stored correctly
    let row = client
        .query_one(
            r#"
            SELECT chunks_processed, embeddings_generated
            FROM maproom.worktree_index_state
            WHERE worktree_id = $1
            "#,
            &[&worktree_id],
        )
        .await
        .expect("Failed to query metrics");

    let chunks: i32 = row.get(0);
    let embeddings: i32 = row.get(1);

    assert_eq!(chunks, 100, "chunks_processed should be stored");
    assert_eq!(embeddings, 50, "embeddings_generated should be stored");
}

#[tokio::test]
#[ignore] // Requires database
async fn test_update_index_state_upsert_behavior() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    // Create a test worktree
    let worktree_id = create_test_worktree(&client)
        .await
        .expect("Failed to create test worktree");

    // First update (INSERT path)
    let stats1 = UpdateStats {
        files_processed: 10,
        chunks_processed: 100,
        embeddings_generated: 50,
    };

    update_index_state(&client, worktree_id, "tree_sha_1", &stats1)
        .await
        .expect("Failed to insert index state");

    // Second update (UPDATE path - upsert)
    let stats2 = UpdateStats {
        files_processed: 20,
        chunks_processed: 200,
        embeddings_generated: 100,
    };

    update_index_state(&client, worktree_id, "tree_sha_2", &stats2)
        .await
        .expect("Failed to update index state");

    // Retrieve the final state
    let retrieved_sha = get_last_indexed_tree(&client, worktree_id)
        .await
        .expect("Failed to get last indexed tree");

    assert_eq!(
        retrieved_sha, "tree_sha_2",
        "Tree SHA should be updated to latest value"
    );

    // Verify metrics were updated
    let row = client
        .query_one(
            r#"
            SELECT chunks_processed, embeddings_generated
            FROM maproom.worktree_index_state
            WHERE worktree_id = $1
            "#,
            &[&worktree_id],
        )
        .await
        .expect("Failed to query metrics");

    let chunks: i32 = row.get(0);
    let embeddings: i32 = row.get(1);

    assert_eq!(chunks, 200, "chunks_processed should be updated");
    assert_eq!(embeddings, 100, "embeddings_generated should be updated");
}

#[tokio::test]
#[ignore] // Requires database
async fn test_update_index_state_updates_timestamp() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    // Create a test worktree
    let worktree_id = create_test_worktree(&client)
        .await
        .expect("Failed to create test worktree");

    // First update
    let stats = UpdateStats {
        files_processed: 10,
        chunks_processed: 100,
        embeddings_generated: 50,
    };

    update_index_state(&client, worktree_id, "tree_sha_1", &stats)
        .await
        .expect("Failed to insert index state");

    // Get first timestamp
    let row1 = client
        .query_one(
            r#"
            SELECT last_indexed
            FROM maproom.worktree_index_state
            WHERE worktree_id = $1
            "#,
            &[&worktree_id],
        )
        .await
        .expect("Failed to query first timestamp");

    let timestamp1: chrono::NaiveDateTime = row1.get(0);

    // Wait a bit to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Second update
    update_index_state(&client, worktree_id, "tree_sha_2", &stats)
        .await
        .expect("Failed to update index state");

    // Get second timestamp
    let row2 = client
        .query_one(
            r#"
            SELECT last_indexed
            FROM maproom.worktree_index_state
            WHERE worktree_id = $1
            "#,
            &[&worktree_id],
        )
        .await
        .expect("Failed to query second timestamp");

    let timestamp2: chrono::NaiveDateTime = row2.get(0);

    assert!(
        timestamp2 > timestamp1,
        "last_indexed timestamp should be updated on each update"
    );
}

#[tokio::test]
#[ignore] // Requires database
async fn test_update_stats_with_large_values() {
    let client = skip_if_no_db!();

    // Run migrations
    db::migrate(&client)
        .await
        .expect("Failed to run migrations");

    // Create a test worktree
    let worktree_id = create_test_worktree(&client)
        .await
        .expect("Failed to create test worktree");

    // Test with large values (typical for monorepos)
    let stats = UpdateStats {
        files_processed: 50000,
        chunks_processed: 500000,
        embeddings_generated: 250000,
    };

    update_index_state(&client, worktree_id, "large_repo_tree_sha", &stats)
        .await
        .expect("Failed to update with large values");

    // Verify large values stored correctly
    let row = client
        .query_one(
            r#"
            SELECT chunks_processed, embeddings_generated
            FROM maproom.worktree_index_state
            WHERE worktree_id = $1
            "#,
            &[&worktree_id],
        )
        .await
        .expect("Failed to query large values");

    let chunks: i32 = row.get(0);
    let embeddings: i32 = row.get(1);

    assert_eq!(chunks, 500000, "Large chunk count should be stored");
    assert_eq!(embeddings, 250000, "Large embedding count should be stored");
}
