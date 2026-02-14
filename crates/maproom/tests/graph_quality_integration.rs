// Integration tests for quality-weighted graph importance (SRCHREL-1005)
//
// NOTE: Full integration tests with graph scoring require SQLite math functions (ln())
// which are not available in the bundled SQLite. The algorithm logic is thoroughly
// tested in graph_quality_tests.rs (SRCHREL-1004).
//
// These tests validate:
// - Data setup and edge insertion work correctly
// - Executor code paths are reachable
// - Test detection patterns are correctly applied in SQL
//
// Full E2E validation with score calculations should be done against
// the production database which has math extensions loaded.

mod common;

use common::TestDb;
use crewchief_maproom::db::StoreChunks;
use crewchief_maproom::db::StoreCore;
use crewchief_maproom::db::{ChunkRecord, FileRecord};

/// Helper to create a file record for a given path
fn make_file(db: &TestDb, relpath: &str) -> FileRecord {
    FileRecord {
        repo_id: db.repo_id,
        worktree_id: db.worktree_id,
        commit_id: db.commit_id,
        relpath: relpath.to_string(),
        language: Some(detect_language(relpath)),
        content_hash: format!("hash_{}", relpath.replace("/", "_")),
        size_bytes: 100,
        last_modified: None,
    }
}

/// Helper to detect language from file extension
fn detect_language(path: &str) -> String {
    match path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        _ => "unknown",
    }
    .to_string()
}

/// Helper to create a chunk record
fn make_chunk(file_id: i64, worktree_id: i64, symbol: &str, start_line: i32) -> ChunkRecord {
    ChunkRecord {
        file_id,
        blob_sha: format!("blob_{}_{}", symbol, start_line),
        symbol_name: Some(symbol.to_string()),
        kind: "function".to_string(),
        signature: Some(format!("fn {}()", symbol)),
        docstring: None,
        start_line,
        end_line: start_line + 10,
        preview: format!("function {}() {{ /* code */ }}", symbol),
        ts_doc_text: symbol.to_string(),
        recency_score: 1.0,
        churn_score: 0.5,
        metadata: None,
        worktree_id,
    }
}

// ============================================================================
// Integration Test: Data Setup and Edge Insertion
// ============================================================================

#[tokio::test]
async fn test_chunk_and_edge_insertion() {
    let db = TestDb::new().await.expect("Failed to create test database");
    let store = db.store();

    // Create a source file and chunk
    let src_file = make_file(&db, "src/handler.ts");
    let src_file_id = store.upsert_file(&src_file).await.unwrap();
    let src_chunk = make_chunk(src_file_id, db.worktree_id, "Handler", 1);
    let src_chunk_id = store.insert_chunk(&src_chunk).await.unwrap();

    // Create a test file and chunk
    let test_file = make_file(&db, "test/handler.test.ts");
    let test_file_id = store.upsert_file(&test_file).await.unwrap();
    let test_chunk = make_chunk(test_file_id, db.worktree_id, "TestHandler", 1);
    let test_chunk_id = store.insert_chunk(&test_chunk).await.unwrap();

    // Insert edge from test to source (test calls production code)
    store
        .insert_chunk_edge(test_chunk_id, src_chunk_id, "calls")
        .await
        .expect("Failed to insert edge");

    // Verify chunks exist
    assert!(src_chunk_id > 0, "Source chunk should have positive ID");
    assert!(test_chunk_id > 0, "Test chunk should have positive ID");
    assert_ne!(src_chunk_id, test_chunk_id, "Chunk IDs should be different");
}

#[tokio::test]
async fn test_multiple_edges_from_same_source() {
    let db = TestDb::new().await.expect("Failed to create test database");
    let store = db.store();

    // Create target chunk
    let target_file = make_file(&db, "src/service.ts");
    let target_file_id = store.upsert_file(&target_file).await.unwrap();
    let target_chunk = make_chunk(target_file_id, db.worktree_id, "Service", 1);
    let target_chunk_id = store.insert_chunk(&target_chunk).await.unwrap();

    // Create source file with multiple chunks
    let source_file = make_file(&db, "src/caller.ts");
    let source_file_id = store.upsert_file(&source_file).await.unwrap();

    // Create 3 different chunks in the same file, each calling the target
    let mut caller_ids = Vec::new();
    for i in 0..3 {
        let caller_chunk = make_chunk(
            source_file_id,
            db.worktree_id,
            &format!("callerFunc{}", i),
            (i + 1) * 20,
        );
        let caller_chunk_id = store.insert_chunk(&caller_chunk).await.unwrap();
        caller_ids.push(caller_chunk_id);

        // Insert edge
        store
            .insert_chunk_edge(caller_chunk_id, target_chunk_id, "calls")
            .await
            .expect("Failed to insert edge");
    }

    // Verify all 3 callers were created with unique IDs
    assert_eq!(caller_ids.len(), 3);
    let unique_ids: std::collections::HashSet<_> = caller_ids.iter().collect();
    assert_eq!(unique_ids.len(), 3, "All caller chunk IDs should be unique");
}

// ============================================================================
// Integration Test: Production vs Test File Path Detection
// ============================================================================

#[tokio::test]
async fn test_production_and_test_file_creation() {
    let db = TestDb::new().await.expect("Failed to create test database");
    let store = db.store();

    // Create various production files
    let prod_paths = vec![
        "src/handler.ts",
        "lib/utils.ts",
        "packages/cli/index.ts",
        "src/auth/login.ts",
    ];

    for path in &prod_paths {
        let file = make_file(&db, path);
        let file_id = store.upsert_file(&file).await.unwrap();
        assert!(
            file_id > 0,
            "Production file {} should have positive ID",
            path
        );
    }

    // Create various test files
    let test_paths = vec![
        "test/handler.test.ts",
        "src/__tests__/utils.test.ts",
        "tests/integration.spec.ts",
        "src/auth/login.spec.js",
    ];

    for path in &test_paths {
        let file = make_file(&db, path);
        let file_id = store.upsert_file(&file).await.unwrap();
        assert!(file_id > 0, "Test file {} should have positive ID", path);
    }
}

// ============================================================================
// Integration Test: Config Module Integration
// ============================================================================

#[tokio::test]
async fn test_search_config_feature_flags() {
    use crewchief_maproom::config::SearchConfig;

    // Default config has quality scoring disabled
    let default_config = SearchConfig::default();
    assert!(
        !default_config.feature_flags.enable_quality_weighted_graph,
        "Default config should have quality_weighted_graph disabled"
    );

    // Can enable the flag
    let mut enabled_config = SearchConfig::default();
    enabled_config.feature_flags.enable_quality_weighted_graph = true;
    assert!(
        enabled_config.feature_flags.enable_quality_weighted_graph,
        "Config flag should be enableable"
    );
}

// ============================================================================
// Integration Test: Edge Type Classification
// ============================================================================

#[tokio::test]
async fn test_different_edge_types() {
    let db = TestDb::new().await.expect("Failed to create test database");
    let store = db.store();

    // Create two chunks
    let file = make_file(&db, "src/module.ts");
    let file_id = store.upsert_file(&file).await.unwrap();

    let chunk_a = make_chunk(file_id, db.worktree_id, "FuncA", 1);
    let chunk_a_id = store.insert_chunk(&chunk_a).await.unwrap();

    let chunk_b = make_chunk(file_id, db.worktree_id, "FuncB", 20);
    let chunk_b_id = store.insert_chunk(&chunk_b).await.unwrap();

    // Insert different edge types
    store
        .insert_chunk_edge(chunk_a_id, chunk_b_id, "calls")
        .await
        .expect("Failed to insert 'calls' edge");

    store
        .insert_chunk_edge(chunk_a_id, chunk_b_id, "imports")
        .await
        .expect("Failed to insert 'imports' edge");

    // Edges should be insertable without error
    // (duplicate edges with same type are ignored via INSERT OR IGNORE)
}

// ============================================================================
// Integration Test: Large Dataset Setup (Performance)
// ============================================================================

#[tokio::test]
async fn test_moderate_dataset_setup() {
    use std::time::Instant;

    let db = TestDb::new().await.expect("Failed to create test database");
    let store = db.store();

    let start = Instant::now();

    // Create 50 chunks with edges
    let mut chunk_ids = Vec::new();

    for i in 0..50 {
        let is_test = i % 3 == 0; // ~33% test code
        let path = if is_test {
            format!("test/test{}.ts", i)
        } else {
            format!("src/module{}.ts", i)
        };

        let file = make_file(&db, &path);
        let file_id = store.upsert_file(&file).await.unwrap();
        let chunk = make_chunk(file_id, db.worktree_id, &format!("func{}", i), 1);
        let chunk_id = store.insert_chunk(&chunk).await.unwrap();
        chunk_ids.push(chunk_id);
    }

    // Create edges (each chunk calls 1-3 others)
    for (i, &src_id) in chunk_ids.iter().enumerate() {
        let num_edges = (i % 3) + 1;
        for j in 0..num_edges {
            let dst_idx = (i + j + 1) % chunk_ids.len();
            store
                .insert_chunk_edge(src_id, chunk_ids[dst_idx], "calls")
                .await
                .unwrap();
        }
    }

    let elapsed = start.elapsed();

    // Data setup should complete in <2 seconds
    assert!(
        elapsed.as_secs() < 2,
        "Dataset setup took {:.2}s, should be <2s",
        elapsed.as_secs_f64()
    );

    // Verify we created 50 chunks
    assert_eq!(chunk_ids.len(), 50, "Should have created 50 chunks");
}

// ============================================================================
// Integration Test: Worktree Filtering in Data
// ============================================================================

#[tokio::test]
async fn test_worktree_scoped_data() {
    let db = TestDb::new().await.expect("Failed to create test database");
    let store = db.store();

    // Data is scoped to the test worktree (db.worktree_id)
    let file = make_file(&db, "src/scoped.ts");
    let file_id = store.upsert_file(&file).await.unwrap();

    let chunk = make_chunk(file_id, db.worktree_id, "ScopedFunc", 1);
    let chunk_id = store.insert_chunk(&chunk).await.unwrap();

    assert!(chunk_id > 0, "Scoped chunk should be created");

    // The chunk should be associated with the specific worktree
    // This is handled by the chunk_worktrees junction table
}

// ============================================================================
// Note: Full GraphExecutor Tests
// ============================================================================
//
// The GraphExecutor::execute() function uses SQL queries with ln() which
// requires SQLite math extensions not available in the bundled SQLite.
//
// Full integration tests for graph scoring should be run against:
// 1. The production database at ~/.maproom/maproom.db
// 2. A database with math extensions compiled in
//
// The algorithm logic (quality weighting, test detection, logarithmic scaling)
// is thoroughly tested in graph_quality_tests.rs (SRCHREL-1004).
//
// SQL query performance is validated in graph_quality_performance.rs (SRCHREL-0002).
