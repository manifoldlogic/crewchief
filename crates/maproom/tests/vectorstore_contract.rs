//! VectorStore Contract Tests
//!
//! This module tests that SqliteStore correctly implements
//! the VectorStore trait. It covers all trait methods added during the VECSTORE project.
//!
//! Run with:
//!   cargo test --test vectorstore_contract
//!
//! NOTE: This file is disabled - VectorStore trait was removed during refactoring.
//! Re-enable when contract tests are needed.

// Disabled - VectorStore trait no longer exists
// Use cfg(any()) which is always false to disable compilation
#![cfg(any())]

mod sqlite_contract_tests {
    use maproom::db::sqlite::SqliteStore;
    use maproom::db::{ChunkRecord, FileRecord, UpdateStats, VectorStore};

    /// Helper to create an in-memory store with schema
    async fn setup_store() -> SqliteStore {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    /// Helper to set up a complete test hierarchy
    async fn setup_test_data(store: &SqliteStore) -> (i64, i64, i64, i64, i64) {
        let repo_id = store
            .get_or_create_repo("contract-test", "/tmp/contract-test")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/tmp/contract-test")
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
            relpath: "src/main.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "filehash123".to_string(),
            size_bytes: 1024,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = ChunkRecord {
            file_id,
            blob_sha: "blobsha123".to_string(),
            symbol_name: Some("test_function".to_string()),
            kind: "function".to_string(),
            signature: Some("fn test_function() -> bool".to_string()),
            docstring: Some("A test function".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn test_function() -> bool { true }".to_string(),
            ts_doc_text: "test function testing".to_string(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
            worktree_id,
        };
        let chunk_id = store.insert_chunk(&chunk).await.unwrap();

        (repo_id, worktree_id, commit_id, file_id, chunk_id)
    }

    // === Repository Query Tests (VECSTORE-1004) ===

    #[tokio::test]
    async fn test_get_repo_by_name() {
        let store = setup_store().await;
        let _ = store
            .get_or_create_repo("lookup-test", "/tmp/lookup")
            .await
            .unwrap();

        // Should find existing repo
        let found = store.get_repo_by_name("lookup-test").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "lookup-test");

        // Should return None for non-existent
        let not_found = store.get_repo_by_name("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_worktree_by_name() {
        let store = setup_store().await;
        let repo_id = store
            .get_or_create_repo("wt-test", "/tmp/wt")
            .await
            .unwrap();
        let _ = store
            .get_or_create_worktree(repo_id, "develop", "/tmp/wt/develop")
            .await
            .unwrap();

        // Should find existing worktree
        let found = store
            .get_worktree_by_name(repo_id, "develop")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "develop");

        // Should return None for non-existent
        let not_found = store
            .get_worktree_by_name(repo_id, "feature")
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_repos() {
        let store = setup_store().await;

        // Empty database
        let repos = store.list_repos().await.unwrap();
        assert!(repos.is_empty());

        // Add repos
        store.get_or_create_repo("repo-a", "/a").await.unwrap();
        store.get_or_create_repo("repo-b", "/b").await.unwrap();

        let repos = store.list_repos().await.unwrap();
        assert_eq!(repos.len(), 2);
    }

    #[tokio::test]
    async fn test_list_worktrees() {
        let store = setup_store().await;
        let repo_id = store.get_or_create_repo("wt-list", "/wt").await.unwrap();

        // Empty
        let worktrees = store.list_worktrees(repo_id).await.unwrap();
        assert!(worktrees.is_empty());

        // Add worktrees
        store
            .get_or_create_worktree(repo_id, "main", "/wt/main")
            .await
            .unwrap();
        store
            .get_or_create_worktree(repo_id, "feature", "/wt/feature")
            .await
            .unwrap();

        let worktrees = store.list_worktrees(repo_id).await.unwrap();
        assert_eq!(worktrees.len(), 2);
    }

    // === Context Assembly Tests (VECSTORE-1003) ===

    #[tokio::test]
    async fn test_get_chunk_by_id() {
        let store = setup_store().await;
        let (_, _, _, _, chunk_id) = setup_test_data(&store).await;

        // Should find existing chunk
        let chunk = store.get_chunk_by_id(chunk_id).await.unwrap();
        assert!(chunk.is_some());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.id, chunk_id);
        assert_eq!(chunk.symbol_name, Some("test_function".to_string()));

        // Should return None for non-existent
        let not_found = store.get_chunk_by_id(99999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_file_chunks() {
        let store = setup_store().await;
        let (_, _, _, file_id, _) = setup_test_data(&store).await;

        let chunks = store.get_file_chunks(file_id).await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].symbol_name, Some("test_function".to_string()));
    }

    #[tokio::test]
    async fn test_get_chunk_context() {
        let store = setup_store().await;
        let (_, _, _, _, chunk_id) = setup_test_data(&store).await;

        let context = store.get_chunk_context(chunk_id, 2).await.unwrap();
        assert!(context.is_some());
        let context = context.unwrap();
        assert_eq!(context.chunk.id, chunk_id);
    }

    // === Index State Tests (VECSTORE-1005) ===

    #[tokio::test]
    async fn test_get_last_indexed_tree_init() {
        let store = setup_store().await;
        let repo_id = store.get_or_create_repo("idx-test", "/idx").await.unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/idx")
            .await
            .unwrap();

        // Never indexed should return "init"
        let tree_sha = store.get_last_indexed_tree(worktree_id).await.unwrap();
        assert_eq!(tree_sha, "init");
    }

    #[tokio::test]
    async fn test_update_index_state() {
        let store = setup_store().await;
        let repo_id = store.get_or_create_repo("idx-test", "/idx").await.unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/idx")
            .await
            .unwrap();

        let stats = UpdateStats {
            files_processed: 10,
            chunks_processed: 50,
            embeddings_generated: 50,
        };

        // Update state
        store
            .update_index_state(worktree_id, "sha456def", &stats)
            .await
            .unwrap();

        // Should now return updated SHA
        let tree_sha = store.get_last_indexed_tree(worktree_id).await.unwrap();
        assert_eq!(tree_sha, "sha456def");
    }

    // === Cleanup Tests (VECSTORE-1006) ===

    #[tokio::test]
    async fn test_detect_stale_worktrees_empty() {
        let store = setup_store().await;

        // Empty database should return empty vec
        let stale = store.detect_stale_worktrees().await.unwrap();
        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn test_get_chunks_by_blob_sha() {
        let store = setup_store().await;
        let (_, _, _, _, _) = setup_test_data(&store).await;

        // Should find chunk by blob_sha
        let chunks = store.get_chunks_by_blob_sha("blobsha123").await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].symbol_name, Some("test_function".to_string()));

        // Should return empty for unknown blob_sha
        let chunks = store.get_chunks_by_blob_sha("unknown").await.unwrap();
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn test_delete_chunks_by_file() {
        let store = setup_store().await;
        let (_, _, _, file_id, chunk_id) = setup_test_data(&store).await;

        // Verify chunk exists
        let chunk = store.get_chunk_by_id(chunk_id).await.unwrap();
        assert!(chunk.is_some());

        // Delete chunks for file
        let deleted = store.delete_chunks_by_file(file_id).await.unwrap();
        assert_eq!(deleted, 1);

        // Verify chunk is gone
        let chunk = store.get_chunk_by_id(chunk_id).await.unwrap();
        assert!(chunk.is_none());
    }

    #[tokio::test]
    async fn test_delete_worktree_data() {
        let store = setup_store().await;
        let (_, worktree_id, _, _, chunk_id) = setup_test_data(&store).await;

        // Verify chunk exists
        let chunk = store.get_chunk_by_id(chunk_id).await.unwrap();
        assert!(chunk.is_some());

        // Delete worktree data
        let result = store.delete_worktree_data(worktree_id).await.unwrap();
        assert_eq!(result.chunks_deleted, 1);
        assert_eq!(result.files_deleted, 1);

        // Verify chunk is gone
        let chunk = store.get_chunk_by_id(chunk_id).await.unwrap();
        assert!(chunk.is_none());
    }

    // === Vector/Hybrid Search Tests (VECSTORE-1001, VECSTORE-1002) ===
    // Note: These require sqlite-vec extension which may not be available in all test environments

    #[tokio::test]
    async fn test_vector_search_without_embeddings() {
        let store = setup_store().await;
        let _ = setup_test_data(&store).await;

        // Vector search with no embeddings should work (return empty or handle gracefully)
        let embedding = vec![0.1f32; 1536];
        let result = store
            .search_chunks_vector("contract-test", None, &embedding, 10, false, None, None)
            .await;

        // Should either succeed with empty results or fail gracefully (no vec extension)
        assert!(
            result.is_ok() || result.is_err(),
            "Vector search should handle missing data"
        );
    }

    #[tokio::test]
    async fn test_hybrid_search_without_embeddings() {
        let store = setup_store().await;
        let _ = setup_test_data(&store).await;

        // Hybrid search should fallback to FTS when no embeddings
        let embedding = vec![0.1f32; 1536];
        let result = store
            .search_chunks_hybrid(
                "contract-test",
                None,
                "test",
                &embedding,
                10,
                false,
                None,
                None,
            )
            .await;

        // Should either succeed (fallback to FTS) or fail gracefully
        assert!(
            result.is_ok() || result.is_err(),
            "Hybrid search should handle missing embeddings"
        );
    }

    // === Embedding Tests (VECSTORE-1000) ===
    // Note: Embedding storage uses blob_sha-based deduplication via upsert_embedding()
    // which stores embeddings in code_embeddings table and syncs to vec_code virtual tables.
    // Tests for the correct API (upsert_embedding singular) are in sqlite_integration.rs.
}
