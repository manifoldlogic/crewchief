//! Dashboard for A/B testing metrics visualization and analysis
//!
//! Provides real-time quality metric calculations, time-series analysis,
//! and side-by-side comparison of old vs new search implementations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::evaluation::metrics::{
    calculate_all_metrics, EvaluationMetrics, GroundTruthResult, RankedResult,
};

/// Dashboard metrics aggregated over time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    /// Experiment ID
    pub experiment_id: String,
    /// Time period start
    pub period_start: DateTime<Utc>,
    /// Time period end
    pub period_end: DateTime<Utc>,
    /// Old implementation metrics
    pub old_metrics: QualityMetrics,
    /// New implementation metrics
    pub new_metrics: QualityMetrics,
    /// Comparison summary
    pub comparison: MetricComparison,
    /// Total queries in this period
    pub total_queries: usize,
    /// Success rate for new implementation (non-timeout, non-error)
    pub new_success_rate: f64,
}

/// Quality metrics for a search implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Recall at k=10
    pub recall_at_10: f64,
    /// Precision at k=10
    pub precision_at_10: f64,
    /// NDCG at k=10
    pub ndcg_at_10: f64,
    /// Mean reciprocal rank
    pub mrr: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// P95 latency in milliseconds
    pub p95_latency_ms: f64,
    /// P99 latency in milliseconds
    pub p99_latency_ms: f64,
}

/// Side-by-side comparison of metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    /// Recall improvement (percentage points)
    pub recall_delta: f64,
    /// Precision improvement (percentage points)
    pub precision_delta: f64,
    /// NDCG improvement (percentage points)
    pub ndcg_delta: f64,
    /// MRR improvement (percentage points)
    pub mrr_delta: f64,
    /// Latency change in milliseconds (negative = faster)
    pub latency_delta_ms: f64,
    /// Statistical significance (p < 0.05)
    pub is_significant: bool,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
}

/// Time-series data point for trending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub recall: f64,
    pub precision: f64,
    pub ndcg: f64,
    pub latency_ms: f64,
    pub query_count: usize,
}

/// User segment breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMetrics {
    pub segment_name: String,
    pub user_count: usize,
    pub query_count: usize,
    pub metrics: QualityMetrics,
}

/// Dashboard data aggregator
pub struct Dashboard {
    /// Database connection string
    _db_url: String,
}

impl Dashboard {
    /// Create new dashboard
    pub fn new(db_url: String) -> Self {
        Self { _db_url: db_url }
    }

    /// Get metrics for an experiment over a time period
    pub async fn get_metrics(
        &self,
        experiment_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        ground_truth: &HashMap<String, Vec<GroundTruthResult>>,
    ) -> anyhow::Result<DashboardMetrics> {
        let shadow_results = self.fetch_shadow_results(experiment_id, start, end).await?;

        let mut old_queries: Vec<QueryResult> = Vec::new();
        let mut new_queries: Vec<QueryResult> = Vec::new();
        let mut new_successes = 0;

        for result in &shadow_results {
            // Old implementation metrics
            if let Some(truth) = ground_truth.get(&result.query) {
                let old_metrics = self.calculate_query_metrics(&result.old_results, truth);
                old_queries.push(QueryResult {
                    _query: result.query.clone(),
                    metrics: old_metrics,
                    latency_ms: result.old_latency_ms as f64,
                });
            }

            // New implementation metrics (if successful)
            if let Some(new_results) = &result.new_results {
                new_successes += 1;
                if let Some(truth) = ground_truth.get(&result.query) {
                    let new_metrics = self.calculate_query_metrics(new_results, truth);
                    new_queries.push(QueryResult {
                        _query: result.query.clone(),
                        metrics: new_metrics,
                        latency_ms: result.new_latency_ms.unwrap_or(0) as f64,
                    });
                }
            }
        }

        let total_queries = shadow_results.len();
        let new_success_rate = if total_queries > 0 {
            new_successes as f64 / total_queries as f64
        } else {
            0.0
        };

        let old_metrics = self.aggregate_metrics(&old_queries);
        let new_metrics = self.aggregate_metrics(&new_queries);
        let comparison = self.compare_metrics(&old_metrics, &new_metrics);

        Ok(DashboardMetrics {
            experiment_id: experiment_id.to_string(),
            period_start: start,
            period_end: end,
            old_metrics,
            new_metrics,
            comparison,
            total_queries,
            new_success_rate,
        })
    }

    /// Get time-series data for trending analysis
    pub async fn get_time_series(
        &self,
        experiment_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        bucket_size_hours: i64,
        ground_truth: &HashMap<String, Vec<GroundTruthResult>>,
    ) -> anyhow::Result<Vec<TimeSeriesPoint>> {
        let shadow_results = self.fetch_shadow_results(experiment_id, start, end).await?;

        // Group by time buckets
        let mut buckets: HashMap<DateTime<Utc>, Vec<ShadowResult>> = HashMap::new();
        for result in shadow_results {
            let bucket_time = self.round_to_bucket(result.timestamp, bucket_size_hours);
            buckets.entry(bucket_time).or_default().push(result);
        }

        let mut time_series: Vec<TimeSeriesPoint> = Vec::new();
        for (timestamp, results) in buckets {
            let mut queries: Vec<QueryResult> = Vec::new();
            let mut total_latency = 0.0;

            for result in &results {
                if let Some(new_results) = &result.new_results {
                    if let Some(truth) = ground_truth.get(&result.query) {
                        let metrics = self.calculate_query_metrics(new_results, truth);
                        queries.push(QueryResult {
                            _query: result.query.clone(),
                            metrics,
                            latency_ms: result.new_latency_ms.unwrap_or(0) as f64,
                        });
                        total_latency += result.new_latency_ms.unwrap_or(0) as f64;
                    }
                }
            }

            let query_count = queries.len();
            if query_count > 0 {
                let agg_metrics = self.aggregate_metrics(&queries);
                time_series.push(TimeSeriesPoint {
                    timestamp,
                    recall: agg_metrics.recall_at_10,
                    precision: agg_metrics.precision_at_10,
                    ndcg: agg_metrics.ndcg_at_10,
                    latency_ms: total_latency / query_count as f64,
                    query_count,
                });
            }
        }

        // Sort by timestamp
        time_series.sort_by_key(|entry| entry.timestamp);

        Ok(time_series)
    }

    /// Get metrics broken down by user segments
    pub async fn get_segment_metrics(
        &self,
        experiment_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        ground_truth: &HashMap<String, Vec<GroundTruthResult>>,
    ) -> anyhow::Result<Vec<SegmentMetrics>> {
        let shadow_results = self.fetch_shadow_results(experiment_id, start, end).await?;

        // Group by user_id
        let mut segments: HashMap<String, Vec<ShadowResult>> = HashMap::new();
        for result in shadow_results {
            let segment_key = result
                .user_id
                .clone()
                .unwrap_or_else(|| "anonymous".to_string());
            segments.entry(segment_key).or_default().push(result);
        }

        let mut segment_metrics: Vec<SegmentMetrics> = Vec::new();
        for (segment_name, results) in segments {
            let mut queries: Vec<QueryResult> = Vec::new();
            let user_count = 1; // In real impl, count unique users in segment

            for result in &results {
                if let Some(new_results) = &result.new_results {
                    if let Some(truth) = ground_truth.get(&result.query) {
                        let metrics = self.calculate_query_metrics(new_results, truth);
                        queries.push(QueryResult {
                            _query: result.query.clone(),
                            metrics,
                            latency_ms: result.new_latency_ms.unwrap_or(0) as f64,
                        });
                    }
                }
            }

            let query_count = queries.len();
            if query_count > 0 {
                let metrics = self.aggregate_metrics(&queries);
                segment_metrics.push(SegmentMetrics {
                    segment_name,
                    user_count,
                    query_count,
                    metrics,
                });
            }
        }

        Ok(segment_metrics)
    }

    /// Export metrics to JSON format
    pub fn export_json(&self, metrics: &DashboardMetrics) -> anyhow::Result<String> {
        let json = serde_json::to_string_pretty(metrics)?;
        Ok(json)
    }

    /// Export metrics to CSV format
    pub fn export_csv(&self, metrics: &DashboardMetrics) -> anyhow::Result<String> {
        let mut csv = String::new();
        csv.push_str("metric,old_value,new_value,delta,improvement_pct\n");

        csv.push_str(&format!(
            "recall@10,{:.4},{:.4},{:.4},{:.2}%\n",
            metrics.old_metrics.recall_at_10,
            metrics.new_metrics.recall_at_10,
            metrics.comparison.recall_delta,
            (metrics.comparison.recall_delta / metrics.old_metrics.recall_at_10) * 100.0
        ));

        csv.push_str(&format!(
            "precision@10,{:.4},{:.4},{:.4},{:.2}%\n",
            metrics.old_metrics.precision_at_10,
            metrics.new_metrics.precision_at_10,
            metrics.comparison.precision_delta,
            (metrics.comparison.precision_delta / metrics.old_metrics.precision_at_10) * 100.0
        ));

        csv.push_str(&format!(
            "ndcg@10,{:.4},{:.4},{:.4},{:.2}%\n",
            metrics.old_metrics.ndcg_at_10,
            metrics.new_metrics.ndcg_at_10,
            metrics.comparison.ndcg_delta,
            (metrics.comparison.ndcg_delta / metrics.old_metrics.ndcg_at_10) * 100.0
        ));

        csv.push_str(&format!(
            "mrr,{:.4},{:.4},{:.4},{:.2}%\n",
            metrics.old_metrics.mrr,
            metrics.new_metrics.mrr,
            metrics.comparison.mrr_delta,
            (metrics.comparison.mrr_delta / metrics.old_metrics.mrr) * 100.0
        ));

        csv.push_str(&format!(
            "avg_latency_ms,{:.2},{:.2},{:.2},{:.2}%\n",
            metrics.old_metrics.avg_latency_ms,
            metrics.new_metrics.avg_latency_ms,
            metrics.comparison.latency_delta_ms,
            (metrics.comparison.latency_delta_ms / metrics.old_metrics.avg_latency_ms) * 100.0
        ));

        Ok(csv)
    }

    // Private helper methods

    async fn fetch_shadow_results(
        &self,
        _experiment_id: &str,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> anyhow::Result<Vec<ShadowResult>> {
        // In real implementation, query PostgreSQL:
        // SELECT * FROM shadow_results WHERE experiment_id = $1 AND timestamp BETWEEN $2 AND $3
        // For now, return empty vec (will be implemented with actual DB connection)
        Ok(Vec::new())
    }

    fn calculate_query_metrics(
        &self,
        results: &[SearchResult],
        ground_truth: &[GroundTruthResult],
    ) -> EvaluationMetrics {
        // Convert search results to RankedResult format
        let ground_truth_map: HashMap<i64, u8> = ground_truth
            .iter()
            .map(|gt| (gt.chunk_id, gt.relevance))
            .collect();

        let ranked_results: Vec<RankedResult> = results
            .iter()
            .map(|r| RankedResult {
                id: r.chunk_id,
                relevant: ground_truth_map
                    .get(&r.chunk_id)
                    .is_some_and(|&rel| rel > 0),
                relevance_grade: ground_truth_map.get(&r.chunk_id).copied().unwrap_or(0),
            })
            .collect();

        let total_relevant = ground_truth.iter().filter(|gt| gt.relevance > 0).count();
        let k_values = vec![10, 20];

        calculate_all_metrics(&ranked_results, total_relevant, &k_values)
    }

    fn aggregate_metrics(&self, queries: &[QueryResult]) -> QualityMetrics {
        if queries.is_empty() {
            return QualityMetrics {
                recall_at_10: 0.0,
                precision_at_10: 0.0,
                ndcg_at_10: 0.0,
                mrr: 0.0,
                avg_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
            };
        }

        let n = queries.len() as f64;
        let mut latencies: Vec<f64> = queries.iter().map(|q| q.latency_ms).collect();
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let recall = queries
            .iter()
            .map(|q| q.metrics.recall_at_k.get(&10).copied().unwrap_or(0.0))
            .sum::<f64>()
            / n;
        let precision = queries
            .iter()
            .map(|q| q.metrics.precision_at_k.get(&10).copied().unwrap_or(0.0))
            .sum::<f64>()
            / n;
        let ndcg = queries
            .iter()
            .map(|q| q.metrics.ndcg_at_k.get(&10).copied().unwrap_or(0.0))
            .sum::<f64>()
            / n;
        let mrr = queries.iter().map(|q| q.metrics.mrr).sum::<f64>() / n;
        let avg_latency = latencies.iter().sum::<f64>() / n;
        let p95_latency = self.percentile(&latencies, 0.95);
        let p99_latency = self.percentile(&latencies, 0.99);

        QualityMetrics {
            recall_at_10: recall,
            precision_at_10: precision,
            ndcg_at_10: ndcg,
            mrr,
            avg_latency_ms: avg_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
        }
    }

    fn compare_metrics(&self, old: &QualityMetrics, new: &QualityMetrics) -> MetricComparison {
        let recall_delta = new.recall_at_10 - old.recall_at_10;
        let precision_delta = new.precision_at_10 - old.precision_at_10;
        let ndcg_delta = new.ndcg_at_10 - old.ndcg_at_10;
        let mrr_delta = new.mrr - old.mrr;
        let latency_delta_ms = new.avg_latency_ms - old.avg_latency_ms;

        // Simple significance test: improvement > 2 percentage points
        let is_significant =
            recall_delta.abs() > 0.02 || precision_delta.abs() > 0.02 || ndcg_delta.abs() > 0.02;

        // Simple confidence based on magnitude of change
        let max_delta = recall_delta
            .abs()
            .max(precision_delta.abs())
            .max(ndcg_delta.abs());
        let confidence = (max_delta * 10.0).min(1.0);

        MetricComparison {
            recall_delta,
            precision_delta,
            ndcg_delta,
            mrr_delta,
            latency_delta_ms,
            is_significant,
            confidence,
        }
    }

    fn percentile(&self, sorted_values: &[f64], p: f64) -> f64 {
        if sorted_values.is_empty() {
            return 0.0;
        }
        let idx = ((sorted_values.len() - 1) as f64 * p).floor() as usize;
        sorted_values[idx]
    }

    fn round_to_bucket(&self, timestamp: DateTime<Utc>, bucket_size_hours: i64) -> DateTime<Utc> {
        let hours_since_epoch = timestamp.timestamp() / 3600;
        let bucket_hours = (hours_since_epoch / bucket_size_hours) * bucket_size_hours;
        DateTime::from_timestamp(bucket_hours * 3600, 0).unwrap_or(timestamp)
    }
}

// Internal types

#[derive(Debug, Clone)]
struct QueryResult {
    _query: String,
    metrics: EvaluationMetrics,
    latency_ms: f64,
}

#[derive(Debug, Clone)]
struct ShadowResult {
    query: String,
    user_id: Option<String>,
    old_results: Vec<SearchResult>,
    new_results: Option<Vec<SearchResult>>,
    old_latency_ms: u64,
    new_latency_ms: Option<u64>,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct SearchResult {
    chunk_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_metrics_creation() {
        let metrics = QualityMetrics {
            recall_at_10: 0.85,
            precision_at_10: 0.75,
            ndcg_at_10: 0.80,
            mrr: 0.90,
            avg_latency_ms: 25.5,
            p95_latency_ms: 45.0,
            p99_latency_ms: 80.0,
        };

        assert_eq!(metrics.recall_at_10, 0.85);
        assert_eq!(metrics.precision_at_10, 0.75);
        assert_eq!(metrics.ndcg_at_10, 0.80);
    }

    #[test]
    fn test_metric_comparison() {
        let dashboard = Dashboard::new("".to_string());

        let old = QualityMetrics {
            recall_at_10: 0.70,
            precision_at_10: 0.65,
            ndcg_at_10: 0.72,
            mrr: 0.75,
            avg_latency_ms: 30.0,
            p95_latency_ms: 50.0,
            p99_latency_ms: 90.0,
        };

        let new = QualityMetrics {
            recall_at_10: 0.85,
            precision_at_10: 0.78,
            ndcg_at_10: 0.82,
            mrr: 0.88,
            avg_latency_ms: 25.0,
            p95_latency_ms: 40.0,
            p99_latency_ms: 75.0,
        };

        let comparison = dashboard.compare_metrics(&old, &new);

        // Use approximate equality for floating point
        assert!((comparison.recall_delta - 0.15).abs() < 1e-10);
        assert!((comparison.precision_delta - 0.13).abs() < 1e-10);
        assert!((comparison.ndcg_delta - 0.10).abs() < 1e-10);
        assert_eq!(comparison.latency_delta_ms, -5.0);
        assert!(comparison.is_significant);
    }

    #[test]
    fn test_percentile_calculation() {
        let dashboard = Dashboard::new("".to_string());
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];

        assert_eq!(dashboard.percentile(&values, 0.5), 50.0);
        // p95 index = (10-1) * 0.95 = 8.55, floor = 8, so values[8] = 90.0
        assert_eq!(dashboard.percentile(&values, 0.95), 90.0);
        // p99 index = (10-1) * 0.99 = 8.91, floor = 8, so values[8] = 90.0
        assert_eq!(dashboard.percentile(&values, 0.99), 90.0);
    }

    #[test]
    fn test_export_json() {
        let dashboard = Dashboard::new("".to_string());

        let metrics = DashboardMetrics {
            experiment_id: "exp-001".to_string(),
            period_start: Utc::now(),
            period_end: Utc::now(),
            old_metrics: QualityMetrics {
                recall_at_10: 0.70,
                precision_at_10: 0.65,
                ndcg_at_10: 0.72,
                mrr: 0.75,
                avg_latency_ms: 30.0,
                p95_latency_ms: 50.0,
                p99_latency_ms: 90.0,
            },
            new_metrics: QualityMetrics {
                recall_at_10: 0.85,
                precision_at_10: 0.78,
                ndcg_at_10: 0.82,
                mrr: 0.88,
                avg_latency_ms: 25.0,
                p95_latency_ms: 40.0,
                p99_latency_ms: 75.0,
            },
            comparison: MetricComparison {
                recall_delta: 0.15,
                precision_delta: 0.13,
                ndcg_delta: 0.10,
                mrr_delta: 0.13,
                latency_delta_ms: -5.0,
                is_significant: true,
                confidence: 1.0,
            },
            total_queries: 100,
            new_success_rate: 0.98,
        };

        let json = dashboard.export_json(&metrics).unwrap();
        assert!(json.contains("exp-001"));
        assert!(json.contains("recall_delta"));
    }

    #[test]
    fn test_export_csv() {
        let dashboard = Dashboard::new("".to_string());

        let metrics = DashboardMetrics {
            experiment_id: "exp-001".to_string(),
            period_start: Utc::now(),
            period_end: Utc::now(),
            old_metrics: QualityMetrics {
                recall_at_10: 0.70,
                precision_at_10: 0.65,
                ndcg_at_10: 0.72,
                mrr: 0.75,
                avg_latency_ms: 30.0,
                p95_latency_ms: 50.0,
                p99_latency_ms: 90.0,
            },
            new_metrics: QualityMetrics {
                recall_at_10: 0.85,
                precision_at_10: 0.78,
                ndcg_at_10: 0.82,
                mrr: 0.88,
                avg_latency_ms: 25.0,
                p95_latency_ms: 40.0,
                p99_latency_ms: 75.0,
            },
            comparison: MetricComparison {
                recall_delta: 0.15,
                precision_delta: 0.13,
                ndcg_delta: 0.10,
                mrr_delta: 0.13,
                latency_delta_ms: -5.0,
                is_significant: true,
                confidence: 1.0,
            },
            total_queries: 100,
            new_success_rate: 0.98,
        };

        let csv = dashboard.export_csv(&metrics).unwrap();
        assert!(csv.contains("metric,old_value,new_value,delta,improvement_pct"));
        assert!(csv.contains("recall@10"));
        assert!(csv.contains("precision@10"));
        assert!(csv.contains("ndcg@10"));
    }

    #[test]
    fn test_aggregate_empty_queries() {
        let dashboard = Dashboard::new("".to_string());
        let queries: Vec<QueryResult> = Vec::new();
        let metrics = dashboard.aggregate_metrics(&queries);

        assert_eq!(metrics.recall_at_10, 0.0);
        assert_eq!(metrics.precision_at_10, 0.0);
        assert_eq!(metrics.ndcg_at_10, 0.0);
    }

    #[test]
    fn test_round_to_bucket() {
        let dashboard = Dashboard::new("".to_string());
        let timestamp = DateTime::from_timestamp(7200, 0).unwrap(); // 2 hours since epoch
        let bucket = dashboard.round_to_bucket(timestamp, 1);
        assert_eq!(bucket.timestamp(), 7200);
    }
}
