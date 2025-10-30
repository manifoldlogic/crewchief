//! Evaluation metrics for search quality assessment.
//!
//! This module implements standard information retrieval metrics:
//! - Precision@K: Proportion of relevant results in top-K
//! - Recall@K: Proportion of all relevant results found in top-K
//! - NDCG@K: Normalized Discounted Cumulative Gain (graded relevance)
//! - MRR: Mean Reciprocal Rank

use std::collections::HashMap;

/// Represents a search result with relevance information
#[derive(Debug, Clone)]
pub struct RankedResult {
    /// Unique identifier for the result (e.g., chunk_id)
    pub id: i64,
    /// Whether this result is relevant (for binary relevance)
    pub relevant: bool,
    /// Graded relevance score (0-3): 0=not relevant, 1=somewhat, 2=relevant, 3=highly relevant
    pub relevance_grade: u8,
}

/// Ground truth result with expected relevance
#[derive(Debug, Clone)]
pub struct GroundTruthResult {
    /// Chunk ID or result identifier
    pub chunk_id: i64,
    /// File path for this result
    pub file_path: String,
    /// Symbol name
    pub symbol: String,
    /// Relevance score: 0-3 (0=not relevant, 1=somewhat, 2=relevant, 3=highly relevant)
    pub relevance: u8,
    /// Rationale for the relevance score
    pub rationale: String,
}

/// Collection of evaluation metrics for a query or query set
#[derive(Debug, Clone)]
pub struct EvaluationMetrics {
    /// Precision at various K values
    pub precision_at_k: HashMap<usize, f64>,
    /// Recall at various K values
    pub recall_at_k: HashMap<usize, f64>,
    /// NDCG at various K values
    pub ndcg_at_k: HashMap<usize, f64>,
    /// Mean Reciprocal Rank
    pub mrr: f64,
}

/// Calculate Precision@K
///
/// Precision@K measures the proportion of relevant results in the top K results.
///
/// Formula: `precision@K = (number of relevant results in top K) / K`
///
/// # Arguments
/// * `results` - Ranked search results with relevance information
/// * `k` - Number of top results to consider
///
/// # Returns
/// Precision value between 0.0 and 1.0
///
/// # Example
/// ```
/// use crewchief_maproom::evaluation::{RankedResult, calculate_precision_at_k};
///
/// let results = vec![
///     RankedResult { id: 1, relevant: true, relevance_grade: 3 },
///     RankedResult { id: 2, relevant: true, relevance_grade: 2 },
///     RankedResult { id: 3, relevant: false, relevance_grade: 0 },
///     RankedResult { id: 4, relevant: true, relevance_grade: 2 },
/// ];
///
/// let precision = calculate_precision_at_k(&results, 3);
/// assert_eq!(precision, 2.0 / 3.0); // 2 relevant out of 3
/// ```
pub fn calculate_precision_at_k(results: &[RankedResult], k: usize) -> f64 {
    if k == 0 {
        return 0.0;
    }

    let top_k = results.iter().take(k);
    let relevant_count = top_k.filter(|r| r.relevant).count();

    relevant_count as f64 / k as f64
}

/// Calculate Recall@K
///
/// Recall@K measures the proportion of all relevant results that appear in the top K.
///
/// Formula: `recall@K = (number of relevant results in top K) / (total number of relevant results)`
///
/// # Arguments
/// * `results` - Ranked search results with relevance information
/// * `k` - Number of top results to consider
/// * `total_relevant` - Total number of relevant results for this query
///
/// # Returns
/// Recall value between 0.0 and 1.0
///
/// # Example
/// ```
/// use crewchief_maproom::evaluation::{RankedResult, calculate_recall_at_k};
///
/// let results = vec![
///     RankedResult { id: 1, relevant: true, relevance_grade: 3 },
///     RankedResult { id: 2, relevant: false, relevance_grade: 0 },
///     RankedResult { id: 3, relevant: true, relevance_grade: 2 },
/// ];
///
/// let recall = calculate_recall_at_k(&results, 3, 5); // 5 total relevant results exist
/// assert_eq!(recall, 2.0 / 5.0); // Found 2 out of 5 relevant
/// ```
pub fn calculate_recall_at_k(results: &[RankedResult], k: usize, total_relevant: usize) -> f64 {
    if total_relevant == 0 {
        return 0.0;
    }

    let top_k = results.iter().take(k);
    let relevant_count = top_k.filter(|r| r.relevant).count();

    relevant_count as f64 / total_relevant as f64
}

/// Calculate NDCG@K (Normalized Discounted Cumulative Gain)
///
/// NDCG@K measures ranking quality with graded relevance, discounting lower-ranked results.
/// Uses logarithmic discount and graded relevance scores (0-3).
///
/// Formula:
/// - `DCG@K = sum(rel_i / log2(i + 1))` for i=1 to K
/// - `IDCG@K = DCG of ideal ranking (all relevant results sorted by relevance)`
/// - `NDCG@K = DCG@K / IDCG@K`
///
/// # Arguments
/// * `results` - Ranked search results with graded relevance (0-3)
/// * `k` - Number of top results to consider
///
/// # Returns
/// NDCG value between 0.0 and 1.0 (1.0 = perfect ranking)
///
/// # Example
/// ```
/// use crewchief_maproom::evaluation::{RankedResult, calculate_ndcg_at_k};
///
/// let results = vec![
///     RankedResult { id: 1, relevant: true, relevance_grade: 3 }, // Highly relevant
///     RankedResult { id: 2, relevant: true, relevance_grade: 2 }, // Relevant
///     RankedResult { id: 3, relevant: false, relevance_grade: 0 }, // Not relevant
/// ];
///
/// let ndcg = calculate_ndcg_at_k(&results, 3);
/// assert!(ndcg > 0.0 && ndcg <= 1.0);
/// ```
pub fn calculate_ndcg_at_k(results: &[RankedResult], k: usize) -> f64 {
    if k == 0 {
        return 0.0;
    }

    // Calculate DCG@K for actual results
    let dcg = results
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, result)| {
            let relevance = result.relevance_grade as f64;
            let position = (i + 2) as f64; // i+2 because log2(1) = 0, so we start from position 2
            relevance / position.log2()
        })
        .sum::<f64>();

    // Calculate IDCG@K (ideal ranking)
    let mut ideal_results: Vec<_> = results.to_vec();
    ideal_results.sort_by(|a, b| b.relevance_grade.cmp(&a.relevance_grade));

    let idcg = ideal_results
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, result)| {
            let relevance = result.relevance_grade as f64;
            let position = (i + 2) as f64;
            relevance / position.log2()
        })
        .sum::<f64>();

    // Return NDCG
    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

/// Calculate MRR (Mean Reciprocal Rank)
///
/// MRR is the average of reciprocal ranks of the first relevant result.
/// For a single query, it's simply 1/rank of first relevant result.
///
/// Formula: `MRR = 1 / rank_of_first_relevant_result`
///
/// # Arguments
/// * `results` - Ranked search results with relevance information
///
/// # Returns
/// MRR value between 0.0 and 1.0 (1.0 = first result is relevant)
///
/// # Example
/// ```
/// use crewchief_maproom::evaluation::{RankedResult, calculate_mrr};
///
/// let results = vec![
///     RankedResult { id: 1, relevant: false, relevance_grade: 0 },
///     RankedResult { id: 2, relevant: false, relevance_grade: 0 },
///     RankedResult { id: 3, relevant: true, relevance_grade: 2 },
/// ];
///
/// let mrr = calculate_mrr(&results);
/// assert_eq!(mrr, 1.0 / 3.0); // First relevant at position 3
/// ```
pub fn calculate_mrr(results: &[RankedResult]) -> f64 {
    for (i, result) in results.iter().enumerate() {
        if result.relevant {
            return 1.0 / (i + 1) as f64;
        }
    }
    0.0 // No relevant results found
}

/// Calculate all standard evaluation metrics for a query
///
/// Computes precision@K, recall@K, NDCG@K for specified K values and MRR.
///
/// # Arguments
/// * `results` - Ranked search results with relevance information
/// * `total_relevant` - Total number of relevant results for this query
/// * `k_values` - List of K values to compute metrics for (e.g., &[1, 5, 10, 20])
///
/// # Returns
/// `EvaluationMetrics` struct containing all computed metrics
///
/// # Example
/// ```
/// use crewchief_maproom::evaluation::{RankedResult, calculate_all_metrics};
///
/// let results = vec![
///     RankedResult { id: 1, relevant: true, relevance_grade: 3 },
///     RankedResult { id: 2, relevant: true, relevance_grade: 2 },
///     RankedResult { id: 3, relevant: false, relevance_grade: 0 },
/// ];
///
/// let metrics = calculate_all_metrics(&results, 5, &[1, 3, 5]);
/// assert_eq!(metrics.precision_at_k[&1], 1.0); // First result is relevant
/// assert!(metrics.mrr > 0.0);
/// ```
pub fn calculate_all_metrics(
    results: &[RankedResult],
    total_relevant: usize,
    k_values: &[usize],
) -> EvaluationMetrics {
    let mut precision_at_k = HashMap::new();
    let mut recall_at_k = HashMap::new();
    let mut ndcg_at_k = HashMap::new();

    for &k in k_values {
        precision_at_k.insert(k, calculate_precision_at_k(results, k));
        recall_at_k.insert(k, calculate_recall_at_k(results, k, total_relevant));
        ndcg_at_k.insert(k, calculate_ndcg_at_k(results, k));
    }

    let mrr = calculate_mrr(results);

    EvaluationMetrics {
        precision_at_k,
        recall_at_k,
        ndcg_at_k,
        mrr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_results() -> Vec<RankedResult> {
        vec![
            RankedResult {
                id: 1,
                relevant: true,
                relevance_grade: 3,
            },
            RankedResult {
                id: 2,
                relevant: true,
                relevance_grade: 2,
            },
            RankedResult {
                id: 3,
                relevant: false,
                relevance_grade: 0,
            },
            RankedResult {
                id: 4,
                relevant: true,
                relevance_grade: 2,
            },
            RankedResult {
                id: 5,
                relevant: false,
                relevance_grade: 0,
            },
        ]
    }

    #[test]
    fn test_precision_at_k() {
        let results = create_test_results();

        // Top 1: 1 relevant out of 1 = 1.0
        assert_eq!(calculate_precision_at_k(&results, 1), 1.0);

        // Top 3: 2 relevant out of 3 = 0.666...
        let p3 = calculate_precision_at_k(&results, 3);
        assert!((p3 - 2.0 / 3.0).abs() < 1e-10);

        // Top 5: 3 relevant out of 5 = 0.6
        assert_eq!(calculate_precision_at_k(&results, 5), 0.6);

        // K=0 edge case
        assert_eq!(calculate_precision_at_k(&results, 0), 0.0);
    }

    #[test]
    fn test_recall_at_k() {
        let results = create_test_results();
        let total_relevant = 5; // Assume 5 total relevant results exist

        // Top 1: 1 relevant out of 5 total = 0.2
        assert_eq!(calculate_recall_at_k(&results, 1, total_relevant), 0.2);

        // Top 3: 2 relevant out of 5 total = 0.4
        assert_eq!(calculate_recall_at_k(&results, 3, total_relevant), 0.4);

        // Top 5: 3 relevant out of 5 total = 0.6
        assert_eq!(calculate_recall_at_k(&results, 5, total_relevant), 0.6);

        // Edge case: no relevant results
        assert_eq!(calculate_recall_at_k(&results, 3, 0), 0.0);
    }

    #[test]
    fn test_ndcg_at_k() {
        let results = create_test_results();

        // NDCG should be between 0 and 1
        let ndcg_3 = calculate_ndcg_at_k(&results, 3);
        assert!(ndcg_3 >= 0.0 && ndcg_3 <= 1.0);

        let ndcg_5 = calculate_ndcg_at_k(&results, 5);
        assert!(ndcg_5 >= 0.0 && ndcg_5 <= 1.0);

        // Perfect ranking should give NDCG = 1.0
        let perfect_results = vec![
            RankedResult {
                id: 1,
                relevant: true,
                relevance_grade: 3,
            },
            RankedResult {
                id: 2,
                relevant: true,
                relevance_grade: 2,
            },
            RankedResult {
                id: 3,
                relevant: true,
                relevance_grade: 1,
            },
        ];
        let perfect_ndcg = calculate_ndcg_at_k(&perfect_results, 3);
        assert!((perfect_ndcg - 1.0).abs() < 1e-10);

        // K=0 edge case
        assert_eq!(calculate_ndcg_at_k(&results, 0), 0.0);
    }

    #[test]
    fn test_mrr() {
        let results = create_test_results();

        // First result is relevant, so MRR = 1.0
        assert_eq!(calculate_mrr(&results), 1.0);

        // Test with first relevant at position 3
        let results2 = vec![
            RankedResult {
                id: 1,
                relevant: false,
                relevance_grade: 0,
            },
            RankedResult {
                id: 2,
                relevant: false,
                relevance_grade: 0,
            },
            RankedResult {
                id: 3,
                relevant: true,
                relevance_grade: 2,
            },
        ];
        let mrr = calculate_mrr(&results2);
        assert!((mrr - 1.0 / 3.0).abs() < 1e-10);

        // No relevant results
        let no_relevant = vec![RankedResult {
            id: 1,
            relevant: false,
            relevance_grade: 0,
        }];
        assert_eq!(calculate_mrr(&no_relevant), 0.0);
    }

    #[test]
    fn test_calculate_all_metrics() {
        let results = create_test_results();
        let k_values = vec![1, 3, 5];
        let metrics = calculate_all_metrics(&results, 5, &k_values);

        // Check that all K values are present
        assert!(metrics.precision_at_k.contains_key(&1));
        assert!(metrics.precision_at_k.contains_key(&3));
        assert!(metrics.precision_at_k.contains_key(&5));

        assert!(metrics.recall_at_k.contains_key(&1));
        assert!(metrics.recall_at_k.contains_key(&3));
        assert!(metrics.recall_at_k.contains_key(&5));

        assert!(metrics.ndcg_at_k.contains_key(&1));
        assert!(metrics.ndcg_at_k.contains_key(&3));
        assert!(metrics.ndcg_at_k.contains_key(&5));

        // MRR should be calculated
        assert!(metrics.mrr > 0.0);

        // Verify specific values
        assert_eq!(metrics.precision_at_k[&1], 1.0);
        assert_eq!(metrics.mrr, 1.0);
    }

    #[test]
    fn test_edge_cases() {
        // Empty results
        let empty: Vec<RankedResult> = vec![];
        assert_eq!(calculate_precision_at_k(&empty, 10), 0.0);
        assert_eq!(calculate_recall_at_k(&empty, 10, 5), 0.0);
        assert_eq!(calculate_ndcg_at_k(&empty, 10), 0.0);
        assert_eq!(calculate_mrr(&empty), 0.0);

        // All irrelevant results
        let all_irrelevant = vec![
            RankedResult {
                id: 1,
                relevant: false,
                relevance_grade: 0,
            },
            RankedResult {
                id: 2,
                relevant: false,
                relevance_grade: 0,
            },
        ];
        assert_eq!(calculate_precision_at_k(&all_irrelevant, 2), 0.0);
        assert_eq!(calculate_ndcg_at_k(&all_irrelevant, 2), 0.0);
        assert_eq!(calculate_mrr(&all_irrelevant), 0.0);
    }
}
