//! Test fixture generator for SQLite E2E tests.
//!
//! Run with: cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture
//!
//! This creates a pre-indexed SQLite database at tests/fixtures/pre-indexed-maproom.db
//! with sample repositories, worktrees, and chunks for E2E testing.

#[cfg(feature = "sqlite")]
mod fixture {
    use crewchief_maproom::db::sqlite::SqliteStore;
    use crewchief_maproom::db::{ChunkRecord, FileRecord, VectorStore};
    use std::path::Path;

    #[tokio::test]
    #[ignore] // Run manually: cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture
    async fn create_test_fixture() {
        let fixture_path = "tests/fixtures/pre-indexed-maproom.db";

        // Remove existing fixture if present
        if Path::new(fixture_path).exists() {
            std::fs::remove_file(fixture_path).expect("Failed to remove existing fixture");
        }

        // Connect to SQLite (creates the database)
        let store = SqliteStore::connect(fixture_path)
            .await
            .expect("Failed to connect to SQLite");

        // Run migrations
        store.migrate().await.expect("Failed to run migrations");

        // Create test repository
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/repo/path")
            .await
            .expect("Failed to create repo");

        // Create test worktree
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/repo/path")
            .await
            .expect("Failed to create worktree");

        // Create test commit
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123def456", None)
            .await
            .expect("Failed to create commit");

        // Create test file
        let file_record = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "main.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash123".to_string(),
            size_bytes: 500,
            last_modified: None,
        };
        let file_id = store
            .upsert_file(&file_record)
            .await
            .expect("Failed to create file");

        // Create test chunks with various types
        let chunks = vec![
            ChunkRecord {
                file_id,
                blob_sha: "blob1".to_string(),
                symbol_name: Some("main".to_string()),
                kind: "function".to_string(),
                signature: Some("fn main()".to_string()),
                docstring: Some("Entry point".to_string()),
                start_line: 1,
                end_line: 5,
                preview: "fn main() { ... }".to_string(),
                ts_doc_text: "main function entry point hello world".to_string(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
                worktree_id,
            },
            ChunkRecord {
                file_id,
                blob_sha: "blob2".to_string(),
                symbol_name: Some("helper_function".to_string()),
                kind: "function".to_string(),
                signature: Some("fn helper_function() -> i32".to_string()),
                docstring: None,
                start_line: 7,
                end_line: 10,
                preview: "fn helper_function() -> i32 { 42 }".to_string(),
                ts_doc_text: "helper function returns integer".to_string(),
                recency_score: 0.8,
                churn_score: 0.3,
                metadata: None,
                worktree_id,
            },
            ChunkRecord {
                file_id,
                blob_sha: "blob3".to_string(),
                symbol_name: Some("Config".to_string()),
                kind: "struct".to_string(),
                signature: Some("struct Config".to_string()),
                docstring: Some("Configuration struct".to_string()),
                start_line: 12,
                end_line: 16,
                preview: "struct Config { name: String, value: i32 }".to_string(),
                ts_doc_text: "config configuration struct name value".to_string(),
                recency_score: 0.9,
                churn_score: 0.2,
                metadata: None,
                worktree_id,
            },
        ];

        // Insert chunks
        for chunk in chunks {
            store
                .insert_chunk(&chunk)
                .await
                .expect("Failed to insert chunk");
        }

        println!("Created test fixture at {}", fixture_path);
        println!("  Repository: test-repo");
        println!("  Worktree: main");
        println!("  Chunks: 3");
    }
}

#[cfg(not(feature = "sqlite"))]
fn main() {
    println!("SQLite feature not enabled");
}
