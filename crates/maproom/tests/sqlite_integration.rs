//! SQLite backend comprehensive integration tests
//!
//! Run with: cargo test --test sqlite_integration
//!
//! These tests validate the complete SQLite pipeline from indexing through search:
//! - Full index->search cycle with filter parameters
//! - End-to-end filter behavior (kind, lang, combined)
//! - Vector and hybrid search filter acceptance
//! - Edge cases: no matches, multiple values, backward compatibility
//!
//! Legacy integration tests (pre-filter) are preserved but disabled below.

// ============================================================================
// Filter Integration Tests (MRIMP-1.1005)
// ============================================================================

mod filter_integration_tests {
    use crewchief_maproom::db::sqlite::SqliteStore;
    use crewchief_maproom::db::StoreChunks;
    use crewchief_maproom::db::StoreCore;
    use crewchief_maproom::db::StoreEmbeddings;
    use crewchief_maproom::db::StoreMigration;
    use crewchief_maproom::db::StoreSearch;
    use crewchief_maproom::db::{ChunkRecord, FileRecord};
    use std::collections::HashSet;

    /// Helper to create an in-memory store with schema
    async fn setup_store() -> SqliteStore {
        let store = SqliteStore::connect(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    /// Helper to create a file record with a specific language.
    fn make_file(
        repo_id: i64,
        worktree_id: i64,
        commit_id: i64,
        relpath: &str,
        language: &str,
        content_hash: &str,
    ) -> FileRecord {
        FileRecord {
            repo_id,
            worktree_id,
            commit_id,
            relpath: relpath.to_string(),
            language: Some(language.to_string()),
            content_hash: content_hash.to_string(),
            size_bytes: 500,
            last_modified: None,
        }
    }

    /// Helper to create a chunk record with a specific kind.
    /// The ts_doc_text and preview fields include "filterable" to enable FTS matching.
    fn make_chunk(
        file_id: i64,
        worktree_id: i64,
        blob_sha: &str,
        symbol_name: &str,
        kind: &str,
        start_line: i32,
        end_line: i32,
    ) -> ChunkRecord {
        ChunkRecord {
            file_id,
            worktree_id,
            blob_sha: blob_sha.to_string(),
            symbol_name: Some(symbol_name.to_string()),
            kind: kind.to_string(),
            signature: Some(format!("{} {}", kind, symbol_name)),
            docstring: Some(format!("filterable {} {}", kind, symbol_name)),
            start_line,
            end_line,
            preview: format!("{} {} filterable code", kind, symbol_name),
            ts_doc_text: format!("{} {} filterable searchable", symbol_name, kind),
            recency_score: 1.0,
            churn_score: 0.5,
            metadata: None,
        }
    }

    /// Set up a store populated with diverse test data for filter integration testing.
    ///
    /// Creates:
    /// - 3 Python files (.py) with 6 chunks (3 func, 2 class, 1 import)
    /// - 3 TypeScript files (.ts) with 6 chunks (3 func, 2 class, 1 variable)
    /// - 2 Rust files (.rs) with 4 chunks (2 func, 2 class)
    /// - 1 Markdown file (.md) with 2 chunks (2 heading_2)
    ///
    /// Total: 18 chunks across 4 languages and 5 kinds.
    async fn setup_diverse_store() -> (SqliteStore, &'static str) {
        let store = setup_store().await;
        let repo_name = "filter-integ-repo";

        let repo_id = store
            .get_or_create_repo(repo_name, "/tmp/filter-integ")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/tmp/filter-integ")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(repo_id, "integ_abc123", None)
            .await
            .unwrap();

        // ---- Python files ----
        let py1 = make_file(
            repo_id,
            worktree_id,
            commit_id,
            "src/auth.py",
            "py",
            "py_h1",
        );
        let py1_id = store.upsert_file(&py1).await.unwrap();

        let py2 = make_file(
            repo_id,
            worktree_id,
            commit_id,
            "src/models.py",
            "py",
            "py_h2",
        );
        let py2_id = store.upsert_file(&py2).await.unwrap();

        let py3 = make_file(
            repo_id,
            worktree_id,
            commit_id,
            "src/utils.py",
            "py",
            "py_h3",
        );
        let py3_id = store.upsert_file(&py3).await.unwrap();

        // Python func chunks (3)
        store
            .insert_chunk(&make_chunk(
                py1_id,
                worktree_id,
                "blob_py_f1",
                "authenticate",
                "func",
                1,
                10,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                py2_id,
                worktree_id,
                "blob_py_f2",
                "validate_user",
                "func",
                1,
                15,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                py3_id,
                worktree_id,
                "blob_py_f3",
                "parse_input",
                "func",
                1,
                12,
            ))
            .await
            .unwrap();

        // Python class chunks (2)
        store
            .insert_chunk(&make_chunk(
                py1_id,
                worktree_id,
                "blob_py_c1",
                "AuthManager",
                "class",
                20,
                40,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                py2_id,
                worktree_id,
                "blob_py_c2",
                "UserModel",
                "class",
                20,
                50,
            ))
            .await
            .unwrap();

        // Python import chunk (1)
        store
            .insert_chunk(&make_chunk(
                py3_id,
                worktree_id,
                "blob_py_i1",
                "import_os",
                "import",
                1,
                2,
            ))
            .await
            .unwrap();

        // ---- TypeScript files ----
        let ts1 = make_file(
            repo_id,
            worktree_id,
            commit_id,
            "src/auth.ts",
            "ts",
            "ts_h1",
        );
        let ts1_id = store.upsert_file(&ts1).await.unwrap();

        let ts2 = make_file(
            repo_id,
            worktree_id,
            commit_id,
            "src/user.ts",
            "ts",
            "ts_h2",
        );
        let ts2_id = store.upsert_file(&ts2).await.unwrap();

        let ts3 = make_file(
            repo_id,
            worktree_id,
            commit_id,
            "src/config.ts",
            "ts",
            "ts_h3",
        );
        let ts3_id = store.upsert_file(&ts3).await.unwrap();

        // TypeScript func chunks (3)
        store
            .insert_chunk(&make_chunk(
                ts1_id,
                worktree_id,
                "blob_ts_f1",
                "getToken",
                "func",
                1,
                10,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                ts2_id,
                worktree_id,
                "blob_ts_f2",
                "fetchUser",
                "func",
                1,
                15,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                ts3_id,
                worktree_id,
                "blob_ts_f3",
                "loadConfig",
                "func",
                1,
                12,
            ))
            .await
            .unwrap();

        // TypeScript class chunks (2)
        store
            .insert_chunk(&make_chunk(
                ts1_id,
                worktree_id,
                "blob_ts_c1",
                "AuthService",
                "class",
                20,
                40,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                ts2_id,
                worktree_id,
                "blob_ts_c2",
                "UserRepo",
                "class",
                20,
                50,
            ))
            .await
            .unwrap();

        // TypeScript variable chunk (1)
        store
            .insert_chunk(&make_chunk(
                ts3_id,
                worktree_id,
                "blob_ts_v1",
                "DEFAULT_CONFIG",
                "variable",
                1,
                3,
            ))
            .await
            .unwrap();

        // ---- Rust files ----
        let rs1 = make_file(
            repo_id,
            worktree_id,
            commit_id,
            "src/main.rs",
            "rs",
            "rs_h1",
        );
        let rs1_id = store.upsert_file(&rs1).await.unwrap();

        let rs2 = make_file(repo_id, worktree_id, commit_id, "src/lib.rs", "rs", "rs_h2");
        let rs2_id = store.upsert_file(&rs2).await.unwrap();

        // Rust func chunks (2)
        store
            .insert_chunk(&make_chunk(
                rs1_id,
                worktree_id,
                "blob_rs_f1",
                "main_entry",
                "func",
                1,
                20,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                rs2_id,
                worktree_id,
                "blob_rs_f2",
                "process_data",
                "func",
                1,
                25,
            ))
            .await
            .unwrap();

        // Rust class chunks (2)
        store
            .insert_chunk(&make_chunk(
                rs1_id,
                worktree_id,
                "blob_rs_c1",
                "AppState",
                "class",
                30,
                50,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                rs2_id,
                worktree_id,
                "blob_rs_c2",
                "DataStore",
                "class",
                30,
                60,
            ))
            .await
            .unwrap();

        // ---- Markdown file ----
        let md1 = make_file(
            repo_id,
            worktree_id,
            commit_id,
            "docs/README.md",
            "md",
            "md_h1",
        );
        let md1_id = store.upsert_file(&md1).await.unwrap();

        // Markdown heading_2 chunks (2)
        store
            .insert_chunk(&make_chunk(
                md1_id,
                worktree_id,
                "blob_md_h1",
                "Installation",
                "heading_2",
                5,
                15,
            ))
            .await
            .unwrap();
        store
            .insert_chunk(&make_chunk(
                md1_id,
                worktree_id,
                "blob_md_h2",
                "Usage",
                "heading_2",
                20,
                30,
            ))
            .await
            .unwrap();

        (store, repo_name)
    }

    // ========================================================================
    // Test 1: FTS search with kind filter
    // ========================================================================

    #[tokio::test]
    async fn test_fts_search_with_kind_filter() {
        let (store, repo) = setup_diverse_store().await;

        // Search with kind=["func"] filter
        let kind_filter = vec!["func".to_string()];
        let (filtered, _total_count) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                Some(&kind_filter),
                None,
            )
            .await
            .unwrap();

        // All results must have kind == "func"
        assert!(!filtered.is_empty(), "Should find at least one func chunk");
        for hit in &filtered {
            assert_eq!(
                hit.kind, "func",
                "All filtered results should have kind 'func', got '{}'",
                hit.kind
            );
        }

        // Expected: 3 py funcs + 3 ts funcs + 2 rs funcs = 8 func chunks total
        assert_eq!(
            filtered.len(),
            8,
            "Should find all 8 func chunks across all languages"
        );

        // Count should be less than unfiltered total
        let (unfiltered, _) = store
            .search_chunks_fts(repo, None, "filterable", 50, false, None, None)
            .await
            .unwrap();
        assert!(
            filtered.len() < unfiltered.len(),
            "Filtered count ({}) should be less than unfiltered count ({})",
            filtered.len(),
            unfiltered.len()
        );
    }

    // ========================================================================
    // Test 2: FTS search with lang filter
    // ========================================================================

    #[tokio::test]
    async fn test_fts_search_with_lang_filter() {
        let (store, repo) = setup_diverse_store().await;

        // Search with lang=["py"] filter
        let lang_filter = vec!["py".to_string()];
        let (filtered, _total_count) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                None,
                Some(&lang_filter),
            )
            .await
            .unwrap();

        // All results must be from Python files
        assert!(
            !filtered.is_empty(),
            "Should find at least one Python chunk"
        );
        for hit in &filtered {
            assert!(
                hit.file_relpath.ends_with(".py"),
                "All results should be from .py files, got '{}'",
                hit.file_relpath
            );
        }

        // Expected: 3 func + 2 class + 1 import = 6 Python chunks
        assert_eq!(filtered.len(), 6, "Should find all 6 Python chunks");

        // Count should be less than unfiltered total
        let (unfiltered, _) = store
            .search_chunks_fts(repo, None, "filterable", 50, false, None, None)
            .await
            .unwrap();
        assert!(
            filtered.len() < unfiltered.len(),
            "Filtered count ({}) should be less than unfiltered count ({})",
            filtered.len(),
            unfiltered.len()
        );
    }

    // ========================================================================
    // Test 3: FTS search with both filters (AND semantics)
    // ========================================================================

    #[tokio::test]
    async fn test_fts_search_with_both_filters() {
        let (store, repo) = setup_diverse_store().await;

        // Search with kind=["func"] AND lang=["py"]
        let kind_filter = vec!["func".to_string()];
        let lang_filter = vec!["py".to_string()];
        let (filtered, _total_count) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                Some(&kind_filter),
                Some(&lang_filter),
            )
            .await
            .unwrap();

        // All results must be Python functions (AND semantics)
        assert!(
            !filtered.is_empty(),
            "Should find at least one Python func chunk"
        );
        for hit in &filtered {
            assert_eq!(hit.kind, "func", "Expected kind 'func', got '{}'", hit.kind);
            assert!(
                hit.file_relpath.ends_with(".py"),
                "Expected .py file, got '{}'",
                hit.file_relpath
            );
        }

        // Expected: exactly 3 Python func chunks
        assert_eq!(
            filtered.len(),
            3,
            "Should find exactly 3 Python func chunks"
        );

        // Verify AND semantics: results exclude TypeScript funcs and Python classes
        let kind_only = vec!["func".to_string()];
        let (kind_only_results, _) = store
            .search_chunks_fts(repo, None, "filterable", 50, false, Some(&kind_only), None)
            .await
            .unwrap();
        assert!(
            filtered.len() < kind_only_results.len(),
            "AND filter ({}) should return fewer results than kind-only filter ({})",
            filtered.len(),
            kind_only_results.len()
        );

        let lang_only = vec!["py".to_string()];
        let (lang_only_results, _) = store
            .search_chunks_fts(repo, None, "filterable", 50, false, None, Some(&lang_only))
            .await
            .unwrap();
        assert!(
            filtered.len() < lang_only_results.len(),
            "AND filter ({}) should return fewer results than lang-only filter ({})",
            filtered.len(),
            lang_only_results.len()
        );
    }

    // ========================================================================
    // Test 4: Vector search with kind filter
    // ========================================================================

    #[tokio::test]
    async fn test_vector_search_with_kind_filter() {
        let (store, repo) = setup_diverse_store().await;

        // Vector search requires sqlite-vec extension and embeddings.
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

        // Must not error -- graceful degradation when vec extension is unavailable
        assert!(
            result.is_ok(),
            "Vector search with kind filter should not error: {:?}",
            result.err()
        );

        // If results are returned (vec extension available), verify kind filtering
        if let Ok(ref hits) = result {
            for hit in hits {
                assert_eq!(
                    hit.kind, "func",
                    "Vector search results should respect kind filter, got '{}'",
                    hit.kind
                );
            }
        }
    }

    // ========================================================================
    // Test 5: Hybrid search with both filters
    // ========================================================================

    #[tokio::test]
    async fn test_hybrid_search_with_both_filters() {
        let (store, repo) = setup_diverse_store().await;

        // Hybrid search combines FTS + vector. Without vec extension,
        // it degrades to FTS-only. Either way, filters should be accepted.
        let kind_filter = vec!["func".to_string()];
        let lang_filter = vec!["py".to_string()];
        let dummy_embedding = vec![0.1f32; 1536];

        let result = store
            .search_chunks_hybrid(
                repo,
                None,
                "filterable",
                &dummy_embedding,
                50,
                false,
                Some(&kind_filter),
                Some(&lang_filter),
            )
            .await;

        // Must not error
        assert!(
            result.is_ok(),
            "Hybrid search with both filters should not error: {:?}",
            result.err()
        );

        // Verify filter semantics on any results returned
        if let Ok(ref hits) = result {
            for hit in hits {
                assert_eq!(
                    hit.kind, "func",
                    "Hybrid results should respect kind filter, got '{}'",
                    hit.kind
                );
                assert!(
                    hit.file_relpath.ends_with(".py"),
                    "Hybrid results should respect lang filter, got '{}'",
                    hit.file_relpath
                );
            }
            // With FTS component, we should get results from the FTS side
            if !hits.is_empty() {
                // Expected: 3 Python func chunks
                assert_eq!(
                    hits.len(),
                    3,
                    "Hybrid search should find 3 Python func chunks (from FTS component)"
                );
            }
        }
    }

    // ========================================================================
    // Test 6: Filter with no matches returns empty results
    // ========================================================================

    #[tokio::test]
    async fn test_filter_with_no_matches() {
        let (store, repo) = setup_diverse_store().await;

        // Search with a kind that does not exist in our test data
        let kind_filter = vec!["nonexistent_kind_xyz".to_string()];
        let (results, _total_count) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                Some(&kind_filter),
                None,
            )
            .await
            .unwrap();

        // Must return empty vec, not an error
        assert!(
            results.is_empty(),
            "Nonexistent kind filter should return empty results, got {} results",
            results.len()
        );

        // Also test with nonexistent lang
        let lang_filter = vec!["nonexistent_lang_xyz".to_string()];
        let (results, _total_count) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                None,
                Some(&lang_filter),
            )
            .await
            .unwrap();

        assert!(
            results.is_empty(),
            "Nonexistent lang filter should return empty results, got {} results",
            results.len()
        );

        // Test with both nonexistent filters
        let kind_filter = vec!["nonexistent_kind_xyz".to_string()];
        let lang_filter = vec!["nonexistent_lang_xyz".to_string()];
        let (results, _total_count) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                Some(&kind_filter),
                Some(&lang_filter),
            )
            .await
            .unwrap();

        assert!(
            results.is_empty(),
            "Both nonexistent filters should return empty results, got {} results",
            results.len()
        );
    }

    // ========================================================================
    // Test 7: Multiple kind values (OR semantics within filter)
    // ========================================================================

    #[tokio::test]
    async fn test_multiple_kind_values() {
        let (store, repo) = setup_diverse_store().await;

        // Search with kind=["func", "class"] -- should return both kinds (OR)
        let kind_filter = vec!["func".to_string(), "class".to_string()];
        let (filtered, _total_count) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                Some(&kind_filter),
                None,
            )
            .await
            .unwrap();

        // All results must have kind == "func" or kind == "class"
        assert!(!filtered.is_empty(), "Should find func and class chunks");

        let allowed_kinds: HashSet<&str> = ["func", "class"].iter().copied().collect();
        let mut found_kinds: HashSet<String> = HashSet::new();

        for hit in &filtered {
            assert!(
                allowed_kinds.contains(hit.kind.as_str()),
                "Result kind '{}' should be 'func' or 'class'",
                hit.kind
            );
            found_kinds.insert(hit.kind.clone());
        }

        // Verify OR semantics: both kinds must appear in results
        assert!(
            found_kinds.contains("func"),
            "Results should include 'func' chunks"
        );
        assert!(
            found_kinds.contains("class"),
            "Results should include 'class' chunks"
        );

        // Expected: 8 func + 6 class = 14 chunks
        assert_eq!(
            filtered.len(),
            14,
            "Should find 8 func + 6 class = 14 chunks"
        );

        // Verify no other kinds are included (no import, variable, heading_2)
        for hit in &filtered {
            assert_ne!(hit.kind, "import", "Should not include 'import' chunks");
            assert_ne!(hit.kind, "variable", "Should not include 'variable' chunks");
            assert_ne!(
                hit.kind, "heading_2",
                "Should not include 'heading_2' chunks"
            );
        }

        // Also test OR with lang: lang=["py", "ts"]
        let lang_filter = vec!["py".to_string(), "ts".to_string()];
        let (lang_filtered, _) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                None,
                Some(&lang_filter),
            )
            .await
            .unwrap();

        let mut found_langs: HashSet<String> = HashSet::new();
        for hit in &lang_filtered {
            let ext = hit.file_relpath.rsplit('.').next().unwrap_or("");
            found_langs.insert(ext.to_string());
            assert!(
                ext == "py" || ext == "ts",
                "Result file '{}' should be .py or .ts",
                hit.file_relpath
            );
        }
        assert!(
            found_langs.contains("py"),
            "Results should include .py files"
        );
        assert!(
            found_langs.contains("ts"),
            "Results should include .ts files"
        );

        // Expected: 6 py + 6 ts = 12
        assert_eq!(
            lang_filtered.len(),
            12,
            "Should find 6 py + 6 ts = 12 chunks"
        );
    }

    // ========================================================================
    // Test 8: Backward compatibility -- no filters produces same results
    // ========================================================================

    #[tokio::test]
    async fn test_backward_compatibility_no_filters() {
        let (store, repo) = setup_diverse_store().await;

        // Run the same query with explicit None filters
        let (results_with_none, _) = store
            .search_chunks_fts(repo, None, "filterable", 50, false, None, None)
            .await
            .unwrap();

        // Run with empty filter arrays (should behave identically)
        let empty_kind: Vec<String> = vec![];
        let empty_lang: Vec<String> = vec![];
        let (results_with_empty, _) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                Some(&empty_kind),
                Some(&empty_lang),
            )
            .await
            .unwrap();

        // Same count
        assert_eq!(
            results_with_none.len(),
            results_with_empty.len(),
            "None filters and empty filter arrays should return same count ({} vs {})",
            results_with_none.len(),
            results_with_empty.len()
        );

        // Same chunk IDs (order may differ due to scoring, so use sets)
        let ids_none: HashSet<i64> = results_with_none.iter().map(|h| h.chunk_id).collect();
        let ids_empty: HashSet<i64> = results_with_empty.iter().map(|h| h.chunk_id).collect();
        assert_eq!(
            ids_none, ids_empty,
            "None filters and empty filter arrays should return same chunk IDs"
        );

        // Verify we actually got all 18 chunks
        assert_eq!(
            results_with_none.len(),
            18,
            "Unfiltered search should return all 18 chunks"
        );

        // Now verify that filtering reduces the count
        let kind_filter = vec!["func".to_string()];
        let (filtered, _) = store
            .search_chunks_fts(
                repo,
                None,
                "filterable",
                50,
                false,
                Some(&kind_filter),
                None,
            )
            .await
            .unwrap();
        assert!(
            filtered.len() < results_with_none.len(),
            "Filtered results ({}) should be fewer than unfiltered ({})",
            filtered.len(),
            results_with_none.len()
        );
    }

    // ==================== Preview Integration Tests (MRIMP-3.2001) ====================

    #[tokio::test]
    async fn test_fts_search_returns_preview() {
        let store = setup_store().await;
        let repo = "preview-test-repo";

        let repo_id = store
            .get_or_create_repo(repo, "/tmp/preview-test")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/tmp/preview-test")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(worktree_id, "abc123", None)
            .await
            .unwrap();

        // Create a test file
        let file = make_file(repo_id, worktree_id, commit_id, "test.py", "py", "hash1");
        let file_id = store.upsert_file(&file).await.unwrap();

        // Create a chunk with known preview content
        let chunk = make_chunk(
            file_id,
            worktree_id,
            "blob1",
            "test_function",
            "func",
            10,
            20,
        );
        store.insert_chunk(&chunk).await.unwrap();

        // Search - preview is always populated by database query
        let (hits, _total_count) = store
            .search_chunks_fts(repo, None, "filterable", 10, false, None, None)
            .await
            .unwrap();

        assert!(!hits.is_empty(), "Should find at least one result");
        for hit in &hits {
            assert!(
                hit.preview.is_some(),
                "Preview should be Some (populated by database)"
            );
            let preview = hit.preview.as_ref().unwrap();
            assert!(!preview.is_empty(), "Preview should not be empty");
            // Verify it's actually populated with chunk preview content
            assert!(
                preview.contains("filterable"),
                "Preview should contain search content"
            );
        }
    }

    #[tokio::test]
    async fn test_fts_search_without_preview_field_absence() {
        let store = setup_store().await;
        let repo = "preview-absence-test";

        let repo_id = store
            .get_or_create_repo(repo, "/tmp/preview-absence-test")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/tmp/preview-absence-test")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(worktree_id, "abc123", None)
            .await
            .unwrap();

        let file = make_file(repo_id, worktree_id, commit_id, "test.py", "py", "hash1");
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = make_chunk(
            file_id,
            worktree_id,
            "blob1",
            "test_function",
            "func",
            10,
            20,
        );
        store.insert_chunk(&chunk).await.unwrap();

        // Search - get results with preview populated
        let (hits, _total_count) = store
            .search_chunks_fts(repo, None, "filterable", 10, false, None, None)
            .await
            .unwrap();

        assert!(!hits.is_empty(), "Should find at least one result");

        // Simulate what CLI does: set preview to None
        for mut hit in hits {
            hit.preview = None;

            // Serialize and verify "preview" key is absent (not null)
            let json = serde_json::to_string(&hit).unwrap();
            assert!(
                !json.contains("\"preview\""),
                "JSON should not contain preview key when None (skip_serializing_if behavior)"
            );
        }
    }

    #[tokio::test]
    async fn test_vector_search_returns_preview() {
        let store = setup_store().await;
        let repo = "vector-preview-test";

        let repo_id = store
            .get_or_create_repo(repo, "/tmp/vector-preview-test")
            .await
            .unwrap();
        let worktree_id = store
            .get_or_create_worktree(repo_id, "main", "/tmp/vector-preview-test")
            .await
            .unwrap();
        let commit_id = store
            .get_or_create_commit(worktree_id, "abc123", None)
            .await
            .unwrap();

        let file = make_file(repo_id, worktree_id, commit_id, "test.py", "py", "hash1");
        let file_id = store.upsert_file(&file).await.unwrap();

        let chunk = make_chunk(
            file_id,
            worktree_id,
            "blob1",
            "test_function",
            "func",
            10,
            20,
        );
        let _chunk_id = store.insert_chunk(&chunk).await.unwrap();

        // Insert a test embedding for vector search
        let test_embedding = vec![0.1; 1024]; // 1024-dim vector
        store
            .upsert_embedding("blob1", &test_embedding, "test-model")
            .await
            .unwrap();

        // Perform vector search - preview is populated by database
        let query_embedding = vec![0.1; 1024];
        let hits = store
            .search_chunks_vector(repo, None, &query_embedding, 10, false, None, None)
            .await
            .unwrap();

        assert!(!hits.is_empty(), "Should find at least one result");
        for hit in &hits {
            assert!(
                hit.preview.is_some(),
                "Preview should be Some (populated by database) in vector search"
            );
            let preview = hit.preview.as_ref().unwrap();
            assert!(!preview.is_empty(), "Preview should not be empty");
        }
    }
}

// ============================================================================
// Legacy Integration Tests (pre-filter, disabled)
// ============================================================================
//
// These tests reference VectorStore trait which was removed during refactoring.
// They are preserved for reference but disabled from compilation.

#[cfg(any())]
mod legacy_integration_tests {
    use crewchief_maproom::db::sqlite::hybrid::{HybridWeights, SemanticRanking};
    use crewchief_maproom::db::sqlite::SqliteStore;
    use crewchief_maproom::db::{ChunkRecord, FileRecord, VectorStore};
    use tempfile::tempdir;

    // ... legacy tests preserved for reference but not compiled ...
}
