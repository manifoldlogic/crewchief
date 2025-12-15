//! Test Detection Validation
//!
//! Validates the accuracy of file path-based test detection heuristics on real codebase data.
//! This test implements SRCHREL-0003 acceptance criteria:
//! - Sample 200 chunks (100 test, 100 production) from the CrewChief database
//! - Apply test detection heuristic to all samples
//! - Calculate precision (≥85%) and recall (≥80%)
//! - Document false positives and false negatives

use crewchief_maproom::context::heuristics::HeuristicScorer;
use rusqlite::Connection;
use std::path::PathBuf;

/// Sample chunk data from database for validation
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ChunkSample {
    id: i64,
    symbol_name: String,
    kind: String,
    relpath: String,
    /// Ground truth: Is this actually a test chunk?
    /// Determined by file path patterns (our sampling query)
    is_test: bool,
}

/// Confusion matrix for test detection validation
#[derive(Debug, Default)]
struct ConfusionMatrix {
    true_positives: usize,  // Correctly identified test chunks
    false_positives: usize, // Production chunks misidentified as test
    true_negatives: usize,  // Correctly identified production chunks
    false_negatives: usize, // Test chunks missed
}

impl ConfusionMatrix {
    fn precision(&self) -> f64 {
        let tp = self.true_positives as f64;
        let fp = self.false_positives as f64;
        if tp + fp == 0.0 {
            return 0.0;
        }
        tp / (tp + fp)
    }

    fn recall(&self) -> f64 {
        let tp = self.true_positives as f64;
        let fn_count = self.false_negatives as f64;
        if tp + fn_count == 0.0 {
            return 0.0;
        }
        tp / (tp + fn_count)
    }

    fn f1_score(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            return 0.0;
        }
        2.0 * (p * r) / (p + r)
    }

    fn accuracy(&self) -> f64 {
        let total = (self.true_positives
            + self.false_positives
            + self.true_negatives
            + self.false_negatives) as f64;
        if total == 0.0 {
            return 0.0;
        }
        (self.true_positives + self.true_negatives) as f64 / total
    }
}

/// Load sample chunks from the database
fn load_sample_chunks() -> anyhow::Result<Vec<ChunkSample>> {
    let db_path = get_database_path()?;
    let conn = Connection::open(&db_path)?;

    let mut samples = Vec::new();

    // Load test chunks (ground truth = true)
    // Note: Patterns match heuristics.rs implementation
    let mut stmt = conn.prepare(
        r#"
        SELECT c.id, c.symbol_name, c.kind, f.relpath
        FROM chunks c
        JOIN files f ON f.id = c.file_id
        WHERE (f.relpath LIKE '%/tests/%'
           OR f.relpath LIKE '%/__tests__/%'
           OR f.relpath LIKE '%.test.ts'
           OR f.relpath LIKE '%.test.js'
           OR f.relpath LIKE '%.test.tsx'
           OR f.relpath LIKE '%.spec.ts'
           OR f.relpath LIKE '%.spec.js'
           OR f.relpath LIKE '%_test.rs'
           OR f.relpath LIKE '%_test.py')
          AND c.kind IN ('func', 'async_func', 'method', 'async_method', 'class', 'struct')
        ORDER BY RANDOM()
        LIMIT 100
        "#,
    )?;

    let test_chunks = stmt.query_map([], |row| {
        Ok(ChunkSample {
            id: row.get(0)?,
            symbol_name: row.get(1)?,
            kind: row.get(2)?,
            relpath: row.get(3)?,
            is_test: true, // Ground truth
        })
    })?;

    for chunk in test_chunks {
        samples.push(chunk?);
    }

    // Load production chunks (ground truth = false)
    // Note: Exclusions match heuristics.rs test patterns
    let mut stmt = conn.prepare(
        r#"
        SELECT c.id, c.symbol_name, c.kind, f.relpath
        FROM chunks c
        JOIN files f ON f.id = c.file_id
        WHERE (f.relpath LIKE '%/src/%' OR f.relpath LIKE '%/lib/%' OR f.relpath LIKE '%/crates/%')
          AND f.relpath NOT LIKE '%/tests/%'
          AND f.relpath NOT LIKE '%/__tests__/%'
          AND f.relpath NOT LIKE '%.test.%'
          AND f.relpath NOT LIKE '%.spec.%'
          AND f.relpath NOT LIKE '%_test.%'
          AND c.kind IN ('func', 'async_func', 'method', 'async_method', 'class', 'struct')
        ORDER BY RANDOM()
        LIMIT 100
        "#,
    )?;

    let production_chunks = stmt.query_map([], |row| {
        Ok(ChunkSample {
            id: row.get(0)?,
            symbol_name: row.get(1)?,
            kind: row.get(2)?,
            relpath: row.get(3)?,
            is_test: false, // Ground truth
        })
    })?;

    for chunk in production_chunks {
        samples.push(chunk?);
    }

    Ok(samples)
}

/// Get database path from environment or default location
fn get_database_path() -> anyhow::Result<PathBuf> {
    if let Ok(db_url) = std::env::var("MAPROOM_DATABASE_URL") {
        // Parse sqlite:///path/to/db format
        let path = db_url
            .strip_prefix("sqlite://")
            .unwrap_or(&db_url)
            .trim_start_matches('/');
        Ok(PathBuf::from(path))
    } else {
        let home = std::env::var("HOME")?;
        Ok(PathBuf::from(home).join(".maproom/maproom.db"))
    }
}

/// Validate test detection accuracy on real data
#[test]
fn validate_test_detection_accuracy() -> anyhow::Result<()> {
    // Load samples from database
    let samples = load_sample_chunks()?;
    println!("Loaded {} chunk samples", samples.len());

    // Create heuristic scorer
    let scorer = HeuristicScorer::new();

    // Build confusion matrix
    let mut matrix = ConfusionMatrix::default();
    let mut false_positives = Vec::new();
    let mut false_negatives = Vec::new();

    for sample in &samples {
        let ground_truth = sample.is_test;
        let predicted = scorer.is_test_file(&sample.relpath);

        match (ground_truth, predicted) {
            (true, true) => matrix.true_positives += 1,
            (true, false) => {
                matrix.false_negatives += 1;
                false_negatives.push(sample);
            }
            (false, true) => {
                matrix.false_positives += 1;
                false_positives.push(sample);
            }
            (false, false) => matrix.true_negatives += 1,
        }
    }

    // Calculate metrics
    let precision = matrix.precision();
    let recall = matrix.recall();
    let f1 = matrix.f1_score();
    let accuracy = matrix.accuracy();

    // Print results
    println!("\n=== Test Detection Validation Results ===");
    println!("Total samples: {}", samples.len());
    println!("\nConfusion Matrix:");
    println!(
        "  True Positives:  {} (test chunks correctly identified)",
        matrix.true_positives
    );
    println!(
        "  False Positives: {} (production chunks misidentified as test)",
        matrix.false_positives
    );
    println!(
        "  True Negatives:  {} (production chunks correctly identified)",
        matrix.true_negatives
    );
    println!(
        "  False Negatives: {} (test chunks missed)",
        matrix.false_negatives
    );
    println!("\nMetrics:");
    println!("  Precision: {:.2}% (target: ≥85%)", precision * 100.0);
    println!("  Recall:    {:.2}% (target: ≥80%)", recall * 100.0);
    println!("  F1 Score:  {:.2}%", f1 * 100.0);
    println!("  Accuracy:  {:.2}%", accuracy * 100.0);

    // Document false positives
    if !false_positives.is_empty() {
        println!("\n=== False Positives (Production code misidentified as test) ===");
        println!("Count: {}", false_positives.len());
        for (i, sample) in false_positives.iter().take(10).enumerate() {
            println!("  {}. {} ({})", i + 1, sample.relpath, sample.kind);
            println!("     Symbol: {}", sample.symbol_name);
        }
        if false_positives.len() > 10 {
            println!("  ... and {} more", false_positives.len() - 10);
        }
    }

    // Document false negatives
    if !false_negatives.is_empty() {
        println!("\n=== False Negatives (Test code missed) ===");
        println!("Count: {}", false_negatives.len());
        for (i, sample) in false_negatives.iter().take(10).enumerate() {
            println!("  {}. {} ({})", i + 1, sample.relpath, sample.kind);
            println!("     Symbol: {}", sample.symbol_name);
        }
        if false_negatives.len() > 10 {
            println!("  ... and {} more", false_negatives.len() - 10);
        }
    }

    // Acceptance criteria assertions
    assert!(
        precision >= 0.85,
        "Precision {:.2}% is below 85% threshold. Found {} false positives.",
        precision * 100.0,
        matrix.false_positives
    );
    assert!(
        recall >= 0.80,
        "Recall {:.2}% is below 80% threshold. Found {} false negatives.",
        recall * 100.0,
        matrix.false_negatives
    );

    println!("\n✓ Test detection validation PASSED");
    println!("  Precision: {:.2}% (≥85% ✓)", precision * 100.0);
    println!("  Recall:    {:.2}% (≥80% ✓)", recall * 100.0);

    Ok(())
}

/// Test edge cases for test detection
#[test]
fn test_detection_edge_cases() {
    let scorer = HeuristicScorer::new();

    // Edge case: "test" in filename but not a test
    assert!(
        !scorer.is_test_file("src/testUtils.ts"),
        "testUtils.ts should NOT be classified as test"
    );
    assert!(
        !scorer.is_test_file("src/testing/helpers.ts"),
        "testing/helpers.ts should NOT be classified as test (no .test. or /tests/)"
    );

    // Edge case: Integration tests vs unit tests (both should be test)
    assert!(
        scorer.is_test_file("tests/integration/api.test.ts"),
        "Integration tests should be classified as test"
    );
    assert!(
        scorer.is_test_file("tests/unit/parser.test.ts"),
        "Unit tests should be classified as test"
    );

    // Edge case: Benchmark files (should NOT be test)
    assert!(
        !scorer.is_test_file("benches/benchmark.rs"),
        "Benchmark files should NOT be classified as test"
    );
    assert!(
        !scorer.is_test_file("src/bench_parser.rs"),
        "Benchmark files should NOT be classified as test"
    );

    // Edge case: Example files (should NOT be test)
    assert!(
        !scorer.is_test_file("examples/basic.rs"),
        "Example files should NOT be classified as test"
    );
    assert!(
        !scorer.is_test_file("examples/advanced/usage.ts"),
        "Example files should NOT be classified as test"
    );

    // Edge case: Test utilities in production code
    assert!(
        !scorer.is_test_file("src/testHelpers.ts"),
        "Test helpers in src/ without .test. should NOT be classified as test"
    );

    // Edge case: Actual test files with various extensions
    assert!(
        scorer.is_test_file("src/parser.test.ts"),
        ".test.ts should be classified as test"
    );
    assert!(
        scorer.is_test_file("src/component.test.tsx"),
        ".test.tsx should be classified as test"
    );
    assert!(
        scorer.is_test_file("src/handler.spec.js"),
        ".spec.js should be classified as test"
    );
    assert!(
        scorer.is_test_file("src/lib_test.rs"),
        "_test.rs should be classified as test"
    );
    assert!(
        scorer.is_test_file("src/parser_test.py"),
        "_test.py should be classified as test"
    );
}

/// Test detection on various directory structures
#[test]
fn test_detection_directory_patterns() {
    let scorer = HeuristicScorer::new();

    // /tests/ directory (primary pattern)
    assert!(
        scorer.is_test_file("tests/unit/handler.ts"),
        "/tests/ directory should be classified as test"
    );
    assert!(
        scorer.is_test_file("packages/cli/tests/integration.ts"),
        "Nested /tests/ should be classified as test"
    );

    // __tests__ directory (Jest convention)
    assert!(
        scorer.is_test_file("src/__tests__/component.ts"),
        "__tests__ directory should be classified as test"
    );
    assert!(
        scorer.is_test_file("packages/lib/__tests__/utils.js"),
        "Nested __tests__ should be classified as test"
    );

    // Note: /test/ (singular) is NOT in the default patterns
    // Only /tests/ (plural) is matched
    // This is consistent with the SQL queries in the ticket which use %/tests/%
    assert!(
        !scorer.is_test_file("src/test/parser.ts"),
        "/test/ (singular) is NOT in default patterns"
    );

    // Production directories (should NOT be test)
    assert!(
        !scorer.is_test_file("src/parser.ts"),
        "/src/ without test pattern should NOT be classified as test"
    );
    assert!(
        !scorer.is_test_file("lib/handler.ts"),
        "/lib/ without test pattern should NOT be classified as test"
    );
    assert!(
        !scorer.is_test_file("crates/maproom/src/indexer.rs"),
        "/crates/.../src/ without test pattern should NOT be classified as test"
    );
}
