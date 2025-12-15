//! Ranking Quality Evaluation Tests (SRCHREL-2005)
//!
//! This test module provides infrastructure for evaluating the ranking quality
//! improvement from quality-weighted graph scoring vs legacy scoring.
//!
//! # Evaluation Methodology
//!
//! For each of 50 diverse test queries:
//! 1. Run search with legacy mode (quality scoring disabled)
//! 2. Run search with enhanced mode (quality scoring enabled)
//! 3. Compare top 3 results
//! 4. Determine: improved, same, or degraded
//!
//! # Target Metrics
//!
//! - ≥32/50 (64%) queries improved
//! - ≤2/50 (4%) queries degraded
//!
//! # Running Evaluation
//!
//! ```bash
//! # Run evaluation with output
//! cargo test --test ranking_quality_evaluation -- --nocapture --test-threads=1
//! ```
//!
//! # Note on Manual Evaluation
//!
//! This test captures search results for comparison, but determining whether
//! a ranking change is an "improvement" requires human judgment about what
//! constitutes "architecturally important" code.
//!
//! Results should be reviewed by a developer familiar with the codebase to
//! validate the improvement/degradation counts.

/// 50 diverse test queries covering different code patterns and domains.
///
/// Query categories:
/// - API/endpoint patterns (10 queries)
/// - Database operations (8 queries)
/// - Authentication/authorization (6 queries)
/// - Error handling (5 queries)
/// - Configuration (5 queries)
/// - Parsing/transformation (5 queries)
/// - Testing patterns (5 queries)
/// - Infrastructure/CLI (6 queries)
pub fn get_evaluation_queries() -> Vec<(&'static str, &'static str)> {
    vec![
        // === API/Endpoint Patterns (10) ===
        ("api", "search handler"),
        ("api", "request handler"),
        ("api", "response builder"),
        ("api", "middleware"),
        ("api", "route definition"),
        ("api", "context builder"),
        ("api", "status endpoint"),
        ("api", "health check"),
        ("api", "json rpc"),
        ("api", "daemon server"),
        // === Database Operations (8) ===
        ("db", "database connection"),
        ("db", "query builder"),
        ("db", "upsert chunk"),
        ("db", "sqlite store"),
        ("db", "migration"),
        ("db", "transaction"),
        ("db", "insert edge"),
        ("db", "graph traversal"),
        // === Authentication/Authorization (6) ===
        ("auth", "authentication"),
        ("auth", "authorization"),
        ("auth", "token validation"),
        ("auth", "credentials"),
        ("auth", "secret storage"),
        ("auth", "permission check"),
        // === Error Handling (5) ===
        ("error", "error handler"),
        ("error", "error type"),
        ("error", "result type"),
        ("error", "fallback"),
        ("error", "recovery"),
        // === Configuration (5) ===
        ("config", "configuration"),
        ("config", "feature flag"),
        ("config", "settings"),
        ("config", "environment variable"),
        ("config", "defaults"),
        // === Parsing/Transformation (5) ===
        ("parse", "parser"),
        ("parse", "tree sitter"),
        ("parse", "chunk extraction"),
        ("parse", "symbol extraction"),
        ("parse", "docstring"),
        // === Testing Patterns (5) ===
        ("test", "test helper"),
        ("test", "mock"),
        ("test", "fixture"),
        ("test", "assertion"),
        ("test", "integration test"),
        // === Infrastructure/CLI (6) ===
        ("infra", "cli command"),
        ("infra", "worker"),
        ("infra", "indexer"),
        ("infra", "scanner"),
        ("infra", "watcher"),
        ("infra", "embedding provider"),
    ]
}

/// Result of comparing rankings for a single query.
#[derive(Debug, Clone)]
pub struct RankingComparison {
    pub query_category: String,
    pub query: String,
    pub legacy_top3: Vec<String>,
    pub enhanced_top3: Vec<String>,
    pub assessment: RankingAssessment,
    pub notes: String,
}

/// Assessment of ranking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingAssessment {
    /// Central code moved up in ranking
    Improved,
    /// Rankings unchanged or already optimal
    Same,
    /// Central code moved down in ranking
    Degraded,
    /// Requires manual review
    PendingReview,
}

impl std::fmt::Display for RankingAssessment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RankingAssessment::Improved => write!(f, "IMPROVED"),
            RankingAssessment::Same => write!(f, "SAME"),
            RankingAssessment::Degraded => write!(f, "DEGRADED"),
            RankingAssessment::PendingReview => write!(f, "PENDING"),
        }
    }
}

/// Summary statistics for evaluation.
#[derive(Debug, Clone)]
pub struct EvaluationSummary {
    pub total_queries: usize,
    pub improved: usize,
    pub same: usize,
    pub degraded: usize,
    pub pending: usize,
    pub improvement_rate: f64,
    pub degradation_rate: f64,
    pub meets_improvement_target: bool,
    pub meets_degradation_target: bool,
}

impl EvaluationSummary {
    pub fn from_comparisons(comparisons: &[RankingComparison]) -> Self {
        let total = comparisons.len();
        let improved = comparisons
            .iter()
            .filter(|c| c.assessment == RankingAssessment::Improved)
            .count();
        let same = comparisons
            .iter()
            .filter(|c| c.assessment == RankingAssessment::Same)
            .count();
        let degraded = comparisons
            .iter()
            .filter(|c| c.assessment == RankingAssessment::Degraded)
            .count();
        let pending = comparisons
            .iter()
            .filter(|c| c.assessment == RankingAssessment::PendingReview)
            .count();

        let reviewed = total - pending;
        let improvement_rate = if reviewed > 0 {
            improved as f64 / reviewed as f64 * 100.0
        } else {
            0.0
        };
        let degradation_rate = if reviewed > 0 {
            degraded as f64 / reviewed as f64 * 100.0
        } else {
            0.0
        };

        Self {
            total_queries: total,
            improved,
            same,
            degraded,
            pending,
            improvement_rate,
            degradation_rate,
            meets_improvement_target: improvement_rate >= 64.0,
            meets_degradation_target: degradation_rate <= 4.0,
        }
    }

    pub fn print_report(&self) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║        RANKING QUALITY EVALUATION SUMMARY                    ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!("\nTotal queries: {}", self.total_queries);
        println!("Reviewed: {}", self.total_queries - self.pending);
        println!("Pending manual review: {}", self.pending);
        println!("\n=== Results ===");
        println!(
            "  Improved: {} ({:.1}%)",
            self.improved, self.improvement_rate
        );
        println!("  Same: {}", self.same);
        println!(
            "  Degraded: {} ({:.1}%)",
            self.degraded, self.degradation_rate
        );
        println!("\n=== Target Validation ===");
        println!(
            "  Improvement target (≥64%): {} (actual: {:.1}%)",
            if self.meets_improvement_target {
                "✓ PASS"
            } else {
                "✗ FAIL"
            },
            self.improvement_rate
        );
        println!(
            "  Degradation target (≤4%): {} (actual: {:.1}%)",
            if self.meets_degradation_target {
                "✓ PASS"
            } else {
                "✗ FAIL"
            },
            self.degradation_rate
        );

        let overall = self.meets_improvement_target && self.meets_degradation_target;
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!(
            "║  Overall: {}                                       ║",
            if overall {
                "✓ PASS - Quality targets met"
            } else {
                "✗ FAIL - Review required  "
            }
        );
        println!("╚══════════════════════════════════════════════════════════════╝");
    }
}

// ============================================================================
// Test Infrastructure
// ============================================================================

#[test]
fn test_query_coverage() {
    let queries = get_evaluation_queries();

    // Verify we have exactly 50 queries
    assert_eq!(
        queries.len(),
        50,
        "Should have exactly 50 evaluation queries"
    );

    // Verify category distribution
    let categories: Vec<_> = queries.iter().map(|(cat, _)| *cat).collect();

    // Check each category has queries
    let expected_categories = vec![
        "api", "db", "auth", "error", "config", "parse", "test", "infra",
    ];
    for cat in &expected_categories {
        let count = categories.iter().filter(|c| *c == cat).count();
        assert!(
            count > 0,
            "Category '{}' should have at least one query",
            cat
        );
    }

    println!("\n=== Query Distribution ===");
    for cat in expected_categories {
        let count = categories.iter().filter(|c| *c == &cat).count();
        println!("  {}: {} queries", cat, count);
    }
}

#[test]
fn test_summary_calculation() {
    // Create sample comparisons
    let comparisons = vec![
        RankingComparison {
            query_category: "api".to_string(),
            query: "test1".to_string(),
            legacy_top3: vec!["a".to_string()],
            enhanced_top3: vec!["a".to_string()],
            assessment: RankingAssessment::Improved,
            notes: "".to_string(),
        },
        RankingComparison {
            query_category: "api".to_string(),
            query: "test2".to_string(),
            legacy_top3: vec!["b".to_string()],
            enhanced_top3: vec!["b".to_string()],
            assessment: RankingAssessment::Same,
            notes: "".to_string(),
        },
        RankingComparison {
            query_category: "api".to_string(),
            query: "test3".to_string(),
            legacy_top3: vec!["c".to_string()],
            enhanced_top3: vec!["c".to_string()],
            assessment: RankingAssessment::Improved,
            notes: "".to_string(),
        },
    ];

    let summary = EvaluationSummary::from_comparisons(&comparisons);

    assert_eq!(summary.total_queries, 3);
    assert_eq!(summary.improved, 2);
    assert_eq!(summary.same, 1);
    assert_eq!(summary.degraded, 0);
    assert!((summary.improvement_rate - 66.67).abs() < 0.1);
}

#[test]
fn test_print_evaluation_queries() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           50 EVALUATION QUERIES (SRCHREL-2005)               ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let queries = get_evaluation_queries();
    let mut current_category = "";

    for (i, (category, query)) in queries.iter().enumerate() {
        if *category != current_category {
            current_category = category;
            println!("\n=== {} ===", category.to_uppercase());
        }
        println!("  {:02}. {}", i + 1, query);
    }

    println!("\nTotal: {} queries", queries.len());
}

// ============================================================================
// Manual Evaluation Template
// ============================================================================

/// Print a template for recording manual evaluation results.
#[test]
fn test_print_evaluation_template() {
    println!("\n");
    println!("# Ranking Quality Evaluation Results (SRCHREL-2005)");
    println!("");
    println!("**Date:** [DATE]");
    println!("**Evaluator:** [NAME]");
    println!("**Database:** ~/.maproom/maproom.db");
    println!("");
    println!("## Methodology");
    println!("");
    println!("For each query:");
    println!("1. Run: `MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=false cargo run --bin crewchief-maproom -- search --repo crewchief --query \"<query>\" --debug`");
    println!("2. Run: `MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=true cargo run --bin crewchief-maproom -- search --repo crewchief --query \"<query>\" --debug`");
    println!("3. Compare top 3 results");
    println!("4. Assess: Did architecturally important code move up (IMPROVED), stay same (SAME), or move down (DEGRADED)?");
    println!("");
    println!("## Results Table");
    println!("");
    println!("| # | Category | Query | Legacy #1 | Enhanced #1 | Assessment | Notes |");
    println!("|---|----------|-------|-----------|-------------|------------|-------|");

    let queries = get_evaluation_queries();
    for (i, (category, query)) in queries.iter().enumerate() {
        println!(
            "| {:02} | {} | {} | [result] | [result] | [I/S/D] | |",
            i + 1,
            category,
            query
        );
    }

    println!("");
    println!("## Summary");
    println!("");
    println!("- **Total queries:** 50");
    println!("- **Improved:** [count] ([percentage]%)");
    println!("- **Same:** [count]");
    println!("- **Degraded:** [count] ([percentage]%)");
    println!("");
    println!("## Target Validation");
    println!("");
    println!("- [ ] ≥32/50 (64%) improved");
    println!("- [ ] ≤2/50 (4%) degraded");
    println!("");
    println!("## Patterns Observed");
    println!("");
    println!("### Improvements");
    println!("- [pattern 1]");
    println!("");
    println!("### Degradations");
    println!("- [pattern 1]");
}
