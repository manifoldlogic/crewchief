//! Edge extraction accuracy tests
//!
//! These tests measure precision and recall against documented ground truth:
//! - Precision: Of all extracted edges, how many are correct?
//! - Recall: Of all expected edges, how many were extracted?
//! - F1 Score: Harmonic mean of precision and recall
//!
//! Success criteria: ≥85% precision for same-file calls (Phase 1)

use crewchief_maproom::db::SqliteStore;
use crewchief_maproom::indexer::scan_worktree;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Helper to create an in-memory store with schema
async fn setup_store() -> SqliteStore {
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);

    let db_name = format!(
        "file:memdb_accuracy_test_{}?mode=memory&cache=shared",
        counter
    );
    let store = SqliteStore::connect(&db_name).await.unwrap();
    store.migrate().await.unwrap();
    store
}

/// Edge representation for comparison (source → destination)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EdgePair {
    src: String,
    dst: String,
}

impl EdgePair {
    fn new(src: &str, dst: &str) -> Self {
        Self {
            src: src.to_string(),
            dst: dst.to_string(),
        }
    }
}

/// Get all edges from the database as (src_symbol, dst_symbol) pairs
async fn get_actual_edges(store: &SqliteStore) -> HashSet<EdgePair> {
    store
        .run(|conn| {
            let mut stmt = conn.prepare(
                "SELECT src.symbol_name, dst.symbol_name
                 FROM chunk_edges e
                 JOIN chunks src ON e.src_chunk_id = src.id
                 JOIN chunks dst ON e.dst_chunk_id = dst.id
                 WHERE e.type = 'calls'
                   AND src.symbol_name IS NOT NULL
                   AND dst.symbol_name IS NOT NULL",
            )?;

            let edges: Result<HashSet<_>, _> = stmt
                .query_map([], |row| {
                    Ok(EdgePair {
                        src: row.get::<_, String>(0)?,
                        dst: row.get::<_, String>(1)?,
                    })
                })?
                .collect();

            Ok(edges?)
        })
        .await
        .unwrap()
}

/// Calculate accuracy metrics
#[derive(Debug)]
struct AccuracyMetrics {
    precision: f64,
    recall: f64,
    f1_score: f64,
    true_positives: usize,
    false_positives: usize,
    false_negatives: usize,
}

impl AccuracyMetrics {
    fn calculate(expected: &HashSet<EdgePair>, actual: &HashSet<EdgePair>) -> Self {
        let true_positives = expected.intersection(actual).count();
        let false_positives = actual.len() - true_positives;
        let false_negatives = expected.len() - true_positives;

        let precision = if actual.is_empty() {
            0.0
        } else {
            true_positives as f64 / actual.len() as f64
        };

        let recall = if expected.is_empty() {
            0.0
        } else {
            true_positives as f64 / expected.len() as f64
        };

        let f1_score = if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * (precision * recall) / (precision + recall)
        };

        Self {
            precision,
            recall,
            f1_score,
            true_positives,
            false_positives,
            false_negatives,
        }
    }

    fn print_report(&self, test_name: &str) {
        println!("\n========== {} ==========", test_name);
        println!("Precision: {:.2}%", self.precision * 100.0);
        println!("Recall:    {:.2}%", self.recall * 100.0);
        println!("F1 Score:  {:.2}%", self.f1_score * 100.0);
        println!("True Positives:  {}", self.true_positives);
        println!("False Positives: {}", self.false_positives);
        println!("False Negatives: {}", self.false_negatives);
        println!("========================================\n");
    }
}

// ==================== Accuracy Tests ====================

#[tokio::test]
async fn test_accuracy_simple_repo() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/typescript_simple");

    // Scan the repository
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
    .unwrap();

    // Ground truth from README.md (same-file edges only)
    let mut expected_edges = HashSet::new();
    expected_edges.insert(EdgePair::new("calculate", "add"));
    expected_edges.insert(EdgePair::new("calculate", "multiply"));

    // Get actual edges
    let actual_edges = get_actual_edges(&store).await;

    // Calculate metrics
    let metrics = AccuracyMetrics::calculate(&expected_edges, &actual_edges);
    metrics.print_report("TypeScript Simple");

    // Assertions
    assert!(
        metrics.precision >= 0.85,
        "Precision should be ≥85%, got {:.2}%",
        metrics.precision * 100.0
    );

    assert!(
        metrics.recall >= 0.60,
        "Recall should be ≥60%, got {:.2}%",
        metrics.recall * 100.0
    );

    // Log any false positives/negatives for debugging
    if metrics.false_positives > 0 {
        let false_pos: Vec<_> = actual_edges.difference(&expected_edges).collect();
        println!("False positives: {:?}", false_pos);
    }

    if metrics.false_negatives > 0 {
        let false_neg: Vec<_> = expected_edges.difference(&actual_edges).collect();
        println!("False negatives: {:?}", false_neg);
    }
}

#[tokio::test]
async fn test_accuracy_methods_repo() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/typescript_methods");

    // Scan the repository
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
    .unwrap();

    // Ground truth from README.md (same-file edges)
    // Note: Top-level calls may not be detected if top-level isn't extracted as a chunk
    let mut expected_edges = HashSet::new();
    expected_edges.insert(EdgePair::new("multiply", "add"));
    expected_edges.insert(EdgePair::new("compute", "add"));
    expected_edges.insert(EdgePair::new("compute", "multiply"));
    expected_edges.insert(EdgePair::new("compute", "subtract"));
    // Top-level → compute is not included because top-level code isn't extracted as a named chunk

    // Get actual edges
    let actual_edges = get_actual_edges(&store).await;

    // Calculate metrics
    let metrics = AccuracyMetrics::calculate(&expected_edges, &actual_edges);
    metrics.print_report("TypeScript Methods");

    // Assertions
    assert!(
        metrics.precision >= 0.85,
        "Method call precision should be ≥85%, got {:.2}%",
        metrics.precision * 100.0
    );

    // Log any false positives/negatives for debugging
    if metrics.false_positives > 0 {
        let false_pos: Vec<_> = actual_edges.difference(&expected_edges).collect();
        println!("False positives: {:?}", false_pos);
    }

    if metrics.false_negatives > 0 {
        let false_neg: Vec<_> = expected_edges.difference(&actual_edges).collect();
        println!("False negatives: {:?}", false_neg);
    }
}

#[tokio::test]
async fn test_accuracy_complex_repo() {
    let store = setup_store().await;
    let test_repo = Path::new("tests/fixtures/edge_extraction/typescript_complex");

    // Scan the repository
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
    .unwrap();

    // Ground truth from README.md (core edges, excluding edge cases and top-level)
    let mut expected_edges = HashSet::new();
    expected_edges.insert(EdgePair::new("outer", "inner"));
    expected_edges.insert(EdgePair::new("inner", "helper"));
    expected_edges.insert(EdgePair::new("orchestrate", "outer"));
    expected_edges.insert(EdgePair::new("orchestrate", "inner"));
    expected_edges.insert(EdgePair::new("orchestrate", "helper"));
    expected_edges.insert(EdgePair::new("orchestrate", "map"));
    expected_edges.insert(EdgePair::new("process", "double"));  // Arrow function should be detected

    // Get actual edges
    let actual_edges = get_actual_edges(&store).await;

    // Calculate metrics
    let metrics = AccuracyMetrics::calculate(&expected_edges, &actual_edges);
    metrics.print_report("TypeScript Complex");

    // Assertions (slightly relaxed for complex patterns)
    assert!(
        metrics.precision >= 0.80,
        "Complex pattern precision should be ≥80%, got {:.2}%",
        metrics.precision * 100.0
    );

    assert!(
        metrics.recall >= 0.60,
        "Complex pattern recall should be ≥60%, got {:.2}%",
        metrics.recall * 100.0
    );

    // Log any false positives/negatives for debugging
    if metrics.false_positives > 0 {
        let false_pos: Vec<_> = actual_edges.difference(&expected_edges).collect();
        println!("False positives: {:?}", false_pos);
    }

    if metrics.false_negatives > 0 {
        let false_neg: Vec<_> = expected_edges.difference(&actual_edges).collect();
        println!("False negatives: {:?}", false_neg);
    }
}

#[tokio::test]
async fn test_overall_accuracy_across_fixtures() {
    let store = setup_store().await;

    // Scan all fixtures
    for fixture in &[
        "tests/fixtures/edge_extraction/typescript_simple",
        "tests/fixtures/edge_extraction/typescript_methods",
        "tests/fixtures/edge_extraction/typescript_complex",
    ] {
        scan_worktree(
            &store,
            "test_repo",
            "main",
            Path::new(fixture),
            "HEAD",
            4,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    }

    // Combined ground truth from all fixtures (excluding top-level calls)
    let mut expected_edges = HashSet::new();

    // typescript_simple
    expected_edges.insert(EdgePair::new("calculate", "add"));
    expected_edges.insert(EdgePair::new("calculate", "multiply"));

    // typescript_methods
    expected_edges.insert(EdgePair::new("multiply", "add"));
    expected_edges.insert(EdgePair::new("compute", "add"));
    expected_edges.insert(EdgePair::new("compute", "multiply"));
    expected_edges.insert(EdgePair::new("compute", "subtract"));

    // typescript_complex (core edges, excluding top-level)
    expected_edges.insert(EdgePair::new("outer", "inner"));
    expected_edges.insert(EdgePair::new("inner", "helper"));
    expected_edges.insert(EdgePair::new("orchestrate", "outer"));
    expected_edges.insert(EdgePair::new("orchestrate", "inner"));
    expected_edges.insert(EdgePair::new("orchestrate", "helper"));
    expected_edges.insert(EdgePair::new("orchestrate", "map"));
    expected_edges.insert(EdgePair::new("process", "double"));

    // Get actual edges
    let actual_edges = get_actual_edges(&store).await;

    // Calculate metrics
    let metrics = AccuracyMetrics::calculate(&expected_edges, &actual_edges);
    metrics.print_report("Overall Accuracy (All Fixtures)");

    // Overall accuracy should meet Phase 1 success criteria
    assert!(
        metrics.precision >= 0.85,
        "Overall precision should be ≥85%, got {:.2}%",
        metrics.precision * 100.0
    );

    println!(
        "\n✅ Phase 1 Success Criteria Met: Precision = {:.2}% (target: ≥85%)",
        metrics.precision * 100.0
    );
}

#[tokio::test]
async fn test_no_false_edges_in_empty_file() {
    use std::fs;
    use tempfile::TempDir;

    let store = setup_store().await;
    let temp_dir = TempDir::new().unwrap();
    let temp_repo = temp_dir.path();

    // Create empty TypeScript file
    let src_dir = temp_repo.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let file_path = src_dir.join("empty.ts");
    fs::write(&file_path, "// Empty file\n").unwrap();

    // Scan
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

    // Verify no edges were created
    let edge_count = store
        .run(|conn| {
            let count = conn.query_row("SELECT COUNT(*) FROM chunk_edges", [], |row| {
                row.get::<_, i64>(0)
            })?;
            Ok(count)
        })
        .await
        .unwrap();

    assert_eq!(edge_count, 0, "Empty file should not generate edges");
}

#[tokio::test]
async fn test_precision_with_noise() {
    use std::fs;
    use tempfile::TempDir;

    let store = setup_store().await;
    let temp_dir = TempDir::new().unwrap();
    let temp_repo = temp_dir.path();

    // Create file with noise (comments, strings, etc.)
    let src_dir = temp_repo.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let file_path = src_dir.join("noise.ts");
    fs::write(
        &file_path,
        r#"
// This comment mentions foo() but shouldn't create an edge
function foo() {
    return 42;
}

function bar() {
    // Call foo in comment
    const message = "calling foo()"; // String literal
    foo(); // Actual call
}
"#,
    )
    .unwrap();

    // Scan
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

    // Expected: only bar → foo (not comment/string mentions)
    let mut expected_edges = HashSet::new();
    expected_edges.insert(EdgePair::new("bar", "foo"));

    let actual_edges = get_actual_edges(&store).await;

    let metrics = AccuracyMetrics::calculate(&expected_edges, &actual_edges);
    metrics.print_report("Precision with Noise");

    // Should not create edges for comments/strings
    assert!(
        metrics.precision >= 0.85,
        "Should ignore foo() mentions in comments/strings, precision: {:.2}%",
        metrics.precision * 100.0
    );
}
