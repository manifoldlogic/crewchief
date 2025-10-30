//! Search metrics collection and tracking.
//!
//! This module provides the SearchMetrics struct for collecting performance
//! and quality metrics from the hybrid search system.

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Gauge, HistogramVec,
    Registry,
};
use std::sync::Arc;

lazy_static! {
    /// Global metrics registry
    static ref METRICS_REGISTRY: Registry = Registry::new();

    /// Global SearchMetrics instance
    static ref SEARCH_METRICS: Arc<SearchMetrics> = Arc::new(
        SearchMetrics::new(&METRICS_REGISTRY)
            .expect("Failed to initialize search metrics")
    );
}

/// Get the global SearchMetrics instance.
///
/// This returns a reference to the singleton metrics collector that should
/// be used throughout the application.
pub fn get_metrics() -> Arc<SearchMetrics> {
    SEARCH_METRICS.clone()
}

/// Get the global Prometheus registry.
///
/// Used by the metrics endpoint to export all metrics.
pub fn get_registry() -> &'static Registry {
    &METRICS_REGISTRY
}

/// SearchMetrics tracks performance and quality metrics for the hybrid search system.
///
/// All metrics are exposed in Prometheus format and can be scraped by monitoring systems.
///
/// # Metrics
///
/// - **query_latency**: Histogram tracking end-to-end search latency in seconds
///   - Labels: `mode` (code/text/auto), `status` (success/error)
///   - Buckets optimized for sub-100ms latencies
///
/// - **fusion_time**: Histogram tracking score fusion computation time in seconds
///   - Labels: `strategy` (basic_weighted/rrf/custom)
///   - Buckets optimized for sub-10ms times
///
/// - **cache_hit_rate**: Gauge tracking cache effectiveness (0.0-1.0)
///   - Updated periodically based on cache statistics
///
/// - **result_count**: Histogram tracking number of results returned per query
///   - Labels: `mode` (code/text/auto)
///   - Buckets: 1, 5, 10, 20, 50, 100
///
/// - **errors_total**: Counter tracking errors by type
///   - Labels: `error_type` (query_processing/search_execution/fusion/database)
///
/// - **queries_total**: Counter tracking total number of queries
///   - Labels: `mode` (code/text/auto), `status` (success/error)
pub struct SearchMetrics {
    /// Query latency histogram (seconds)
    query_latency: HistogramVec,

    /// Fusion computation time histogram (seconds)
    fusion_time: HistogramVec,

    /// Cache hit rate gauge (0.0-1.0)
    cache_hit_rate: Gauge,

    /// Result count histogram
    result_count: HistogramVec,

    /// Error counter by type
    errors_total: CounterVec,

    /// Total queries counter
    queries_total: CounterVec,
}

impl SearchMetrics {
    /// Create a new SearchMetrics instance and register all metrics.
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        // Query latency histogram with buckets optimized for sub-100ms latencies
        // Buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 75ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s
        let query_latency = register_histogram_vec!(
            prometheus::HistogramOpts::new(
                "maproom_search_query_latency_seconds",
                "End-to-end search query latency in seconds"
            )
            .buckets(vec![
                0.001, 0.005, 0.010, 0.025, 0.050, 0.075, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0
            ]),
            &["mode", "status"]
        )?;
        registry.register(Box::new(query_latency.clone()))?;

        // Fusion time histogram with buckets optimized for sub-10ms times
        // Buckets: 0.5ms, 1ms, 2ms, 5ms, 10ms, 25ms, 50ms
        let fusion_time = register_histogram_vec!(
            prometheus::HistogramOpts::new(
                "maproom_search_fusion_time_seconds",
                "Score fusion computation time in seconds"
            )
            .buckets(vec![0.0005, 0.001, 0.002, 0.005, 0.010, 0.025, 0.050]),
            &["strategy"]
        )?;
        registry.register(Box::new(fusion_time.clone()))?;

        // Cache hit rate gauge (0.0-1.0)
        let cache_hit_rate = register_gauge!(
            "maproom_search_cache_hit_rate",
            "Cache effectiveness as a ratio of hits to total requests (0.0-1.0)"
        )?;
        registry.register(Box::new(cache_hit_rate.clone()))?;

        // Result count histogram
        // Buckets: 0, 1, 5, 10, 20, 50, 100
        let result_count = register_histogram_vec!(
            prometheus::HistogramOpts::new(
                "maproom_search_result_count",
                "Number of results returned per search query"
            )
            .buckets(vec![0.0, 1.0, 5.0, 10.0, 20.0, 50.0, 100.0]),
            &["mode"]
        )?;
        registry.register(Box::new(result_count.clone()))?;

        // Error counter by type
        let errors_total = register_counter_vec!(
            "maproom_search_errors_total",
            "Total number of search errors by type",
            &["error_type"]
        )?;
        registry.register(Box::new(errors_total.clone()))?;

        // Total queries counter
        let queries_total = register_counter_vec!(
            "maproom_search_queries_total",
            "Total number of search queries executed",
            &["mode", "status"]
        )?;
        registry.register(Box::new(queries_total.clone()))?;

        Ok(Self {
            query_latency,
            fusion_time,
            cache_hit_rate,
            result_count,
            errors_total,
            queries_total,
        })
    }

    /// Record end-to-end query latency.
    ///
    /// # Arguments
    /// * `duration_seconds` - Query duration in seconds
    /// * `mode` - Search mode (code/text/auto)
    /// * `success` - Whether the query succeeded
    pub fn record_query_latency(&self, duration_seconds: f64, mode: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        self.query_latency
            .with_label_values(&[mode, status])
            .observe(duration_seconds);
    }

    /// Record score fusion computation time.
    ///
    /// # Arguments
    /// * `duration_seconds` - Fusion duration in seconds
    /// * `strategy` - Fusion strategy name (basic_weighted/rrf/custom)
    pub fn record_fusion_time(&self, duration_seconds: f64, strategy: &str) {
        self.fusion_time
            .with_label_values(&[strategy])
            .observe(duration_seconds);
    }

    /// Update cache hit rate gauge.
    ///
    /// # Arguments
    /// * `hit_rate` - Cache hit rate as a ratio (0.0-1.0)
    pub fn update_cache_hit_rate(&self, hit_rate: f64) {
        self.cache_hit_rate.set(hit_rate);
    }

    /// Record number of results returned for a query.
    ///
    /// # Arguments
    /// * `count` - Number of results returned
    /// * `mode` - Search mode (code/text/auto)
    pub fn record_result_count(&self, count: usize, mode: &str) {
        self.result_count
            .with_label_values(&[mode])
            .observe(count as f64);
    }

    /// Increment error counter by type.
    ///
    /// # Arguments
    /// * `error_type` - Type of error (query_processing/search_execution/fusion/database)
    pub fn record_error(&self, error_type: &str) {
        self.errors_total.with_label_values(&[error_type]).inc();
    }

    /// Increment total queries counter.
    ///
    /// # Arguments
    /// * `mode` - Search mode (code/text/auto)
    /// * `success` - Whether the query succeeded
    pub fn increment_queries(&self, mode: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        self.queries_total.with_label_values(&[mode, status]).inc();
    }

    /// Update cache hit rate from cache statistics.
    ///
    /// This is a convenience method to update the cache hit rate gauge
    /// from a CacheStats object.
    ///
    /// # Arguments
    /// * `cache_stats` - Cache statistics containing hits and misses
    pub fn update_cache_hit_rate_from_stats(&self, cache_stats: &crate::search::CacheStats) {
        let hit_rate = cache_stats.hit_rate();
        self.update_cache_hit_rate(hit_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        // Use global metrics instance to avoid registry conflicts
        let metrics = get_metrics();

        // Test that metrics can be recorded without panicking
        metrics.record_query_latency(0.025, "code_test", true);
        metrics.record_fusion_time(0.003, "basic_weighted");
        metrics.update_cache_hit_rate(0.75);
        metrics.record_result_count(10, "text_test");
        metrics.record_error("query_processing");
        metrics.increment_queries("auto_test", true);
    }

    #[test]
    fn test_global_metrics_instance() {
        let metrics1 = get_metrics();
        let metrics2 = get_metrics();

        // Should return the same instance
        assert!(Arc::ptr_eq(&metrics1, &metrics2));
    }

    #[test]
    fn test_cache_hit_rate_bounds() {
        let metrics = get_metrics();

        // Test valid values
        metrics.update_cache_hit_rate(0.0);
        metrics.update_cache_hit_rate(0.5);
        metrics.update_cache_hit_rate(1.0);

        // Note: Prometheus gauges don't enforce bounds, so values outside 0-1 are technically allowed
        // but monitoring/alerting should handle this
    }

    #[test]
    fn test_error_types() {
        let metrics = get_metrics();

        // Test different error types
        metrics.record_error("query_processing_test");
        metrics.record_error("search_execution_test");
        metrics.record_error("fusion_test");
        metrics.record_error("database_test");
    }

    #[test]
    fn test_search_modes() {
        let metrics = get_metrics();

        // Test different search modes with unique labels to avoid conflicts
        metrics.record_query_latency(0.025, "code_mode_test", true);
        metrics.record_query_latency(0.030, "text_mode_test", true);
        metrics.record_query_latency(0.028, "auto_mode_test", true);
        metrics.record_query_latency(0.100, "code_mode_error_test", false);
    }

    #[test]
    fn test_result_count_histogram() {
        let metrics = get_metrics();

        // Test various result counts
        metrics.record_result_count(0, "code_hist_test");
        metrics.record_result_count(1, "text_hist_test");
        metrics.record_result_count(10, "auto_hist_test");
        metrics.record_result_count(50, "code_hist_test2");
        metrics.record_result_count(100, "text_hist_test2");
    }
}
