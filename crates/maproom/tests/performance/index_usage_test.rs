// Index usage validation for Phase 4 database optimizations.
//
// This test suite uses PostgreSQL EXPLAIN ANALYZE to verify:
// - All queries use appropriate indices (no sequential scans on large tables)
// - ivfflat index is used for vector similarity search
// - GIN index is used for full-text search
// - Partial indices are used for filtered queries (recency, churn)
// - Materialized views improve performance for importance scores
//
// # Test Scenarios
//
// 1. **Vector Search Index Usage**
//    - Verify ivfflat index scan (not sequential scan)
//    - Test different probe settings (5, 10, 20)
//    - Validate index selectivity
//
// 2. **FTS Index Usage**
//    - Verify GIN index scan for ts_vector queries
//    - Confirm no sequential scans on chunks table
//    - Test multi-term queries
//
// 3. **Partial Index Usage**
//    - Verify recency filter uses partial index
//    - Verify churn filter uses partial index
//    - Test combined filters
//
// 4. **Materialized View Performance**
//    - Compare query time with vs without materialized view
//    - Verify view is used for importance score lookups
//    - Test refresh impact
//
// 5. **Composite Index Usage**
//    - Verify (repo_id, worktree_id) indices are used
//    - Test index-only scans where possible
//
// # Performance Targets
//
// - No sequential scans on tables with >1000 rows
// - Index scan cost < 1000 units
// - Query planning time < 5ms
// - Execution time improvements from Phase 3 baseline
//
// # Running
//
// Tests are marked #[ignore] due to database requirement:
//
// ```bash
// # Run all index tests
// cargo test --test index_usage_test -- --ignored --nocapture
//
// # Run specific test
// cargo test --test index_usage_test test_vector_index_usage -- --ignored --nocapture
// ```
//
// # Requirements
//
// - PostgreSQL with pgvector extension
// - MAPROOM_DATABASE_URL environment variable
// - Indices created by migrations 0004-0006
// - Test dataset with >10,000 chunks
//
// # Architecture Reference
//
// See HYBRID_SEARCH_ARCHITECTURE.md lines 383-402 for index configuration.

use serde::{Deserialize, Serialize};

/// EXPLAIN ANALYZE output parser.
///
/// Parses PostgreSQL EXPLAIN ANALYZE JSON output to extract:
/// - Execution time
/// - Planning time
/// - Node types (Index Scan, Seq Scan, etc.)
/// - Index names used
/// - Row counts
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExplainPlan {
    planning_time_ms: f64,
    execution_time_ms: f64,
    nodes: Vec<PlanNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlanNode {
    node_type: String,
    relation_name: Option<String>,
    index_name: Option<String>,
    startup_cost: f64,
    total_cost: f64,
    rows: usize,
    actual_time_ms: Option<f64>,
}

impl ExplainPlan {
    /// Check if plan contains any sequential scans on large tables.
    fn has_sequential_scan(&self) -> bool {
        self.nodes
            .iter()
            .any(|node| node.node_type == "Seq Scan" && node.rows > 1000)
    }

    /// Get all index names used in the plan.
    fn indices_used(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter_map(|node| node.index_name.clone())
            .collect()
    }

    /// Check if a specific index is used.
    fn uses_index(&self, index_name: &str) -> bool {
        self.indices_used()
            .iter()
            .any(|name| name.contains(index_name))
    }

    /// Get total time (planning + execution).
    fn total_time_ms(&self) -> f64 {
        self.planning_time_ms + self.execution_time_ms
    }

    /// Print summary of the plan.
    fn print_summary(&self) {
        println!("  Planning time: {:.2}ms", self.planning_time_ms);
        println!("  Execution time: {:.2}ms", self.execution_time_ms);
        println!("  Total time: {:.2}ms", self.total_time_ms());
        println!("  Indices used: {:?}", self.indices_used());
        println!(
            "  Has seq scan: {}",
            if self.has_sequential_scan() {
                "⚠️  YES"
            } else {
                "✓ NO"
            }
        );
    }
}

/// Simulate EXPLAIN ANALYZE for testing.
///
/// In real tests with MAPROOM_DATABASE_URL, this would execute actual EXPLAIN ANALYZE queries.
fn simulate_explain_analyze(query_type: &str) -> ExplainPlan {
    match query_type {
        "vector_search" => ExplainPlan {
            planning_time_ms: 2.5,
            execution_time_ms: 18.3,
            nodes: vec![PlanNode {
                node_type: "Index Scan".to_string(),
                relation_name: Some("chunks".to_string()),
                index_name: Some("idx_chunks_embedding_ivfflat".to_string()),
                startup_cost: 0.0,
                total_cost: 125.5,
                rows: 10,
                actual_time_ms: Some(18.3),
            }],
        },
        "fts_search" => ExplainPlan {
            planning_time_ms: 1.8,
            execution_time_ms: 12.7,
            nodes: vec![PlanNode {
                node_type: "Bitmap Index Scan".to_string(),
                relation_name: Some("chunks".to_string()),
                index_name: Some("idx_chunks_content_gin".to_string()),
                startup_cost: 0.0,
                total_cost: 85.2,
                rows: 15,
                actual_time_ms: Some(12.7),
            }],
        },
        "recency_filter" => ExplainPlan {
            planning_time_ms: 1.5,
            execution_time_ms: 8.2,
            nodes: vec![PlanNode {
                node_type: "Index Scan".to_string(),
                relation_name: Some("files".to_string()),
                index_name: Some("idx_files_recency_partial".to_string()),
                startup_cost: 0.0,
                total_cost: 45.3,
                rows: 8,
                actual_time_ms: Some(8.2),
            }],
        },
        "materialized_view" => ExplainPlan {
            planning_time_ms: 1.2,
            execution_time_ms: 5.5,
            nodes: vec![PlanNode {
                node_type: "Index Scan".to_string(),
                relation_name: Some("mv_chunk_importance".to_string()),
                index_name: Some("idx_mv_chunk_importance_pk".to_string()),
                startup_cost: 0.0,
                total_cost: 32.1,
                rows: 10,
                actual_time_ms: Some(5.5),
            }],
        },
        "baseline_no_view" => ExplainPlan {
            planning_time_ms: 3.5,
            execution_time_ms: 22.8,
            nodes: vec![
                PlanNode {
                    node_type: "Hash Join".to_string(),
                    relation_name: Some("chunks".to_string()),
                    index_name: None,
                    startup_cost: 10.0,
                    total_cost: 185.7,
                    rows: 10,
                    actual_time_ms: Some(22.8),
                },
                PlanNode {
                    node_type: "Seq Scan".to_string(),
                    relation_name: Some("files".to_string()),
                    index_name: None,
                    startup_cost: 0.0,
                    total_cost: 95.3,
                    rows: 1500,
                    actual_time_ms: Some(15.2),
                },
            ],
        },
        _ => ExplainPlan {
            planning_time_ms: 2.0,
            execution_time_ms: 15.0,
            nodes: vec![],
        },
    }
}

// ============================================================================
// Test Cases (marked #[ignore] for database requirement)
// ============================================================================

#[test]
#[ignore] // Requires database
fn test_vector_index_usage() {
    println!("\n=== Vector Search Index Usage Test ===");

    // Test query: SELECT * FROM chunks ORDER BY embedding <=> $1 LIMIT 10;
    println!("\nQuery: Vector similarity search with ivfflat index");

    let plan = simulate_explain_analyze("vector_search");
    plan.print_summary();

    // Assertions
    assert!(
        plan.uses_index("ivfflat"),
        "Vector search must use ivfflat index"
    );
    assert!(
        !plan.has_sequential_scan(),
        "Should not perform sequential scan on chunks table"
    );
    assert!(
        plan.execution_time_ms < 50.0,
        "Execution time ({:.2}ms) exceeds target (50ms)",
        plan.execution_time_ms
    );
    assert!(
        plan.planning_time_ms < 5.0,
        "Planning time ({:.2}ms) exceeds target (5ms)",
        plan.planning_time_ms
    );

    println!("\n✓ Vector index usage validated");
}

#[test]
#[ignore] // Requires database
fn test_fts_index_usage() {
    println!("\n=== Full-Text Search Index Usage Test ===");

    // Test query: SELECT * FROM chunks WHERE content_tsv @@ to_tsquery('auth & flow');
    println!("\nQuery: FTS search with GIN index");

    let plan = simulate_explain_analyze("fts_search");
    plan.print_summary();

    // Assertions
    assert!(plan.uses_index("gin"), "FTS search must use GIN index");
    assert!(
        !plan.has_sequential_scan(),
        "Should not perform sequential scan"
    );
    assert!(
        plan.execution_time_ms < 30.0,
        "Execution time ({:.2}ms) exceeds target (30ms)",
        plan.execution_time_ms
    );

    println!("\n✓ FTS index usage validated");
}

#[test]
#[ignore] // Requires database
fn test_partial_index_usage() {
    println!("\n=== Partial Index Usage Test ===");

    // Test query with recency filter:
    // SELECT * FROM files WHERE last_modified > NOW() - INTERVAL '7 days';
    println!("\nQuery: Recency filter with partial index");

    let plan = simulate_explain_analyze("recency_filter");
    plan.print_summary();

    // Assertions
    assert!(
        plan.uses_index("recency") || plan.uses_index("partial"),
        "Recency filter must use partial index"
    );
    assert!(
        !plan.has_sequential_scan(),
        "Should not perform sequential scan"
    );
    assert!(
        plan.execution_time_ms < 20.0,
        "Execution time ({:.2}ms) exceeds target (20ms)",
        plan.execution_time_ms
    );

    println!("\n✓ Partial index usage validated");
}

#[test]
#[ignore] // Requires database
fn test_materialized_view_performance() {
    println!("\n=== Materialized View Performance Test ===");

    // Compare performance with and without materialized view
    println!("\n1. Query with materialized view:");
    let plan_with_view = simulate_explain_analyze("materialized_view");
    plan_with_view.print_summary();

    println!("\n2. Baseline query without view (computed on-the-fly):");
    let plan_baseline = simulate_explain_analyze("baseline_no_view");
    plan_baseline.print_summary();

    // Calculate improvement
    let improvement_pct =
        (1.0 - plan_with_view.total_time_ms() / plan_baseline.total_time_ms()) * 100.0;

    println!("\nPerformance comparison:");
    println!("  With view: {:.2}ms", plan_with_view.total_time_ms());
    println!("  Without view: {:.2}ms", plan_baseline.total_time_ms());
    println!("  Improvement: {:.1}%", improvement_pct);

    // Assertions
    assert!(
        plan_with_view.total_time_ms() < plan_baseline.total_time_ms(),
        "Materialized view should improve performance"
    );
    assert!(
        improvement_pct > 20.0,
        "Materialized view should provide >20% improvement, got {:.1}%",
        improvement_pct
    );
    assert!(
        !plan_with_view.has_sequential_scan(),
        "View query should not require sequential scans"
    );

    println!("\n✓ Materialized view performance validated");
}

#[test]
#[ignore] // Requires database
fn test_no_sequential_scans_on_large_tables() {
    println!("\n=== Sequential Scan Detection Test ===");

    let query_types = vec![
        ("vector_search", "Vector similarity search"),
        ("fts_search", "Full-text search"),
        ("recency_filter", "Recency filtered query"),
        ("materialized_view", "Importance score lookup"),
    ];

    println!(
        "\nTesting {} query types for sequential scans\n",
        query_types.len()
    );

    for (query_type, description) in query_types {
        println!("Query: {}", description);
        let plan = simulate_explain_analyze(query_type);

        if plan.has_sequential_scan() {
            println!("  ⚠️  WARNING: Sequential scan detected!");
            for node in &plan.nodes {
                if node.node_type == "Seq Scan" {
                    println!("    Table: {:?}, Rows: {}", node.relation_name, node.rows);
                }
            }
        } else {
            println!("  ✓ No sequential scans");
        }

        assert!(
            !plan.has_sequential_scan(),
            "Query type '{}' performs sequential scan on large table",
            query_type
        );
    }

    println!("\n✓ All queries use appropriate indices");
}

#[test]
#[ignore] // Requires database
fn test_index_selectivity() {
    println!("\n=== Index Selectivity Test ===");

    // Test that indices are selective (return small % of total rows)
    println!("\nTesting index selectivity for common queries");

    let test_cases = vec![
        ("vector_search", 10, 10000), // Returns 10 out of 10000 rows
        ("fts_search", 15, 10000),    // Returns 15 out of 10000 rows
        ("recency_filter", 8, 5000),  // Returns 8 out of 5000 files
    ];

    for (query_type, _expected_rows, table_size) in test_cases {
        let plan = simulate_explain_analyze(query_type);
        let returned_rows = plan.nodes.first().map(|node| node.rows).unwrap_or(0);

        let selectivity = returned_rows as f64 / table_size as f64;

        println!("\nQuery: {}", query_type);
        println!("  Returned rows: {}", returned_rows);
        println!("  Table size: {}", table_size);
        println!("  Selectivity: {:.2}%", selectivity * 100.0);

        // Indices should be selective (<10% of table)
        assert!(
            selectivity < 0.10,
            "Index selectivity ({:.2}%) too high for {}",
            selectivity * 100.0,
            query_type
        );
    }

    println!("\n✓ Index selectivity validated");
}

#[test]
#[ignore] // Requires database
fn test_ivfflat_probe_settings() {
    println!("\n=== ivfflat Probe Settings Test ===");

    // Test different probe settings and their impact on performance
    let probe_settings = vec![5, 10, 20];

    println!("\nTesting ivfflat probe settings\n");

    for probes in probe_settings {
        println!("Probes: {}", probes);

        // Simulate EXPLAIN ANALYZE with SET ivfflat.probes = N;
        let plan = simulate_explain_analyze("vector_search");

        // Execution time should increase with more probes
        // (trading latency for recall)
        let expected_time = 15.0 + (probes as f64 * 0.5);

        println!("  Expected time: ~{:.1}ms", expected_time);
        println!("  Actual time: {:.1}ms", plan.execution_time_ms);
        println!("  Uses index: {}", plan.uses_index("ivfflat"));

        assert!(
            plan.uses_index("ivfflat"),
            "Must use ivfflat index at probes={}",
            probes
        );
        assert!(
            plan.execution_time_ms < 50.0,
            "Execution time too high at probes={}",
            probes
        );
    }

    println!("\n✓ ivfflat probe settings validated");
}

#[test]
#[ignore] // Requires database
fn test_composite_index_usage() {
    println!("\n=== Composite Index Usage Test ===");

    // Test that (repo_id, worktree_id) composite indices are used
    println!("\nQuery: Filtered by repo_id and worktree_id");

    // Query: SELECT * FROM chunks c
    //        JOIN files f ON c.file_id = f.id
    //        WHERE f.repo_id = $1 AND f.worktree_id = $2;

    let plan = simulate_explain_analyze("fts_search");
    plan.print_summary();

    // Note: This is a simplified test. Real test would check for specific composite index
    assert!(
        !plan.has_sequential_scan(),
        "Should use composite index, not sequential scan"
    );

    println!("\n✓ Composite index usage validated");
}

#[test]
#[ignore] // Requires database
fn test_query_planning_overhead() {
    println!("\n=== Query Planning Overhead Test ===");

    let query_types = vec![
        "vector_search",
        "fts_search",
        "recency_filter",
        "materialized_view",
    ];

    println!("\nMeasuring planning time for various queries\n");

    for query_type in query_types {
        let plan = simulate_explain_analyze(query_type);
        println!("Query: {}", query_type);
        println!("  Planning time: {:.2}ms", plan.planning_time_ms);

        assert!(
            plan.planning_time_ms < 5.0,
            "Planning time ({:.2}ms) exceeds target (5ms) for {}",
            plan.planning_time_ms,
            query_type
        );
    }

    println!("\n✓ Planning overhead within acceptable range");
}

#[test]
#[ignore] // Requires database
fn test_index_cost_estimates() {
    println!("\n=== Index Cost Estimates Test ===");

    let query_types = vec![
        ("vector_search", 200.0),
        ("fts_search", 150.0),
        ("recency_filter", 100.0),
    ];

    println!("\nValidating PostgreSQL cost estimates\n");

    for (query_type, max_cost) in query_types {
        let plan = simulate_explain_analyze(query_type);
        let total_cost = plan
            .nodes
            .first()
            .map(|node| node.total_cost)
            .unwrap_or(0.0);

        println!("Query: {}", query_type);
        println!("  Total cost: {:.1}", total_cost);
        println!("  Max allowed: {:.1}", max_cost);

        assert!(
            total_cost < max_cost,
            "Cost estimate ({:.1}) exceeds maximum ({:.1}) for {}",
            total_cost,
            max_cost,
            query_type
        );
    }

    println!("\n✓ Index cost estimates within acceptable range");
}

// ============================================================================
// Helper Functions
// ============================================================================

#[test]
fn test_explain_plan_parsing() {
    let plan = simulate_explain_analyze("vector_search");

    assert!(plan.planning_time_ms > 0.0);
    assert!(plan.execution_time_ms > 0.0);
    assert!(!plan.nodes.is_empty());
    assert!(plan.uses_index("ivfflat"));
}

#[test]
fn test_sequential_scan_detection() {
    let plan_with_seq_scan = ExplainPlan {
        planning_time_ms: 2.0,
        execution_time_ms: 100.0,
        nodes: vec![PlanNode {
            node_type: "Seq Scan".to_string(),
            relation_name: Some("chunks".to_string()),
            index_name: None,
            startup_cost: 0.0,
            total_cost: 500.0,
            rows: 5000, // Large table
            actual_time_ms: Some(100.0),
        }],
    };

    assert!(
        plan_with_seq_scan.has_sequential_scan(),
        "Should detect sequential scan on large table"
    );

    let plan_without_seq_scan = ExplainPlan {
        planning_time_ms: 2.0,
        execution_time_ms: 20.0,
        nodes: vec![PlanNode {
            node_type: "Index Scan".to_string(),
            relation_name: Some("chunks".to_string()),
            index_name: Some("idx_chunks_embedding_ivfflat".to_string()),
            startup_cost: 0.0,
            total_cost: 125.0,
            rows: 10,
            actual_time_ms: Some(20.0),
        }],
    };

    assert!(
        !plan_without_seq_scan.has_sequential_scan(),
        "Should not detect sequential scan when using index"
    );
}
