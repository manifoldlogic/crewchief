//! Golden test suite for search quality evaluation.
//!
//! This test loads 100 representative search queries with curated ground truth results,
//! executes them through the search pipeline, and evaluates quality metrics.
//!
//! **Note**: These tests are marked with `#[ignore]` because they require:
//! - A running PostgreSQL database
//! - Indexed codebase data
//! - Embedding service (for vector search)
//!
//! To run these tests:
//! ```bash
//! cargo test --test golden_test -- --ignored
//! ```
//!
//! Or run specific categories:
//! ```bash
//! cargo test --test golden_test test_simple_symbol_queries -- --ignored
//! ```

use crewchief_maproom::evaluation::{
    calculate_all_metrics, EvaluationMetrics, GroundTruthResult, RankedResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Test query from queries.jsonl
#[derive(Debug, Deserialize, Serialize, Clone)]
struct TestQuery {
    id: usize,
    query: String,
    category: String,
    expected_language: Option<String>,
}

/// Ground truth entry from ground_truth.jsonl
#[derive(Debug, Deserialize, Serialize, Clone)]
struct GroundTruthEntry {
    query_id: usize,
    results: Vec<GroundTruthResultJson>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GroundTruthResultJson {
    chunk_id: i64,
    file_path: String,
    symbol: String,
    relevance: u8,
    rationale: String,
}

/// Load all test queries from queries.jsonl
fn load_queries() -> anyhow::Result<Vec<TestQuery>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("queries.jsonl");

    let file = File::open(&path)
        .map_err(|e| anyhow::anyhow!("Failed to open queries.jsonl at {:?}: {}", path, e))?;
    let reader = BufReader::new(file);

    let mut queries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let query: TestQuery = serde_json::from_str(&line)?;
        queries.push(query);
    }

    Ok(queries)
}

/// Load all ground truth data from ground_truth.jsonl
fn load_ground_truth() -> anyhow::Result<HashMap<usize, GroundTruthEntry>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("ground_truth.jsonl");

    let file = File::open(&path)
        .map_err(|e| anyhow::anyhow!("Failed to open ground_truth.jsonl at {:?}: {}", path, e))?;
    let reader = BufReader::new(file);

    let mut ground_truth = HashMap::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: GroundTruthEntry = serde_json::from_str(&line)?;
        ground_truth.insert(entry.query_id, entry);
    }

    Ok(ground_truth)
}

/// Execute a search query and return ranked results
///
/// This is a mock implementation for demonstration.
/// In production, this would call the actual search pipeline.
fn execute_search_query(_query: &str) -> Vec<RankedResult> {
    // Mock implementation: returns empty results
    // In production, this would:
    // 1. Connect to database
    // 2. Execute hybrid search (FTS + vector + metadata signals)
    // 3. Return ranked results with chunk IDs
    vec![]
}

/// Compare search results against ground truth and label relevance
fn label_results_with_ground_truth(
    results: &[i64], // Chunk IDs from search results
    ground_truth: &[GroundTruthResultJson],
) -> Vec<RankedResult> {
    results
        .iter()
        .map(|&chunk_id| {
            // Find this chunk in ground truth
            let relevance = ground_truth
                .iter()
                .find(|gt| gt.chunk_id == chunk_id)
                .map(|gt| gt.relevance)
                .unwrap_or(0);

            RankedResult {
                id: chunk_id,
                relevant: relevance > 0,
                relevance_grade: relevance,
            }
        })
        .collect()
}

/// Evaluate a single query and return metrics
fn evaluate_query(
    query: &TestQuery,
    ground_truth: &GroundTruthEntry,
) -> anyhow::Result<EvaluationMetrics> {
    // Execute search query
    let search_results = execute_search_query(&query.query);

    // Extract chunk IDs from search results
    let result_ids: Vec<i64> = search_results.iter().map(|r| r.id).collect();

    // Label results with ground truth relevance
    let labeled_results = label_results_with_ground_truth(&result_ids, &ground_truth.results);

    // Count total relevant results in ground truth
    let total_relevant = ground_truth
        .results
        .iter()
        .filter(|r| r.relevance > 0)
        .count();

    // Calculate all metrics
    let k_values = vec![1, 5, 10, 20];
    let metrics = calculate_all_metrics(&labeled_results, total_relevant, &k_values);

    Ok(metrics)
}

/// Aggregate metrics across multiple queries
fn aggregate_metrics(all_metrics: &[EvaluationMetrics]) -> EvaluationMetrics {
    if all_metrics.is_empty() {
        return EvaluationMetrics {
            precision_at_k: HashMap::new(),
            recall_at_k: HashMap::new(),
            ndcg_at_k: HashMap::new(),
            mrr: 0.0,
        };
    }

    let k_values = vec![1, 5, 10, 20];
    let mut aggregated = EvaluationMetrics {
        precision_at_k: HashMap::new(),
        recall_at_k: HashMap::new(),
        ndcg_at_k: HashMap::new(),
        mrr: 0.0,
    };

    // Average precision@k across all queries
    for &k in &k_values {
        let avg_precision: f64 = all_metrics
            .iter()
            .map(|m| m.precision_at_k.get(&k).copied().unwrap_or(0.0))
            .sum::<f64>()
            / all_metrics.len() as f64;
        aggregated.precision_at_k.insert(k, avg_precision);

        let avg_recall: f64 = all_metrics
            .iter()
            .map(|m| m.recall_at_k.get(&k).copied().unwrap_or(0.0))
            .sum::<f64>()
            / all_metrics.len() as f64;
        aggregated.recall_at_k.insert(k, avg_recall);

        let avg_ndcg: f64 = all_metrics
            .iter()
            .map(|m| m.ndcg_at_k.get(&k).copied().unwrap_or(0.0))
            .sum::<f64>()
            / all_metrics.len() as f64;
        aggregated.ndcg_at_k.insert(k, avg_ndcg);
    }

    // Average MRR
    aggregated.mrr = all_metrics.iter().map(|m| m.mrr).sum::<f64>() / all_metrics.len() as f64;

    aggregated
}

/// Print evaluation metrics in a readable format
fn print_metrics(metrics: &EvaluationMetrics, label: &str) {
    println!("\n{}", "=".repeat(60));
    println!("{}", label);
    println!("{}", "=".repeat(60));

    println!("\nPrecision@K:");
    for k in [1, 5, 10, 20] {
        if let Some(value) = metrics.precision_at_k.get(&k) {
            println!("  P@{:2}: {:.4}", k, value);
        }
    }

    println!("\nRecall@K:");
    for k in [1, 5, 10, 20] {
        if let Some(value) = metrics.recall_at_k.get(&k) {
            println!("  R@{:2}: {:.4}", k, value);
        }
    }

    println!("\nNDCG@K:");
    for k in [1, 5, 10, 20] {
        if let Some(value) = metrics.ndcg_at_k.get(&k) {
            println!("  NDCG@{:2}: {:.4}", k, value);
        }
    }

    println!("\nMRR: {:.4}", metrics.mrr);
    println!("{}\n", "=".repeat(60));
}

/// Identify queries with poor performance
fn identify_degraded_queries(
    queries: &[TestQuery],
    all_metrics: &[EvaluationMetrics],
    threshold: f64,
) -> Vec<(usize, f64)> {
    queries
        .iter()
        .zip(all_metrics.iter())
        .filter_map(|(query, metrics)| {
            let p10 = metrics.precision_at_k.get(&10).copied().unwrap_or(0.0);
            if p10 < threshold {
                Some((query.id, p10))
            } else {
                None
            }
        })
        .collect()
}

// ============================================================================
// Integration Tests (require database)
// ============================================================================

#[test]
#[ignore = "requires database and indexed data"]
fn test_load_golden_test_data() {
    // Test that we can load the test data files
    let queries = load_queries().expect("Failed to load queries");
    assert_eq!(queries.len(), 100, "Expected 100 test queries");

    let ground_truth = load_ground_truth().expect("Failed to load ground truth");
    assert_eq!(ground_truth.len(), 100, "Expected 100 ground truth entries");

    // Verify all queries have corresponding ground truth
    for query in &queries {
        assert!(
            ground_truth.contains_key(&query.id),
            "Query {} missing ground truth",
            query.id
        );
    }

    println!("✓ Successfully loaded {} test queries", queries.len());
    println!(
        "✓ Successfully loaded {} ground truth entries",
        ground_truth.len()
    );
}

#[test]
#[ignore = "requires database and indexed data"]
fn test_all_queries() {
    let queries = load_queries().expect("Failed to load queries");
    let ground_truth_map = load_ground_truth().expect("Failed to load ground truth");

    let mut all_metrics = Vec::new();

    for query in &queries {
        let ground_truth = ground_truth_map
            .get(&query.id)
            .expect(&format!("Missing ground truth for query {}", query.id));

        let metrics = evaluate_query(query, ground_truth)
            .expect(&format!("Failed to evaluate query {}", query.id));

        all_metrics.push(metrics);
    }

    // Aggregate and print overall metrics
    let aggregated = aggregate_metrics(&all_metrics);
    print_metrics(&aggregated, "Overall Quality Metrics (All 100 Queries)");

    // Identify degraded queries (precision@10 < 0.5)
    let degraded = identify_degraded_queries(&queries, &all_metrics, 0.5);
    if !degraded.is_empty() {
        println!("\n⚠️  Queries with P@10 < 0.5:");
        for (query_id, p10) in degraded {
            println!("  Query {}: P@10 = {:.4}", query_id, p10);
        }
    }

    // Assert quality targets (from ticket requirements)
    let p10 = aggregated.precision_at_k.get(&10).copied().unwrap_or(0.0);
    let ndcg10 = aggregated.ndcg_at_k.get(&10).copied().unwrap_or(0.0);
    let mrr = aggregated.mrr;

    assert!(p10 > 0.7, "Precision@10 ({:.4}) below target (0.7)", p10);
    assert!(ndcg10 > 0.65, "NDCG@10 ({:.4}) below target (0.65)", ndcg10);
    assert!(mrr > 0.8, "MRR ({:.4}) below target (0.8)", mrr);
}

#[test]
#[ignore = "requires database and indexed data"]
fn test_simple_symbol_queries() {
    let queries = load_queries().expect("Failed to load queries");
    let ground_truth_map = load_ground_truth().expect("Failed to load ground truth");

    // Filter to simple_symbol category
    let simple_queries: Vec<_> = queries
        .iter()
        .filter(|q| q.category == "simple_symbol")
        .collect();

    let mut all_metrics = Vec::new();

    for query in &simple_queries {
        let ground_truth = ground_truth_map
            .get(&query.id)
            .expect(&format!("Missing ground truth for query {}", query.id));

        let metrics = evaluate_query(query, ground_truth)
            .expect(&format!("Failed to evaluate query {}", query.id));

        all_metrics.push(metrics);
    }

    let aggregated = aggregate_metrics(&all_metrics);
    print_metrics(
        &aggregated,
        &format!("Simple Symbol Queries ({} queries)", simple_queries.len()),
    );

    // Simple symbol searches should have high precision
    let p10 = aggregated.precision_at_k.get(&10).copied().unwrap_or(0.0);
    assert!(
        p10 > 0.8,
        "Simple symbol P@10 ({:.4}) below expected (0.8)",
        p10
    );
}

#[test]
#[ignore = "requires database and indexed data"]
fn test_semantic_queries() {
    let queries = load_queries().expect("Failed to load queries");
    let ground_truth_map = load_ground_truth().expect("Failed to load ground truth");

    let semantic_queries: Vec<_> = queries
        .iter()
        .filter(|q| q.category == "semantic")
        .collect();

    let mut all_metrics = Vec::new();

    for query in &semantic_queries {
        let ground_truth = ground_truth_map
            .get(&query.id)
            .expect(&format!("Missing ground truth for query {}", query.id));

        let metrics = evaluate_query(query, ground_truth)
            .expect(&format!("Failed to evaluate query {}", query.id));

        all_metrics.push(metrics);
    }

    let aggregated = aggregate_metrics(&all_metrics);
    print_metrics(
        &aggregated,
        &format!("Semantic Queries ({} queries)", semantic_queries.len()),
    );
}

#[test]
#[ignore = "requires database and indexed data"]
fn test_edge_case_queries() {
    let queries = load_queries().expect("Failed to load queries");
    let ground_truth_map = load_ground_truth().expect("Failed to load ground truth");

    let edge_queries: Vec<_> = queries
        .iter()
        .filter(|q| q.category == "edge_case")
        .collect();

    let mut all_metrics = Vec::new();

    for query in &edge_queries {
        let ground_truth = ground_truth_map
            .get(&query.id)
            .expect(&format!("Missing ground truth for query {}", query.id));

        let metrics = evaluate_query(query, ground_truth)
            .expect(&format!("Failed to evaluate query {}", query.id));

        all_metrics.push(metrics);
    }

    let aggregated = aggregate_metrics(&all_metrics);
    print_metrics(
        &aggregated,
        &format!("Edge Case Queries ({} queries)", edge_queries.len()),
    );

    // Edge cases are expected to have lower precision
    println!("Note: Edge cases (single char, very long, ambiguous) may have lower scores");
}

// ============================================================================
// Unit Tests (do not require database)
// ============================================================================

#[test]
fn test_label_results_with_ground_truth() {
    let ground_truth = vec![
        GroundTruthResultJson {
            chunk_id: 101,
            file_path: "src/test.ts".to_string(),
            symbol: "test".to_string(),
            relevance: 3,
            rationale: "Highly relevant".to_string(),
        },
        GroundTruthResultJson {
            chunk_id: 102,
            file_path: "src/foo.ts".to_string(),
            symbol: "foo".to_string(),
            relevance: 1,
            rationale: "Somewhat relevant".to_string(),
        },
    ];

    let result_ids = vec![101, 999, 102]; // 999 not in ground truth

    let labeled = label_results_with_ground_truth(&result_ids, &ground_truth);

    assert_eq!(labeled.len(), 3);
    assert_eq!(labeled[0].id, 101);
    assert_eq!(labeled[0].relevance_grade, 3);
    assert!(labeled[0].relevant);

    assert_eq!(labeled[1].id, 999);
    assert_eq!(labeled[1].relevance_grade, 0);
    assert!(!labeled[1].relevant);

    assert_eq!(labeled[2].id, 102);
    assert_eq!(labeled[2].relevance_grade, 1);
    assert!(labeled[2].relevant);
}

#[test]
fn test_aggregate_metrics() {
    let metrics1 = EvaluationMetrics {
        precision_at_k: [(10, 0.8)].iter().cloned().collect(),
        recall_at_k: [(10, 0.6)].iter().cloned().collect(),
        ndcg_at_k: [(10, 0.75)].iter().cloned().collect(),
        mrr: 0.9,
    };

    let metrics2 = EvaluationMetrics {
        precision_at_k: [(10, 0.6)].iter().cloned().collect(),
        recall_at_k: [(10, 0.4)].iter().cloned().collect(),
        ndcg_at_k: [(10, 0.65)].iter().cloned().collect(),
        mrr: 0.7,
    };

    let aggregated = aggregate_metrics(&[metrics1, metrics2]);

    // Check averaged values
    assert_eq!(aggregated.precision_at_k[&10], 0.7); // (0.8 + 0.6) / 2
    assert_eq!(aggregated.recall_at_k[&10], 0.5); // (0.6 + 0.4) / 2
    assert_eq!(aggregated.ndcg_at_k[&10], 0.7); // (0.75 + 0.65) / 2
    assert_eq!(aggregated.mrr, 0.8); // (0.9 + 0.7) / 2
}

#[test]
fn test_identify_degraded_queries() {
    let queries = vec![
        TestQuery {
            id: 1,
            query: "test1".to_string(),
            category: "test".to_string(),
            expected_language: None,
        },
        TestQuery {
            id: 2,
            query: "test2".to_string(),
            category: "test".to_string(),
            expected_language: None,
        },
    ];

    let metrics = vec![
        EvaluationMetrics {
            precision_at_k: [(10, 0.3)].iter().cloned().collect(), // Below threshold
            recall_at_k: HashMap::new(),
            ndcg_at_k: HashMap::new(),
            mrr: 0.5,
        },
        EvaluationMetrics {
            precision_at_k: [(10, 0.8)].iter().cloned().collect(), // Above threshold
            recall_at_k: HashMap::new(),
            ndcg_at_k: HashMap::new(),
            mrr: 0.9,
        },
    ];

    let degraded = identify_degraded_queries(&queries, &metrics, 0.5);

    assert_eq!(degraded.len(), 1);
    assert_eq!(degraded[0].0, 1); // Query 1 is degraded
    assert_eq!(degraded[0].1, 0.3); // P@10 = 0.3
}
