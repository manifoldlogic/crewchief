//! Edge extraction integration tests
//!
//! These tests validate edge extraction integration with the scan pipeline:
//! - scan_worktree creates edges in chunk_edges table
//! - Incremental updates recompute edges correctly
//! - Parse errors don't fail the scan
//! - Edge data is accurate and queryable

use crewchief_maproom::db::SqliteStore;
use crewchief_maproom::db::StoreMigration;
use crewchief_maproom::indexer::scan_worktree;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Helper to create an in-memory store with schema
async fn setup_store() -> SqliteStore {
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);

    // Use unique in-memory database for each test
    let db_name = format!("file:memdb_edge_test_{}?mode=memory&cache=shared", counter);
    let store = SqliteStore::connect(&db_name).await.unwrap();
    store.migrate().await.unwrap();
    store
}

/// Helper to count edges in chunk_edges table
async fn get_edge_count(store: &SqliteStore) -> i64 {
    store
        .run(|conn| {
            let count = conn.query_row("SELECT COUNT(*) FROM chunk_edges", [], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .unwrap()
}

/// Helper to check if a specific edge exists
async fn has_edge(
    store: &SqliteStore,
    src_symbol: &str,
    dst_symbol: &str,
    edge_type: &str,
) -> bool {
    let src = src_symbol.to_string();
    let dst = dst_symbol.to_string();
    let etype = edge_type.to_string();

    store
        .run(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM chunk_edges e
                 JOIN chunks src ON e.src_chunk_id = src.id
                 JOIN chunks dst ON e.dst_chunk_id = dst.id
                 WHERE src.symbol_name = ?1 AND dst.symbol_name = ?2 AND e.type = ?3",
                rusqlite::params![&src, &dst, &etype],
                |row| row.get(0),
            )?;
            Ok(count > 0)
        })
        .await
        .unwrap()
}

// ==================== Integration Tests ====================

#[tokio::test]
async fn test_scan_creates_edges_simple() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/typescript_simple");

    // Scan the test repository
    scan_worktree(
        &store,
        "test_repo",
        "main",
        test_repo,
        "HEAD",
        4,
        None,
        None,
        None,
    )
    .await
    .expect("Scan should succeed");

    // Verify edges were created
    let edge_count = get_edge_count(&store).await;
    assert!(
        edge_count >= 2,
        "Expected at least 2 same-file edges in utils.ts, got {}",
        edge_count
    );

    // Verify specific edge: calculate → add
    let has_calculate_to_add = has_edge(&store, "calculate", "add", "calls").await;
    assert!(has_calculate_to_add, "Expected edge from calculate to add");

    // Verify specific edge: calculate → multiply
    let has_calculate_to_multiply = has_edge(&store, "calculate", "multiply", "calls").await;
    assert!(
        has_calculate_to_multiply,
        "Expected edge from calculate to multiply"
    );
}

#[tokio::test]
async fn test_scan_creates_edges_methods() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/typescript_methods");

    // Scan the test repository
    scan_worktree(
        &store,
        "test_repo",
        "main",
        test_repo,
        "HEAD",
        4,
        None,
        None,
        None,
    )
    .await
    .expect("Scan should succeed");

    // Verify edges were created
    let edge_count = get_edge_count(&store).await;
    assert!(
        edge_count >= 4,
        "Expected at least 4 method call edges, got {}",
        edge_count
    );

    // Verify specific edges
    let has_multiply_to_add = has_edge(&store, "multiply", "add", "calls").await;
    assert!(has_multiply_to_add, "Expected edge from multiply to add");

    let has_compute_to_add = has_edge(&store, "compute", "add", "calls").await;
    assert!(has_compute_to_add, "Expected edge from compute to add");

    let has_compute_to_multiply = has_edge(&store, "compute", "multiply", "calls").await;
    assert!(
        has_compute_to_multiply,
        "Expected edge from compute to multiply"
    );

    let has_compute_to_subtract = has_edge(&store, "compute", "subtract", "calls").await;
    assert!(
        has_compute_to_subtract,
        "Expected edge from compute to subtract"
    );
}

#[tokio::test]
async fn test_scan_creates_edges_complex() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/typescript_complex");

    // Scan the test repository
    scan_worktree(
        &store,
        "test_repo",
        "main",
        test_repo,
        "HEAD",
        4,
        None,
        None,
        None,
    )
    .await
    .expect("Scan should succeed");

    // Verify edges were created
    let edge_count = get_edge_count(&store).await;
    assert!(
        edge_count >= 6,
        "Expected at least 6 edges for complex patterns, got {}",
        edge_count
    );

    // Verify nested call chain
    let has_outer_to_inner = has_edge(&store, "outer", "inner", "calls").await;
    assert!(has_outer_to_inner, "Expected edge from outer to inner");

    let has_inner_to_helper = has_edge(&store, "inner", "helper", "calls").await;
    assert!(has_inner_to_helper, "Expected edge from inner to helper");

    // Verify orchestrate calls multiple functions
    let has_orchestrate_to_outer = has_edge(&store, "orchestrate", "outer", "calls").await;
    assert!(
        has_orchestrate_to_outer,
        "Expected edge from orchestrate to outer"
    );

    let has_orchestrate_to_inner = has_edge(&store, "orchestrate", "inner", "calls").await;
    assert!(
        has_orchestrate_to_inner,
        "Expected edge from orchestrate to inner"
    );

    let has_orchestrate_to_helper = has_edge(&store, "orchestrate", "helper", "calls").await;
    assert!(
        has_orchestrate_to_helper,
        "Expected edge from orchestrate to helper"
    );
}

#[tokio::test]
async fn test_incremental_update_recomputes_edges() {
    use crewchief_maproom::incremental::edge_updater::EdgeUpdater;
    use std::fs;
    use tempfile::TempDir;

    let store = setup_store().await;
    let temp_dir = TempDir::new().unwrap();
    let temp_repo = temp_dir.path();

    // Create initial TypeScript file
    let src_dir = temp_repo.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let file_path = src_dir.join("test.ts");

    // Initial content with one function call in bar, none in baz
    fs::write(
        &file_path,
        "function foo() { return 42; }\nfunction bar() { return 1; }\nfunction baz() { return 2; }",
    )
    .unwrap();

    // Initial scan
    scan_worktree(
        &store,
        "test_repo",
        "main",
        temp_repo,
        "HEAD",
        4,
        Some(vec!["ts".to_string()]),
        None,
        None,
    )
    .await
    .unwrap();

    let initial_count = get_edge_count(&store).await;
    assert_eq!(initial_count, 0, "Should have no edges initially");

    // Modify file: add calls to foo() in bar and baz (without changing chunk boundaries)
    fs::write(
        &file_path,
        "function foo() { return 42; }\nfunction bar() { foo(); return 1; }\nfunction baz() { foo(); return 2; }",
    )
    .unwrap();

    // Get file_id
    let file_id = store
        .run(move |conn| {
            let id = conn.query_row(
                "SELECT id FROM files WHERE relpath LIKE '%test.ts'",
                [],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(id)
        })
        .await
        .unwrap();

    // Trigger incremental update
    let edge_updater = EdgeUpdater::new(std::sync::Arc::new(store.clone()));
    edge_updater.update_edges(file_id).await.unwrap();

    // Verify edge count increased (should have bar→foo and baz→foo)
    let updated_count = get_edge_count(&store).await;
    assert!(
        updated_count >= 2,
        "Expected at least 2 edges after adding calls (bar→foo, baz→foo), got {}",
        updated_count
    );
}

#[tokio::test]
async fn test_parse_errors_dont_fail_scan() {
    use std::fs;
    use tempfile::TempDir;

    let store = setup_store().await;
    let temp_dir = TempDir::new().unwrap();
    let temp_repo = temp_dir.path();

    // Create directory structure
    let src_dir = temp_repo.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create valid TypeScript file
    let valid_file = src_dir.join("valid.ts");
    fs::write(&valid_file, "function valid() { return 42; }").unwrap();

    // Create invalid TypeScript file (syntax error)
    let invalid_file = src_dir.join("invalid.ts");
    fs::write(&invalid_file, "function invalid( { broken syntax }").unwrap();

    // Scan should succeed despite invalid file
    let result = scan_worktree(
        &store,
        "test_repo",
        "main",
        temp_repo,
        "HEAD",
        4,
        Some(vec!["ts".to_string()]),
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Scan should not fail on parse errors");

    // Verify valid files still got indexed (check for chunks)
    let chunk_count = store
        .run(|conn| {
            let count = conn.query_row("SELECT COUNT(*) FROM chunks", [], |row| {
                row.get::<_, i64>(0)
            })?;
            Ok(count)
        })
        .await
        .unwrap();

    assert!(chunk_count > 0, "Valid files should still have chunks");
}

#[tokio::test]
async fn test_edges_queryable_by_type() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/typescript_simple");

    // Scan the test repository
    scan_worktree(
        &store,
        "test_repo",
        "main",
        test_repo,
        "HEAD",
        4,
        None,
        None,
        None,
    )
    .await
    .expect("Scan should succeed");

    // Query for 'calls' edges specifically
    let calls_count = store
        .run(|conn| {
            let count = conn.query_row(
                "SELECT COUNT(*) FROM chunk_edges WHERE type = 'calls'",
                [],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(count)
        })
        .await
        .unwrap();

    assert!(calls_count >= 2, "Should have at least 2 'calls' edges");

    // Verify we can query edges with chunk metadata
    let edges_with_symbols = store
        .run(|conn| {
            let mut stmt = conn.prepare(
                "SELECT src.symbol_name, dst.symbol_name, e.type
                 FROM chunk_edges e
                 JOIN chunks src ON e.src_chunk_id = src.id
                 JOIN chunks dst ON e.dst_chunk_id = dst.id
                 WHERE e.type = 'calls'",
            )?;

            let edges: Result<Vec<_>, _> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect();

            Ok(edges?)
        })
        .await
        .unwrap();

    assert!(
        !edges_with_symbols.is_empty(),
        "Should be able to query edges with symbol names"
    );
}
