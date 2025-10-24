//! Evaluation module for search quality metrics.
//!
//! This module provides evaluation metrics for assessing search quality,
//! including precision, recall, NDCG, and MRR.

pub mod metrics;

pub use metrics::{
    calculate_all_metrics, calculate_mrr, calculate_ndcg_at_k, calculate_precision_at_k,
    calculate_recall_at_k, EvaluationMetrics, GroundTruthResult, RankedResult,
};
