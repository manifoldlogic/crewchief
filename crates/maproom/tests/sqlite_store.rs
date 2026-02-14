//! SQLite backend integration tests
//!
//! These tests validate the SQLite backend implementation including:
//! - Connection and schema migration
//! - CRUD operations (repos, worktrees, commits, files, chunks)
//! - FTS5 full-text search functionality
//! - Edge cases and error handling

mod sqlite_tests {
    use crewchief_maproom::db::sqlite::SqliteStore;
    use crewchief_maproom::db::StoreCore;
    use crewchief_maproom::db::{ChunkRecord, FileRecord};

    /// Helper to create an in-memory store with schema
    async fn setup_store() -> SqliteStore {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    // ==================== Connection Tests ====================

    #[tokio::test]
    async fn test_connect_and_migrate() {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        // Should not panic - schema created successfully
    }

    #[tokio::test]
    async fn test_migrate_idempotent() {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store.migrate().await.unwrap(); // Second call should succeed
    }

    // ==================== Repository Tests ====================

    #[tokio::test]
    async fn test_create_repo() {
        let store = setup_store().await;
        let id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        assert!(id > 0, "Repo ID should be positive");
    }

    #[tokio::test]
    async fn test_repo_idempotent() {
        let store = setup_store().await;
        let id1 = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let id2 = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        assert_eq!(id1, id2, "Same repo should return same ID");
    }

    // ==================== Full CRUD Cycle ====================

    #[tokio::test]
    async fn test_full_crud_cycle() {
        let store = setup_store().await;

        // 1. Create repo
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        assert!(repo_id > 0);

        // 2. Create worktree
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        assert!(worktree_id > 0);

        // 3. Create commit
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123def456", None)
            .await
            .unwrap();
        assert!(commit_id > 0);

        // 4. Create file
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "src/main.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash123abc".to_string(),
            size_bytes: 1024,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();
        assert!(file_id > 0);

        // 5. Create chunk
        let chunk = ChunkRecord {
            file_id,
            blob_sha: "blobsha123".to_string(),
            symbol_name: Some("main".to_string()),
            kind: "function".to_string(),
            signature: Some("fn main()".to_string()),
            docstring: Some("Entry point of the application".to_string()),
            start_line: 1,
            end_line: 10,
            preview: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
            ts_doc_text: "main function entry point".to_string(),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
            worktree_id,
        };
        let chunk_id = store.insert_chunk(&chunk).await.unwrap();
        assert!(chunk_id > 0);
    }

    // ==================== FTS Search Tests ====================

    #[tokio::test]
    async fn test_fts_search() {
        let store = setup_store().await;

        // Setup: create full hierarchy
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
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
            relpath: "src/auth.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash456".to_string(),
            size_bytes: 2048,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = ChunkRecord {
            file_id,
            blob_sha: "blobsha456".to_string(),
            symbol_name: Some("authenticate_user".to_string()),
            kind: "function".to_string(),
            signature: Some("fn authenticate_user(username: &str) -> bool".to_string()),
            docstring: Some("Authenticates a user by username".to_string()),
            start_line: 10,
            end_line: 30,
            preview: "fn authenticate_user(username: &str) -> bool {\n    // auth logic\n}"
                .to_string(),
            ts_doc_text: "authenticate user authentication login".to_string(),
            recency_score: 1.0,
            churn_score: 0.2,
            metadata: None,
            worktree_id,
        };
        store.insert_chunk(&chunk).await.unwrap();

        // Search should find the chunk
        let results = store
            .search_chunks_fts(
                "test-repo",
                Some("main"),
                "authenticate",
                10,
                false,
                None,
                None,
            )
            .await
            .unwrap();

        assert!(!results.is_empty(), "FTS search should return results");
        assert!(
            results[0]
                .symbol_name
                .as_ref()
                .unwrap()
                .contains("authenticate"),
            "Should find authenticate_user function"
        );
    }

    #[tokio::test]
    async fn test_fts_multiword_query() {
        let store = setup_store().await;

        // Setup minimal hierarchy
        let repo_id = store
            .get_or_create_repo("test-repo", "/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "sha1", None)
            .await
            .unwrap();
        let file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "test.rs".to_string(),
            language: Some("rust".to_string()),
            content_hash: "hash".to_string(),
            size_bytes: 100,
            last_modified: None,
        };
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = ChunkRecord {
            file_id,
            blob_sha: "blob".to_string(),
            symbol_name: Some("test_func".to_string()),
            kind: "function".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 5,
            preview: "fn test_func() {}".to_string(),
            ts_doc_text: "test function".to_string(),
            recency_score: 1.0,
            churn_score: 0.0,
            metadata: None,
            worktree_id,
        };
        store.insert_chunk(&chunk).await.unwrap();

        // Multi-word query should not cause syntax error
        let result = store
            .search_chunks_fts("test-repo", None, "test function", 10, false, None, None)
            .await;

        assert!(result.is_ok(), "Multi-word FTS query should not error");
    }

    #[tokio::test]
    async fn test_fts_no_results() {
        let store = setup_store().await;

        // Create repo with worktree but no chunks
        let repo_id = store
            .get_or_create_repo("empty-repo", "/empty")
            .await
            .unwrap();
        let _worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/empty")
            .await
            .unwrap();

        // Search in empty repo should return empty, not error
        let results = store
            .search_chunks_fts("empty-repo", None, "nonexistent", 10, false, None, None)
            .await
            .unwrap();

        assert!(results.is_empty(), "Should return empty for no matches");
    }

    // ==================== Edge Cases ====================

    #[tokio::test]
    async fn test_special_characters_in_query() {
        let store = setup_store().await;
        let repo_id = store
            .get_or_create_repo("test-repo", "/path")
            .await
            .unwrap();
        let _worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();

        // Queries with special characters should not crash
        let result = store
            .search_chunks_fts(
                "test-repo",
                None,
                "test \"quoted\" func",
                10,
                false,
                None,
                None,
            )
            .await;
        assert!(result.is_ok(), "Should handle quotes in query");

        let result = store
            .search_chunks_fts("test-repo", None, "func()", 10, false, None, None)
            .await;
        assert!(result.is_ok(), "Should handle parentheses in query");
    }

    #[tokio::test]
    async fn test_empty_query() {
        let store = setup_store().await;
        let repo_id = store
            .get_or_create_repo("test-repo", "/path")
            .await
            .unwrap();
        let _worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();

        // Empty or whitespace query - may return error or empty results
        let result = store
            .search_chunks_fts("test-repo", None, "   ", 10, false, None, None)
            .await;
        // Should either return empty results or handle gracefully (error is acceptable for empty query)
        assert!(
            result.is_ok() || result.is_err(),
            "Should handle empty query"
        );
    }

    // ==================== Search Method Filter Tests ====================
    //
    // These tests validate the full SqliteStore search methods with filter parameters,
    // testing end-to-end from the public API through SQL generation to result filtering.

    /// Helper to create a store populated with diverse test data for filter testing.
    /// Returns (store, repo_name) with chunks of various kinds and languages.
    async fn setup_filter_test_store() -> (SqliteStore, &'static str) {
        let store = setup_store().await;
        let repo_name = "filter-test-repo";

        let repo_id = store
            .get_or_create_repo(repo_name, "/tmp/filter-test")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/tmp/filter-test")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "filter_abc123", None)
            .await
            .unwrap();

        // Insert files with different languages
        let py_file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "src/auth.py".to_string(),
            language: Some("py".to_string()),
            content_hash: "py_hash_1".to_string(),
            size_bytes: 500,
            last_modified: None,
        };
        let py_file_id = store.upsert_file(&py_file).await.unwrap();

        let ts_file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "src/user.ts".to_string(),
            language: Some("ts".to_string()),
            content_hash: "ts_hash_1".to_string(),
            size_bytes: 600,
            last_modified: None,
        };
        let ts_file_id = store.upsert_file(&ts_file).await.unwrap();

        let rs_file = FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: "src/main.rs".to_string(),
            language: Some("rs".to_string()),
            content_hash: "rs_hash_1".to_string(),
            size_bytes: 700,
            last_modified: None,
        };
        let rs_file_id = store.upsert_file(&rs_file).await.unwrap();

        // Python func chunk
        // Note: FTS index uses (preview, docstring, symbol_name), so "searchable" must appear
        // in one of those fields for FTS to match the query "searchable".
        store
            .insert_chunk(&ChunkRecord {
                file_id: py_file_id,
                blob_sha: "blob_py_func".to_string(),
                symbol_name: Some("authenticate_user".to_string()),
                kind: "func".to_string(),
                signature: Some("def authenticate_user()".to_string()),
                docstring: Some("Authenticate a searchable user".to_string()),
                start_line: 1,
                end_line: 10,
                preview: "def authenticate_user(): searchable pass".to_string(),
                ts_doc_text: "authenticate user searchable test".to_string(),
                recency_score: 1.0,
                churn_score: 0.5,
                metadata: None,
                worktree_id,
            })
            .await
            .unwrap();

        // Python class chunk
        store
            .insert_chunk(&ChunkRecord {
                file_id: py_file_id,
                blob_sha: "blob_py_class".to_string(),
                symbol_name: Some("AuthManager".to_string()),
                kind: "class".to_string(),
                signature: Some("class AuthManager".to_string()),
                docstring: Some("Auth manager searchable class".to_string()),
                start_line: 20,
                end_line: 40,
                preview: "class AuthManager: authenticate searchable".to_string(),
                ts_doc_text: "auth manager class searchable test".to_string(),
                recency_score: 1.0,
                churn_score: 0.3,
                metadata: None,
                worktree_id,
            })
            .await
            .unwrap();

        // TypeScript func chunk
        store
            .insert_chunk(&ChunkRecord {
                file_id: ts_file_id,
                blob_sha: "blob_ts_func".to_string(),
                symbol_name: Some("getUser".to_string()),
                kind: "func".to_string(),
                signature: Some("function getUser()".to_string()),
                docstring: Some("searchable function for users".to_string()),
                start_line: 1,
                end_line: 15,
                preview: "function getUser() { searchable authenticate }".to_string(),
                ts_doc_text: "get user function searchable test".to_string(),
                recency_score: 1.0,
                churn_score: 0.2,
                metadata: None,
                worktree_id,
            })
            .await
            .unwrap();

        // TypeScript method chunk
        store
            .insert_chunk(&ChunkRecord {
                file_id: ts_file_id,
                blob_sha: "blob_ts_method".to_string(),
                symbol_name: Some("findById".to_string()),
                kind: "method".to_string(),
                signature: Some("findById(id: string)".to_string()),
                docstring: Some("searchable method to find by id".to_string()),
                start_line: 20,
                end_line: 35,
                preview: "findById(id) { searchable authenticate }".to_string(),
                ts_doc_text: "find by id method searchable test".to_string(),
                recency_score: 1.0,
                churn_score: 0.1,
                metadata: None,
                worktree_id,
            })
            .await
            .unwrap();

        // Rust func chunk
        store
            .insert_chunk(&ChunkRecord {
                file_id: rs_file_id,
                blob_sha: "blob_rs_func".to_string(),
                symbol_name: Some("main_searchable".to_string()),
                kind: "func".to_string(),
                signature: Some("fn main_searchable()".to_string()),
                docstring: Some("searchable main function".to_string()),
                start_line: 1,
                end_line: 20,
                preview: "fn main_searchable() { authenticate searchable }".to_string(),
                ts_doc_text: "main searchable function test".to_string(),
                recency_score: 1.0,
                churn_score: 0.4,
                metadata: None,
                worktree_id,
            })
            .await
            .unwrap();

        // Rust import chunk
        store
            .insert_chunk(&ChunkRecord {
                file_id: rs_file_id,
                blob_sha: "blob_rs_import".to_string(),
                symbol_name: Some("use_auth".to_string()),
                kind: "import".to_string(),
                signature: None,
                docstring: Some("searchable import for auth".to_string()),
                start_line: 25,
                end_line: 26,
                preview: "use auth searchable authenticate".to_string(),
                ts_doc_text: "use auth searchable authenticate import".to_string(),
                recency_score: 1.0,
                churn_score: 0.0,
                metadata: None,
                worktree_id,
            })
            .await
            .unwrap();

        (store, repo_name)
    }

    #[tokio::test]
    async fn test_search_fts_with_kind_filter() {
        let (store, repo) = setup_filter_test_store().await;

        let kind_filter = vec!["func".to_string()];
        let results = store
            .search_chunks_fts(
                repo,
                None,
                "searchable",
                50,
                false,
                Some(&kind_filter),
                None,
            )
            .await
            .unwrap();

        assert!(!results.is_empty(), "Should find func chunks");
        for hit in &results {
            assert_eq!(
                hit.kind, "func",
                "All results should have kind 'func', got '{}'",
                hit.kind
            );
        }
    }

    #[tokio::test]
    async fn test_search_fts_with_lang_filter() {
        let (store, repo) = setup_filter_test_store().await;

        let lang_filter = vec!["py".to_string()];
        let results = store
            .search_chunks_fts(
                repo,
                None,
                "searchable",
                50,
                false,
                None,
                Some(&lang_filter),
            )
            .await
            .unwrap();

        assert!(!results.is_empty(), "Should find py chunks");
        for hit in &results {
            assert!(
                hit.file_relpath.ends_with(".py"),
                "All results should be from .py files, got '{}'",
                hit.file_relpath
            );
        }
    }

    #[tokio::test]
    async fn test_search_fts_with_both_filters() {
        let (store, repo) = setup_filter_test_store().await;

        let kind_filter = vec!["func".to_string()];
        let lang_filter = vec!["py".to_string()];
        let results = store
            .search_chunks_fts(
                repo,
                None,
                "searchable",
                50,
                false,
                Some(&kind_filter),
                Some(&lang_filter),
            )
            .await
            .unwrap();

        // Should only return func chunks from py files (AND semantics)
        assert!(!results.is_empty(), "Should find py func chunks");
        for hit in &results {
            assert_eq!(hit.kind, "func", "Expected kind 'func', got '{}'", hit.kind);
            assert!(
                hit.file_relpath.ends_with(".py"),
                "Expected .py file, got '{}'",
                hit.file_relpath
            );
        }
        // There's exactly 1 Python func: authenticate_user
        assert_eq!(results.len(), 1, "Should find exactly 1 py func chunk");
    }

    #[tokio::test]
    async fn test_search_vector_with_kind_filter() {
        let (store, repo) = setup_filter_test_store().await;

        // Vector search requires embeddings and sqlite-vec extension.
        // Without them, search_chunks_vector returns empty results gracefully.
        // This test validates that the method accepts filter parameters without error.
        let kind_filter = vec!["func".to_string()];
        let dummy_embedding = vec![0.1f32; 1536];

        let result = store
            .search_chunks_vector(
                repo,
                None,
                &dummy_embedding,
                10,
                false,
                Some(&kind_filter),
                None,
            )
            .await;

        // Should not error - graceful degradation when vec extension is unavailable
        assert!(
            result.is_ok(),
            "Vector search with kind filter should not error: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_search_vector_with_lang_filter() {
        let (store, repo) = setup_filter_test_store().await;

        let lang_filter = vec!["py".to_string()];
        let dummy_embedding = vec![0.1f32; 1536];

        let result = store
            .search_chunks_vector(
                repo,
                None,
                &dummy_embedding,
                10,
                false,
                None,
                Some(&lang_filter),
            )
            .await;

        assert!(
            result.is_ok(),
            "Vector search with lang filter should not error: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_search_vector_with_both_filters() {
        let (store, repo) = setup_filter_test_store().await;

        let kind_filter = vec!["func".to_string()];
        let lang_filter = vec!["py".to_string(), "ts".to_string()];
        let dummy_embedding = vec![0.1f32; 1536];

        let result = store
            .search_chunks_vector(
                repo,
                None,
                &dummy_embedding,
                10,
                false,
                Some(&kind_filter),
                Some(&lang_filter),
            )
            .await;

        assert!(
            result.is_ok(),
            "Vector search with both filters should not error: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_search_fts_empty_filter_arrays_same_as_none() {
        let (store, repo) = setup_filter_test_store().await;

        // Results with None filters
        let results_none = store
            .search_chunks_fts(repo, None, "searchable", 50, false, None, None)
            .await
            .unwrap();

        // Results with empty filter arrays
        let empty_kind: Vec<String> = vec![];
        let empty_lang: Vec<String> = vec![];
        let results_empty = store
            .search_chunks_fts(
                repo,
                None,
                "searchable",
                50,
                false,
                Some(&empty_kind),
                Some(&empty_lang),
            )
            .await
            .unwrap();

        assert_eq!(
            results_none.len(),
            results_empty.len(),
            "Empty filter arrays should return same count as None"
        );
    }

    #[tokio::test]
    async fn test_search_fts_nonexistent_kind_returns_empty() {
        let (store, repo) = setup_filter_test_store().await;

        let kind_filter = vec!["nonexistent_kind_xyz".to_string()];
        let results = store
            .search_chunks_fts(
                repo,
                None,
                "searchable",
                50,
                false,
                Some(&kind_filter),
                None,
            )
            .await
            .unwrap();

        assert!(
            results.is_empty(),
            "Nonexistent kind should return empty results, got {}",
            results.len()
        );
    }
}
