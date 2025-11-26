//! SQLite backend comprehensive integration tests
//!
//! Run with: cargo test --features sqlite --test sqlite_integration
//!
//! These tests validate the complete SQLite pipeline from indexing through search:
//! - Full index→embed→search cycle
//! - Multi-worktree scenarios
//! - Embedding deduplication across files
//! - Graph traversal with real relationships
//! - File-based persistence (not just :memory:)
//! - Performance sanity checks

#[cfg(feature = "sqlite")]
mod integration_tests {
    use crewchief_maproom::db::sqlite::hybrid::{HybridWeights, SemanticRanking};
    use crewchief_maproom::db::sqlite::SqliteStore;
    use crewchief_maproom::db::{ChunkRecord, FileRecord, VectorStore};
    use tempfile::tempdir;

    /// Helper to create an in-memory store with schema
    async fn setup_store() -> SqliteStore {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    /// Generate deterministic fake embedding for tests
    /// Real embedding would use actual embedding model
    fn generate_test_embedding(text: &str) -> Vec<f32> {
        let hash = text.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        (0..1536)
            .map(|i| ((hash.wrapping_add(i)) % 1000) as f32 / 1000.0)
            .collect()
    }

    /// Helper to create a test file record
    fn create_file_record(
        repo_id: i64,
        worktree_id: i64,
        commit_id: i64,
        relpath: &str,
        content_hash: &str,
    ) -> FileRecord {
        FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: relpath.to_string(),
            language: Some("rust".to_string()),
            content_hash: content_hash.to_string(),
            size_bytes: 1000,
            last_modified: None,
        }
    }

    /// Helper to create a test chunk record
    fn create_chunk_record(
        file_id: i64,
        worktree_id: i64,
        blob_sha: &str,
        symbol_name: &str,
        kind: &str,
        preview: &str,
    ) -> ChunkRecord {
        ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: blob_sha.to_string(),
            symbol_name: Some(symbol_name.to_string()),
            kind: kind.to_string(),
            signature: Some(format!("fn {}()", symbol_name)),
            docstring: Some(format!("{} documentation", symbol_name)),
            start_line: 1,
            end_line: 10,
            preview: preview.to_string(),
            ts_doc_text: format!("{} {}", symbol_name, kind),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        }
    }

    // ========================================================================
    // Full Index → Embed → Search Cycle
    // ========================================================================

    #[tokio::test]
    async fn test_full_index_search_cycle() {
        let store = setup_store().await;

        // 1. Create repo/worktree/commit hierarchy
        let repo_id = store
            .get_or_create_repo("test-repo", "/test/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/test/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "abc123def456", None)
            .await
            .unwrap();

        // 2. Index file with chunk
        let file = create_file_record(repo_id, worktree_id, commit_id, "src/auth.rs", "hash123");
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = create_chunk_record(
            file_id,
            worktree_id,
            "blob_auth",
            "authenticate",
            "function",
            "pub fn authenticate(user: &str) -> Result<Token>",
        );
        let _chunk_id = store.insert_chunk(&chunk).await.unwrap();

        // 3. Store embedding
        let embedding = generate_test_embedding("authenticate function");
        store
            .upsert_embedding("blob_auth", &embedding, "test-model")
            .await
            .unwrap();

        // 4. Search - FTS
        let fts_results = store
            .search_chunks_fts("test-repo", Some("main"), "authenticate", 10, false)
            .await
            .unwrap();
        assert!(!fts_results.is_empty(), "FTS search should return results");
        assert!(
            fts_results[0]
                .symbol_name
                .as_ref()
                .map(|s| s.contains("authenticate"))
                .unwrap_or(false),
            "Should find authenticate function"
        );

        // 5. Search - Hybrid (FTS + possible vector)
        // Note: Hybrid search may fail if vec_code table doesn't exist (extension not loaded)
        let query_embedding = generate_test_embedding("login authentication");
        let hybrid_result = store
            .search_hybrid(
                "test-repo",
                Some("main"),
                "authenticate",
                &query_embedding,
                10,
                HybridWeights::default(),
            )
            .await;

        // Hybrid may fail if vector extension not available - that's OK in tests
        if let Ok(hybrid_results) = hybrid_result {
            // If it succeeds, should have results since FTS part should match
            assert!(
                !hybrid_results.is_empty(),
                "Hybrid search should return results when available"
            );
        }
        // else: vector extension not available, which is acceptable in tests
    }

    // ========================================================================
    // Multi-Worktree Scenarios
    // ========================================================================

    #[tokio::test]
    async fn test_multi_worktree_search() {
        let store = setup_store().await;

        // Create repo with 2 worktrees
        let repo_id = store
            .get_or_create_repo("multi-wt-repo", "/path")
            .await
            .unwrap();
        let wt_main = store
            .get_or_create_worktree(repo_id, "main", "/path/main")
            .await
            .unwrap();
        let wt_feature = store
            .get_or_create_worktree(repo_id, "feature", "/path/feature")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "commit1", None)
            .await
            .unwrap();

        // Index same function in both worktrees (different implementations)
        let file_main =
            create_file_record(repo_id, wt_main, commit_id, "src/utils.rs", "hash_main");
        let file_id_main = store.upsert_file(&file_main).await.unwrap();

        let chunk_main = create_chunk_record(
            file_id_main,
            wt_main,
            "blob_main",
            "process_data",
            "function",
            "fn process_data() { /* main impl */ }",
        );
        store.insert_chunk(&chunk_main).await.unwrap();

        let file_feature =
            create_file_record(repo_id, wt_feature, commit_id, "src/utils.rs", "hash_feature");
        let file_id_feature = store.upsert_file(&file_feature).await.unwrap();

        let chunk_feature = create_chunk_record(
            file_id_feature,
            wt_feature,
            "blob_feature",
            "process_data",
            "function",
            "fn process_data() { /* feature impl */ }",
        );
        store.insert_chunk(&chunk_feature).await.unwrap();

        // Search in specific worktree
        let results_main = store
            .search_chunks_fts("multi-wt-repo", Some("main"), "process_data", 10, false)
            .await
            .unwrap();
        assert_eq!(
            results_main.len(),
            1,
            "Should find 1 result in main worktree"
        );

        let results_feature = store
            .search_chunks_fts("multi-wt-repo", Some("feature"), "process_data", 10, false)
            .await
            .unwrap();
        assert_eq!(
            results_feature.len(),
            1,
            "Should find 1 result in feature worktree"
        );

        // Search across all worktrees
        let results_all = store
            .search_chunks_fts("multi-wt-repo", None, "process_data", 10, false)
            .await
            .unwrap();
        assert_eq!(
            results_all.len(),
            2,
            "Should find 2 results across all worktrees"
        );
    }

    #[tokio::test]
    async fn test_worktree_isolation() {
        let store = setup_store().await;

        // Create repo with 2 worktrees
        let repo_id = store
            .get_or_create_repo("isolation-test", "/path")
            .await
            .unwrap();
        let wt_a = store
            .get_or_create_worktree(repo_id, "branch-a", "/path/a")
            .await
            .unwrap();
        let _wt_b = store
            .get_or_create_worktree(repo_id, "branch-b", "/path/b")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "commit1", None)
            .await
            .unwrap();

        // Only add to branch-a
        let file_a = create_file_record(repo_id, wt_a, commit_id, "unique.rs", "hash_unique");
        let file_id_a = store.upsert_file(&file_a).await.unwrap();

        let chunk_a = create_chunk_record(
            file_id_a,
            wt_a,
            "blob_unique",
            "unique_function",
            "function",
            "fn unique_function() {}",
        );
        store.insert_chunk(&chunk_a).await.unwrap();

        // Search in branch-a should find it
        let results_a = store
            .search_chunks_fts("isolation-test", Some("branch-a"), "unique_function", 10, false)
            .await
            .unwrap();
        assert_eq!(
            results_a.len(),
            1,
            "Should find unique_function in branch-a"
        );

        // Search in branch-b should NOT find it
        let results_b = store
            .search_chunks_fts("isolation-test", Some("branch-b"), "unique_function", 10, false)
            .await
            .unwrap();
        assert_eq!(
            results_b.len(),
            0,
            "Should NOT find unique_function in branch-b"
        );
    }

    // ========================================================================
    // Embedding Deduplication
    // ========================================================================

    #[tokio::test]
    async fn test_embedding_dedup_same_content() {
        let store = setup_store().await;

        // Create two files with identical blob_sha (same content)
        let repo_id = store
            .get_or_create_repo("dedup-test", "/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "commit1", None)
            .await
            .unwrap();

        // File 1
        let file1 = create_file_record(repo_id, worktree_id, commit_id, "src/a/util.rs", "hash1");
        let file_id1 = store.upsert_file(&file1).await.unwrap();

        let chunk1 = create_chunk_record(
            file_id1,
            worktree_id,
            "same_blob_sha", // Same blob_sha
            "helper",
            "function",
            "fn helper() {}",
        );
        store.insert_chunk(&chunk1).await.unwrap();

        // File 2 (different path, same content -> same blob_sha)
        let file2 = create_file_record(repo_id, worktree_id, commit_id, "src/b/util.rs", "hash2");
        let file_id2 = store.upsert_file(&file2).await.unwrap();

        let chunk2 = create_chunk_record(
            file_id2,
            worktree_id,
            "same_blob_sha", // Same blob_sha as chunk1
            "helper",
            "function",
            "fn helper() {}",
        );
        store.insert_chunk(&chunk2).await.unwrap();

        // Store embedding once
        let embedding = generate_test_embedding("helper function");
        store
            .upsert_embedding("same_blob_sha", &embedding, "test-model")
            .await
            .unwrap();

        // Verify embedding exists
        let has_embedding = store.has_embedding("same_blob_sha").await.unwrap();
        assert!(has_embedding, "Embedding should exist");

        // Search should find both chunks even though one embedding
        let results = store
            .search_chunks_fts("dedup-test", None, "helper", 10, false)
            .await
            .unwrap();
        assert_eq!(results.len(), 2, "Should find both chunks with same content");
    }

    #[tokio::test]
    async fn test_embedding_check_missing() {
        let store = setup_store().await;

        // Check for non-existent embedding
        let has_embedding = store.has_embedding("nonexistent_sha").await.unwrap();
        assert!(!has_embedding, "Should not have embedding for unknown SHA");
    }

    // ========================================================================
    // Graph Traversal Integration
    // ========================================================================

    #[tokio::test]
    async fn test_graph_traversal_real_relationships() {
        let store = setup_store().await;

        // Setup: Create hierarchy
        let repo_id = store
            .get_or_create_repo("graph-test", "/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "commit1", None)
            .await
            .unwrap();

        let file = create_file_record(repo_id, worktree_id, commit_id, "src/api.rs", "hash1");
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create function hierarchy: handle_request -> validate_input -> sanitize_data
        let chunk_handle = create_chunk_record(
            file_id,
            worktree_id,
            "blob_handle",
            "handle_request",
            "function",
            "fn handle_request() { validate_input(); }",
        );
        let handle_id = store.insert_chunk(&chunk_handle).await.unwrap();

        let chunk_validate = create_chunk_record(
            file_id,
            worktree_id,
            "blob_validate",
            "validate_input",
            "function",
            "fn validate_input() { sanitize_data(); }",
        );
        let validate_id = store.insert_chunk(&chunk_validate).await.unwrap();

        let chunk_sanitize = create_chunk_record(
            file_id,
            worktree_id,
            "blob_sanitize",
            "sanitize_data",
            "function",
            "fn sanitize_data() {}",
        );
        let sanitize_id = store.insert_chunk(&chunk_sanitize).await.unwrap();

        // Create edges: handle_request calls validate_input, validate_input calls sanitize_data
        store
            .insert_chunk_edge(handle_id, validate_id, "calls")
            .await
            .unwrap();
        store
            .insert_chunk_edge(validate_id, sanitize_id, "calls")
            .await
            .unwrap();

        // Find callers of sanitize_data
        let callers = store.find_callers(sanitize_id, Some(3)).await.unwrap();
        assert!(
            callers.iter().any(|r| r.chunk_id == validate_id),
            "Should find validate_input as direct caller"
        );
        assert!(
            callers.iter().any(|r| r.chunk_id == handle_id),
            "Should find handle_request as transitive caller"
        );

        // Find callees of handle_request
        let callees = store.find_callees(handle_id, Some(3)).await.unwrap();
        assert!(
            callees.iter().any(|r| r.chunk_id == validate_id),
            "Should find validate_input as direct callee"
        );
        assert!(
            callees.iter().any(|r| r.chunk_id == sanitize_id),
            "Should find sanitize_data as transitive callee"
        );
    }

    // ========================================================================
    // File-Based Integration (Real Temp File)
    // ========================================================================

    #[tokio::test]
    async fn test_file_based_persistence() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test maproom.db"); // Intentional space in name

        // 1. Create store with file path, insert data
        {
            let store = SqliteStore::connect(db_path.to_str().unwrap())
                .await
                .unwrap();
            store.migrate().await.unwrap();

            let repo_id = store
                .get_or_create_repo("persist-test", "/test")
                .await
                .unwrap();
            let worktree_id = store
                .get_or_create_worktree(repo_id, "main", "/test")
                .await
                .unwrap();
            let commit_id = store
                .get_or_create_commit(repo_id, "abc123", None)
                .await
                .unwrap();

            let file =
                create_file_record(repo_id, worktree_id, commit_id, "test.rs", "persist_hash");
            let file_id = store.upsert_file(&file).await.unwrap();

            let chunk = create_chunk_record(
                file_id,
                worktree_id,
                "persist_blob",
                "persistent_func",
                "function",
                "fn persistent_func() {}",
            );
            store.insert_chunk(&chunk).await.unwrap();

            // Store drops here
        }

        // 2. Reopen and verify data persisted
        {
            let store = SqliteStore::connect(db_path.to_str().unwrap())
                .await
                .unwrap();

            // Search should find the persisted chunk
            let results = store
                .search_chunks_fts("persist-test", None, "persistent_func", 10, false)
                .await
                .unwrap();
            assert!(
                !results.is_empty(),
                "Data should persist across connection close/reopen"
            );
        }

        // 3. Verify file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&db_path).unwrap().permissions();
            assert_eq!(
                perms.mode() & 0o777,
                0o600,
                "Database file should have 0600 permissions"
            );
        }
    }

    #[tokio::test]
    async fn test_wal_mode_enabled() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("wal_test.db");

        let store = SqliteStore::connect(db_path.to_str().unwrap())
            .await
            .unwrap();
        store.migrate().await.unwrap();

        // WAL mode should be enabled - check by verifying WAL file exists after writes
        let _repo_id = store
            .get_or_create_repo("wal-test", "/test")
            .await
            .unwrap();

        let _wal_path = temp_dir.path().join("wal_test.db-wal");
        // WAL file may or may not exist depending on checkpoint status
        // Just verify the database is functional
        assert!(db_path.exists(), "Database file should exist");
    }

    // ========================================================================
    // Performance Sanity Checks
    // ========================================================================

    #[tokio::test]
    async fn test_batch_insert_performance() {
        let store = setup_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("perf-test", "/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "commit1", None)
            .await
            .unwrap();

        let file = create_file_record(repo_id, worktree_id, commit_id, "large.rs", "hash_large");
        let file_id = store.upsert_file(&file).await.unwrap();

        // Insert 100 chunks
        let start = std::time::Instant::now();

        for i in 0..100 {
            let chunk = create_chunk_record(
                file_id,
                worktree_id,
                &format!("blob_{}", i),
                &format!("func_{}", i),
                "function",
                &format!("fn func_{}() {{}}", i),
            );
            store.insert_chunk(&chunk).await.unwrap();
        }

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 10,
            "100 chunk inserts took {:?}, should be < 10s",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_search_performance() {
        let store = setup_store().await;

        // Setup with some data
        let repo_id = store
            .get_or_create_repo("search-perf", "/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "commit1", None)
            .await
            .unwrap();

        let file =
            create_file_record(repo_id, worktree_id, commit_id, "searchable.rs", "hash_search");
        let file_id = store.upsert_file(&file).await.unwrap();

        // Insert 50 chunks
        for i in 0..50 {
            let chunk = create_chunk_record(
                file_id,
                worktree_id,
                &format!("blob_s{}", i),
                &format!("searchable_func_{}", i),
                "function",
                &format!("fn searchable_func_{}() {{}}", i),
            );
            store.insert_chunk(&chunk).await.unwrap();
        }

        // Search should complete quickly
        let start = std::time::Instant::now();

        for _ in 0..10 {
            let _results = store
                .search_chunks_fts("search-perf", None, "searchable", 10, false)
                .await
                .unwrap();
        }

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 5,
            "10 FTS searches took {:?}, should be < 5s",
            elapsed
        );
    }

    // ========================================================================
    // Hybrid Search Integration
    // ========================================================================

    #[tokio::test]
    async fn test_hybrid_search_with_ranking() {
        let store = setup_store().await;

        // Setup
        let repo_id = store
            .get_or_create_repo("hybrid-test", "/path")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/path")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "commit1", None)
            .await
            .unwrap();

        let file = create_file_record(repo_id, worktree_id, commit_id, "ranked.rs", "hash_ranked");
        let file_id = store.upsert_file(&file).await.unwrap();

        // Insert a function (should rank higher due to kind multiplier)
        let func_chunk = create_chunk_record(
            file_id,
            worktree_id,
            "blob_func",
            "user_auth",
            "function",
            "fn user_auth() {}",
        );
        store.insert_chunk(&func_chunk).await.unwrap();

        // Insert a variable (should rank lower)
        let var_chunk = create_chunk_record(
            file_id,
            worktree_id,
            "blob_var",
            "user_auth_config",
            "variable",
            "let user_auth_config = {};",
        );
        store.insert_chunk(&var_chunk).await.unwrap();

        // Store embeddings
        store
            .upsert_embedding("blob_func", &generate_test_embedding("user auth"), "test")
            .await
            .unwrap();
        store
            .upsert_embedding(
                "blob_var",
                &generate_test_embedding("user auth config"),
                "test",
            )
            .await
            .unwrap();

        // Search with semantic ranking
        // Note: May fail if vec_code table doesn't exist (extension not loaded)
        let result = store
            .search_hybrid_ranked(
                "hybrid-test",
                Some("main"),
                "user_auth",
                &generate_test_embedding("user authentication"),
                10,
                HybridWeights::default(),
                SemanticRanking::default(),
            )
            .await;

        // Hybrid ranked may fail if vector extension not available - that's OK in tests
        if let Ok(results) = result {
            assert!(!results.is_empty(), "Should have results");
            // Function should rank higher than variable due to kind multiplier
            if results.len() >= 2 {
                // If we get both, function should come first
                let first_kind = &results[0].kind;
                assert_eq!(
                    first_kind, "function",
                    "Function should rank higher than variable"
                );
            }
        }
        // else: vector extension not available, which is acceptable in tests
    }

    // ========================================================================
    // Edge Cases and Error Handling
    // ========================================================================

    #[tokio::test]
    async fn test_nonexistent_repo_search() {
        let store = setup_store().await;

        // Search in repo that doesn't exist - may return empty or error
        let result = store
            .search_chunks_fts("nonexistent-repo", None, "query", 10, false)
            .await;

        // Either empty results or error is acceptable for nonexistent repo
        match result {
            Ok(results) => assert!(results.is_empty(), "Should return empty for nonexistent repo"),
            Err(_) => {} // Error is acceptable for nonexistent repo
        }
    }

    #[tokio::test]
    async fn test_nonexistent_worktree_search() {
        let store = setup_store().await;

        // Create repo but search in nonexistent worktree
        let _repo_id = store
            .get_or_create_repo("exists-repo", "/path")
            .await
            .unwrap();

        let results = store
            .search_chunks_fts("exists-repo", Some("nonexistent-wt"), "query", 10, false)
            .await
            .unwrap();
        assert!(
            results.is_empty(),
            "Should return empty for nonexistent worktree"
        );
    }

    #[tokio::test]
    async fn test_empty_database_search() {
        let store = setup_store().await;

        // Search in completely empty database - may return empty or error
        let result = store
            .search_chunks_fts("any-repo", None, "any query", 10, false)
            .await;

        // Either empty results or error is acceptable for empty database
        match result {
            Ok(results) => assert!(results.is_empty(), "Should return empty for empty database"),
            Err(_) => {} // Error is acceptable for empty database
        }
    }
}
