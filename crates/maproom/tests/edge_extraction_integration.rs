//! Edge extraction integration tests
//!
//! These tests validate edge extraction integration with the scan pipeline:
//! - scan_worktree creates edges in chunk_edges table
//! - Incremental updates recompute edges correctly
//! - Parse errors don't fail the scan
//! - Edge data is accurate and queryable

use maproom::context::relationships::find_test_files;
use maproom::context::{AssemblyStrategy, ContextBundle, DefaultAssemblyStrategy, ExpandOptions};
use maproom::db::traits::StoreGraph;
use maproom::db::types::ImportDirection;
use maproom::db::SqliteStore;
use maproom::db::Store;
use maproom::db::StoreMigration;
use maproom::indexer::{scan_worktree, upsert_files};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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
    use maproom::incremental::edge_updater::EdgeUpdater;
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

// ==================== Edge-depth spec F-A: Rust enablement ====================

/// Spec A1/A2: the single shared language gate includes rs.
#[test]
fn test_supports_call_extraction_predicate() {
    use maproom::indexer::edges::supports_call_extraction;
    // py is enabled once the F-D extractor lands and meets its accuracy gate (A2).
    for lang in ["ts", "tsx", "js", "jsx", "rs", "py"] {
        assert!(supports_call_extraction(lang), "{lang} must be supported");
    }
    for lang in ["go", "rb", "java", "md", "json"] {
        assert!(
            !supports_call_extraction(lang),
            "{lang} must not be enabled"
        );
    }
}

/// F-A end-to-end: scanning a Rust fixture produces calls edges, with
/// method-body calls attributed to the METHOD chunk (spec A3), and a
/// rescan is idempotent.
#[tokio::test]
async fn test_scan_creates_rust_edges() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/rust_simple");

    scan_worktree(
        &store,
        "rust_repo",
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

    assert!(
        has_edge(&store, "alpha", "beta", "calls").await,
        "alpha -> beta"
    );
    assert!(
        has_edge(&store, "multiply", "add", "calls").await,
        "multiply -> add must exist with the METHOD as src (A3)"
    );
    assert!(
        has_edge(&store, "test_alpha", "alpha", "calls").await,
        "cfg(test) fn call must be extracted"
    );
    // The impl container must NOT be a call source for add (A3 innermost).
    assert!(
        !has_edge(&store, "Calculator", "add", "calls").await,
        "container chunk must not own method-body calls"
    );

    // Idempotence: rescan changes nothing (UNIQUE + OR IGNORE, asserted).
    let before = get_edge_count(&store).await;
    scan_worktree(
        &store,
        "rust_repo",
        "main",
        test_repo,
        "HEAD",
        4,
        None,
        None,
        None,
    )
    .await
    .expect("Rescan should succeed");
    assert_eq!(
        before,
        get_edge_count(&store).await,
        "rescan must be idempotent"
    );
}

// ==================== Edge-depth spec F-C: Python import scoping ====================

/// Rows describing each `imports` edge: (src_relpath, dst_relpath, dst_symbol).
async fn import_edges(store: &SqliteStore) -> Vec<(String, String, Option<String>)> {
    store
        .run(|conn| {
            let mut stmt = conn.prepare(
                "SELECT sf.relpath, df.relpath, dst.symbol_name
                 FROM chunk_edges e
                 JOIN chunks src ON e.src_chunk_id = src.id
                 JOIN files  sf  ON src.file_id = sf.id
                 JOIN chunks dst ON e.dst_chunk_id = dst.id
                 JOIN files  df  ON dst.file_id = df.id
                 WHERE e.type = 'imports'
                 ORDER BY sf.relpath",
            )?;
            let rows: Result<Vec<_>, _> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })?
                .collect();
            Ok(rows?)
        })
        .await
        .unwrap()
}

/// Count chunks named `__imports__` (one per file that has imports).
async fn imports_chunk_count(store: &SqliteStore) -> i64 {
    store
        .run(|conn| {
            let n = conn.query_row(
                "SELECT COUNT(*) FROM chunks WHERE symbol_name = '__imports__'",
                [],
                |row| row.get(0),
            )?;
            Ok(n)
        })
        .await
        .unwrap()
}

/// Spec §6 (F-C): two files import the same symbol from a shared module; the
/// `imports` edges must be per-file-scoped, decoy-proof, external-proof, and
/// idempotent across rescans.
#[tokio::test]
async fn test_scan_scopes_python_imports() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/python_imports");

    scan_worktree(
        &store, "py_repo", "main", test_repo, "HEAD", 4, None, None, None,
    )
    .await
    .expect("Scan should succeed");

    // Exactly two __imports__ chunks (a.py and b.py; pkg/utils.py has no imports).
    assert_eq!(
        imports_chunk_count(&store).await,
        2,
        "exactly two __imports__ chunks (a.py, b.py)"
    );

    let edges = import_edges(&store).await;
    assert_eq!(
        edges.len(),
        2,
        "exactly two imports edges (os/some_external_lib must not resolve), got {edges:?}"
    );

    // Distinct src chunks — a.py's and b.py's — never collapsed onto one.
    let distinct_src: std::collections::HashSet<&str> =
        edges.iter().map(|(s, _, _)| s.as_str()).collect();
    assert_eq!(
        distinct_src.len(),
        2,
        "src chunks must be distinct, got {edges:?}"
    );
    assert!(distinct_src.contains("a.py") && distinct_src.contains("b.py"));

    // Both dst == pkg/utils.py's `helper`, never b.py's local decoy or any external.
    for (src, dst_relpath, dst_symbol) in &edges {
        assert_eq!(
            dst_relpath, "pkg/utils.py",
            "{src}'s import must resolve to pkg/utils.py, not {dst_relpath} (decoy/external leak)"
        );
        assert_eq!(
            dst_symbol.as_deref(),
            Some("helper"),
            "dst must be `helper`"
        );
    }

    // Idempotence: a second scan changes none of the above (spec §6 Gherkin).
    scan_worktree(
        &store, "py_repo", "main", test_repo, "HEAD", 4, None, None, None,
    )
    .await
    .expect("Rescan should succeed");
    let edges_again = import_edges(&store).await;
    assert_eq!(
        edges_again.len(),
        2,
        "rescan must not add/duplicate import edges"
    );
    assert_eq!(
        imports_chunk_count(&store).await,
        2,
        "rescan must not duplicate __imports__ chunks"
    );
    assert_eq!(
        edges, edges_again,
        "rescan must leave import edges unchanged"
    );
}

// ==================== Edge-depth spec F-B: cross-file call resolution ====================

/// Resolve a chunk id by symbol name and a relpath suffix (e.g. "helper.rs").
async fn chunk_id_by(store: &SqliteStore, relpath_suffix: &str, symbol: &str) -> i64 {
    let like = format!("%{relpath_suffix}");
    let symbol = symbol.to_string();
    store
        .run(move |conn| {
            let id = conn.query_row(
                "SELECT c.id FROM chunks c JOIN files f ON f.id = c.file_id \
                 WHERE c.symbol_name = ?1 AND f.relpath LIKE ?2",
                rusqlite::params![symbol, like],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(id)
        })
        .await
        .unwrap()
}

/// Does an edge (src_id, dst_id, type) exist?
async fn edge_exists(store: &SqliteStore, src_id: i64, dst_id: i64, etype: &str) -> bool {
    let etype = etype.to_string();
    store
        .run(move |conn| {
            let n: i64 = conn.query_row(
                "SELECT COUNT(*) FROM chunk_edges WHERE src_chunk_id=?1 AND dst_chunk_id=?2 AND type=?3",
                rusqlite::params![src_id, dst_id, etype],
                |row| row.get(0),
            )?;
            Ok(n > 0)
        })
        .await
        .unwrap()
}

/// The file_id of a chunk.
async fn file_of(store: &SqliteStore, chunk_id: i64) -> i64 {
    store
        .run(move |conn| {
            let id = conn.query_row(
                "SELECT file_id FROM chunks WHERE id=?1",
                rusqlite::params![chunk_id],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(id)
        })
        .await
        .unwrap()
}

/// Spec §5 Gherkin: cross-file rust caller appears; `find_callers` resolves it.
#[tokio::test]
async fn test_scan_creates_crossfile_rust_calls() {
    let store = setup_store().await;
    let repo = Path::new("tests/fixtures/edge_extraction/rust_crossfile");

    scan_worktree(&store, "xf_repo", "main", repo, "HEAD", 4, None, None, None)
        .await
        .expect("Scan should succeed");

    let caller_a = chunk_id_by(&store, "caller.rs", "caller_a").await;
    let helper_b = chunk_id_by(&store, "helper.rs", "helper_b").await;

    assert!(
        edge_exists(&store, caller_a, helper_b, "calls").await,
        "cross-file calls edge caller_a -> helper_b must exist"
    );
    // Gherkin: src.file != dst.file.
    assert_ne!(
        file_of(&store, caller_a).await,
        file_of(&store, helper_b).await,
        "the edge must be genuinely cross-file"
    );
    // find_callers(helper_b) returns caller_a's chunk.
    let callers = store.find_callers(helper_b, Some(1)).await.unwrap();
    assert!(
        callers.iter().any(|g| g.chunk_id == caller_a),
        "find_callers(helper_b) must include caller_a, got {callers:?}"
    );
}

/// Spec §5 Gherkin: cross-file TS caller (main.ts -> utils.ts calculate).
#[tokio::test]
async fn test_scan_creates_crossfile_ts_calls() {
    let store = setup_store().await;
    let repo = Path::new("tests/fixtures/edge_extraction/typescript_simple");

    scan_worktree(&store, "ts_xf", "main", repo, "HEAD", 4, None, None, None)
        .await
        .expect("Scan should succeed");

    let main = chunk_id_by(&store, "main.ts", "main").await;
    let calculate = chunk_id_by(&store, "utils.ts", "calculate").await;
    assert!(
        edge_exists(&store, main, calculate, "calls").await,
        "cross-file calls edge main -> calculate must exist"
    );
    assert_ne!(
        file_of(&store, main).await,
        file_of(&store, calculate).await,
        "the edge must be genuinely cross-file"
    );
}

/// Spec §5 Gherkin: ambiguity never guesses — two `multiply` defs, zero edges.
#[tokio::test]
async fn test_crossfile_ambiguity_never_guesses() {
    let store = setup_store().await;
    let repo = Path::new("tests/fixtures/edge_extraction/rust_ambiguous");

    scan_worktree(
        &store, "amb_repo", "main", repo, "HEAD", 4, None, None, None,
    )
    .await
    .expect("Scan should succeed");

    let run = chunk_id_by(&store, "caller.rs", "run").await;
    // No calls edge may originate from `run` (both `multiply` targets are ambiguous).
    let outgoing = store
        .run(move |conn| {
            let n: i64 = conn.query_row(
                "SELECT COUNT(*) FROM chunk_edges WHERE src_chunk_id=?1 AND type='calls'",
                rusqlite::params![run],
                |row| row.get(0),
            )?;
            Ok(n)
        })
        .await
        .unwrap();
    assert_eq!(
        outgoing, 0,
        "ambiguous multiply() call must produce zero edges"
    );
}

/// Spec B5 pin: inbound cross-file edge goes stale on single-file upsert and a full
/// rescan restores it (documented v1 policy — deliberate, not accidental).
#[tokio::test]
async fn test_inbound_edge_staleness_is_deliberate() {
    let store = setup_store().await;
    let repo = Path::new("tests/fixtures/edge_extraction/rust_crossfile");

    scan_worktree(
        &store,
        "stale_repo",
        "main",
        repo,
        "HEAD",
        4,
        None,
        None,
        None,
    )
    .await
    .expect("Scan should succeed");

    let caller_a = chunk_id_by(&store, "caller.rs", "caller_a").await;
    let helper_b = chunk_id_by(&store, "helper.rs", "helper_b").await;
    assert!(
        edge_exists(&store, caller_a, helper_b, "calls").await,
        "precondition: caller_a -> helper_b exists after scan"
    );

    // Re-index ONLY helper.rs: deletes its edges (src OR dst), so the inbound
    // caller_a -> helper_b edge is removed and NOT recomputed (caller.rs untouched).
    upsert_files(
        &store,
        "stale_repo",
        "main",
        repo,
        "HEAD",
        &[PathBuf::from("src/helper.rs")],
    )
    .await
    .expect("upsert should succeed");
    // chunk ids are stable (content-addressed upsert), so the same ids still name them.
    let caller_a2 = chunk_id_by(&store, "caller.rs", "caller_a").await;
    let helper_b2 = chunk_id_by(&store, "helper.rs", "helper_b").await;
    assert!(
        !edge_exists(&store, caller_a2, helper_b2, "calls").await,
        "inbound edge must be absent after single-file upsert (v1 staleness)"
    );

    // A full rescan restores it.
    scan_worktree(
        &store,
        "stale_repo",
        "main",
        repo,
        "HEAD",
        4,
        None,
        None,
        None,
    )
    .await
    .expect("Rescan should succeed");
    let caller_a3 = chunk_id_by(&store, "caller.rs", "caller_a").await;
    let helper_b3 = chunk_id_by(&store, "helper.rs", "helper_b").await;
    assert!(
        edge_exists(&store, caller_a3, helper_b3, "calls").await,
        "full rescan must restore the inbound edge"
    );
}

// ==================== Edge-depth spec F-B: test_of derivation ====================

/// Spec §5 Gherkin: test_of ⊆ calls-from-tests. `test_alpha` (a test) calling
/// `alpha` yields a test_of edge; `beta` (not a test) calling `alpha` does not.
/// find_test_files resolves the test; find_callers stays pure (calls-only).
#[tokio::test]
async fn test_scan_derives_test_of_edges() {
    let store = setup_store().await;
    let repo = Path::new("tests/fixtures/edge_extraction/rust_test_of");

    scan_worktree(&store, "to_repo", "main", repo, "HEAD", 4, None, None, None)
        .await
        .expect("Scan should succeed");

    let alpha = chunk_id_by(&store, "lib.rs", "alpha").await;
    let test_alpha = chunk_id_by(&store, "lib.rs", "test_alpha").await;
    let beta = chunk_id_by(&store, "lib.rs", "beta").await;

    // Both callers produce a `calls` edge.
    assert!(
        edge_exists(&store, test_alpha, alpha, "calls").await,
        "test_alpha -> alpha calls"
    );
    assert!(
        edge_exists(&store, beta, alpha, "calls").await,
        "beta -> alpha calls"
    );

    // Only the test caller produces a `test_of` edge (B7/B8).
    assert!(
        edge_exists(&store, test_alpha, alpha, "test_of").await,
        "test_alpha -> alpha test_of must exist"
    );
    assert!(
        !edge_exists(&store, beta, alpha, "test_of").await,
        "beta is not a test — no test_of edge may exist"
    );

    // Consumer: find_test_files(alpha) returns test_alpha (and not beta).
    let tests = find_test_files(&store, alpha).await.unwrap();
    let test_ids: Vec<i64> = tests.iter().map(|t| t.id).collect();
    assert!(
        test_ids.contains(&test_alpha),
        "find_test_files must return test_alpha, got {test_ids:?}"
    );
    assert!(
        !test_ids.contains(&beta),
        "find_test_files must not return the non-test beta"
    );

    // Purity: find_callers(alpha) returns both callers via `calls` ONLY — test_of
    // must not leak in.
    let callers = store.find_callers(alpha, Some(2)).await.unwrap();
    let caller_ids: Vec<i64> = callers.iter().map(|g| g.chunk_id).collect();
    assert!(caller_ids.contains(&test_alpha) && caller_ids.contains(&beta));
    assert!(
        callers.iter().all(|g| g.edge_type == "calls"),
        "find_callers must never surface test_of edges, got {callers:?}"
    );

    // The B9 consumer path (get_direct_edges incoming, filtered to test_of) sees it.
    let incoming = store
        .get_direct_edges(alpha, ImportDirection::Incoming)
        .await
        .unwrap();
    let test_of_srcs: Vec<i64> = incoming
        .iter()
        .filter(|e| e.edge_type == "test_of")
        .map(|e| e.chunk_id)
        .collect();
    assert_eq!(
        test_of_srcs,
        vec![test_alpha],
        "only test_alpha targets alpha via test_of"
    );
}

// ==================== Edge-depth DoD §2: tri-language E2E ====================

/// Resolve a chunk id by EXACT relpath (disambiguates app.py vs test_app.py).
async fn chunk_id_exact(store: &SqliteStore, relpath: &str, symbol: &str) -> i64 {
    let relpath = relpath.to_string();
    let symbol = symbol.to_string();
    store
        .run(move |conn| {
            let id = conn.query_row(
                "SELECT c.id FROM chunks c JOIN files f ON f.id = c.file_id \
                 WHERE c.symbol_name = ?1 AND f.relpath = ?2",
                rusqlite::params![symbol, relpath],
                |row| row.get::<_, i64>(0),
            )?;
            Ok(id)
        })
        .await
        .unwrap()
}

fn bundle_roles<'a>(bundle: &'a ContextBundle, role: &str) -> Vec<&'a str> {
    bundle
        .items
        .iter()
        .filter(|i| i.role == role)
        .map(|i| i.reason.as_str())
        .collect()
}

/// DoD §2: one worktree with rust + ts + py — cross-file calls (rust & ts & py),
/// per-file-scoped python imports, test_of per language, and a Wave-1-default
/// context bundle containing caller + callee + import + test items.
#[tokio::test]
async fn test_trilang_end_to_end() {
    let store = setup_store().await;
    let repo = Path::new("tests/fixtures/edge_extraction/trilang");

    scan_worktree(
        &store, "tri_repo", "main", repo, "HEAD", 4, None, None, None,
    )
    .await
    .expect("Scan should succeed");

    // ---- cross-file calls edges (src.file != dst.file) per language ----
    for (lang, sfile, src, dfile, dst) in [
        ("rust", "caller.rs", "r_caller", "helper.rs", "r_helper"),
        ("ts", "main.ts", "t_main", "util.ts", "t_util"),
        ("py", "app.py", "p_caller", "pkg/mod.py", "p_helper"),
    ] {
        let s = chunk_id_exact(&store, sfile, src).await;
        let d = chunk_id_exact(&store, dfile, dst).await;
        assert!(
            edge_exists(&store, s, d, "calls").await,
            "{lang}: cross-file {src} -> {dst} calls edge missing"
        );
        assert_ne!(
            file_of(&store, s).await,
            file_of(&store, d).await,
            "{lang}: {src} -> {dst} must be cross-file"
        );
    }

    // ---- scoped python imports ----
    let app_imports = chunk_id_exact(&store, "app.py", "__imports__").await;
    let p_helper = chunk_id_exact(&store, "pkg/mod.py", "p_helper").await;
    assert!(
        edge_exists(&store, app_imports, p_helper, "imports").await,
        "app.py must import pkg/mod.py's p_helper (scoped)"
    );
    let testapp_imports = chunk_id_exact(&store, "test_app.py", "__imports__").await;
    let p_caller = chunk_id_exact(&store, "app.py", "p_caller").await;
    assert!(
        edge_exists(&store, testapp_imports, p_caller, "imports").await,
        "test_app.py must import app.py's p_caller (scoped)"
    );

    // ---- test_of per language ----
    for (lang, tfile, test, dfile, dst) in [
        (
            "rust",
            "caller.rs",
            "test_r_caller",
            "caller.rs",
            "r_caller",
        ),
        ("ts", "main.test.ts", "test_t_main", "main.ts", "t_main"),
        ("py", "test_app.py", "test_p_caller", "app.py", "p_caller"),
    ] {
        let t = chunk_id_exact(&store, tfile, test).await;
        let d = chunk_id_exact(&store, dfile, dst).await;
        assert!(
            edge_exists(&store, t, d, "test_of").await,
            "{lang}: test_of {test} -> {dst} missing"
        );
    }

    // ---- Wave-1-default context bundle for p_caller: caller+callee+import+test ----
    let store: Arc<dyn Store + Send + Sync> = Arc::new(store);
    let assembler = DefaultAssemblyStrategy::new(store);
    let bundle = assembler
        .assemble(p_caller, 8000, ExpandOptions::default())
        .await
        .expect("assemble should succeed");

    for role in ["primary", "caller", "callee", "import", "test"] {
        assert!(
            !bundle_roles(&bundle, role).is_empty(),
            "context bundle for p_caller missing a `{role}` item; items = {:?}",
            bundle
                .items
                .iter()
                .map(|i| (&i.role, &i.reason))
                .collect::<Vec<_>>()
        );
    }
}
