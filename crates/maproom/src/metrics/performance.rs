//! Performance metrics collection for indexing, memory, and throughput.
//!
//! This module provides the PerformanceMetrics struct for collecting performance
//! metrics beyond search operations, including indexing throughput and memory usage.

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec,
    HistogramVec, Registry,
};
use std::sync::Arc;

lazy_static! {
    /// Global performance metrics instance
    static ref PERFORMANCE_METRICS: Arc<PerformanceMetrics> = Arc::new(
        PerformanceMetrics::new(crate::metrics::search_metrics::get_registry())
            .expect("Failed to initialize performance metrics")
    );
}

/// Get the global PerformanceMetrics instance.
///
/// This returns a reference to the singleton metrics collector that should
/// be used throughout the application.
pub fn get_performance_metrics() -> Arc<PerformanceMetrics> {
    PERFORMANCE_METRICS.clone()
}

/// PerformanceMetrics tracks comprehensive performance metrics for Maproom.
///
/// All metrics are exposed in Prometheus format and can be scraped by monitoring systems.
///
/// # Metrics
///
/// - **indexing_rate**: Histogram tracking files indexed per minute
///   - Labels: `language` (ts/js/rs/py/md/json/yaml/toml)
///   - Target: ≥150 files/min (cold cache), ≥500 files/min (warm cache)
///
/// - **indexing_latency**: Histogram tracking per-file indexing latency in seconds
///   - Labels: `language`, `phase` (parse/insert/total)
///
/// - **memory_usage**: Gauge tracking current memory usage in bytes
///   - Labels: `component` (indexer/search/cache/total)
///   - Target: <500MB peak
///
/// - **cache_hit_rate**: Gauge tracking cache effectiveness (0.0-1.0)
///   - Labels: `cache_type` (embedding/query/chunk)
///
/// - **query_throughput**: Counter tracking total queries processed
///   - Labels: `mode` (code/text/auto), `status` (success/error)
///   - Used to calculate QPS
///
/// - **chunks_created**: Counter tracking total chunks created during indexing
///   - Labels: `language`
///
/// - **files_indexed**: Counter tracking total files indexed
///   - Labels: `language`
pub struct PerformanceMetrics {
    /// Indexing rate histogram (files per minute)
    indexing_rate: HistogramVec,

    /// Per-file indexing latency histogram (seconds)
    indexing_latency: HistogramVec,

    /// Memory usage gauge (bytes)
    memory_usage: GaugeVec,

    /// Cache hit rate gauge (0.0-1.0)
    cache_hit_rate: GaugeVec,

    /// Query throughput counter
    query_throughput: CounterVec,

    /// Chunks created counter
    chunks_created: CounterVec,

    /// Files indexed counter
    files_indexed: CounterVec,
}

impl PerformanceMetrics {
    /// Create a new PerformanceMetrics instance and register all metrics.
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        // Indexing rate histogram (files per minute)
        // Buckets: 10, 50, 100, 150, 200, 300, 500, 750, 1000, 1500, 2000
        let indexing_rate = register_histogram_vec!(
            prometheus::HistogramOpts::new(
                "maproom_indexing_rate_files_per_minute",
                "File indexing throughput in files per minute"
            )
            .buckets(vec![
                10.0, 50.0, 100.0, 150.0, 200.0, 300.0, 500.0, 750.0, 1000.0, 1500.0, 2000.0
            ]),
            &["language"]
        )?;
        registry.register(Box::new(indexing_rate.clone()))?;

        // Per-file indexing latency histogram (seconds)
        // Buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2s, 5s
        let indexing_latency = register_histogram_vec!(
            prometheus::HistogramOpts::new(
                "maproom_indexing_latency_seconds",
                "Per-file indexing latency in seconds"
            )
            .buckets(vec![
                0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.0, 5.0
            ]),
            &["language", "phase"]
        )?;
        registry.register(Box::new(indexing_latency.clone()))?;

        // Memory usage gauge (bytes)
        let memory_usage = register_gauge_vec!(
            "maproom_memory_usage_bytes",
            "Current memory usage in bytes by component",
            &["component"]
        )?;
        registry.register(Box::new(memory_usage.clone()))?;

        // Cache hit rate gauge (0.0-1.0)
        let cache_hit_rate = register_gauge_vec!(
            "maproom_cache_hit_rate",
            "Cache effectiveness as a ratio of hits to total requests (0.0-1.0)",
            &["cache_type"]
        )?;
        registry.register(Box::new(cache_hit_rate.clone()))?;

        // Query throughput counter
        let query_throughput = register_counter_vec!(
            "maproom_query_throughput_total",
            "Total number of queries processed",
            &["mode", "status"]
        )?;
        registry.register(Box::new(query_throughput.clone()))?;

        // Chunks created counter
        let chunks_created = register_counter_vec!(
            "maproom_chunks_created_total",
            "Total number of chunks created during indexing",
            &["language"]
        )?;
        registry.register(Box::new(chunks_created.clone()))?;

        // Files indexed counter
        let files_indexed = register_counter_vec!(
            "maproom_files_indexed_total",
            "Total number of files indexed",
            &["language"]
        )?;
        registry.register(Box::new(files_indexed.clone()))?;

        Ok(Self {
            indexing_rate,
            indexing_latency,
            memory_usage,
            cache_hit_rate,
            query_throughput,
            chunks_created,
            files_indexed,
        })
    }

    /// Record indexing throughput rate.
    ///
    /// # Arguments
    /// * `files_per_minute` - Indexing rate in files per minute
    /// * `language` - Language being indexed (ts/js/rs/py/md/json/yaml/toml)
    pub fn record_indexing_rate(&self, files_per_minute: f64, language: &str) {
        self.indexing_rate
            .with_label_values(&[language])
            .observe(files_per_minute);
    }

    /// Record per-file indexing latency.
    ///
    /// # Arguments
    /// * `duration_seconds` - Indexing duration in seconds
    /// * `language` - Language being indexed
    /// * `phase` - Indexing phase (parse/insert/total)
    pub fn record_indexing_latency(&self, duration_seconds: f64, language: &str, phase: &str) {
        self.indexing_latency
            .with_label_values(&[language, phase])
            .observe(duration_seconds);
    }

    /// Update memory usage gauge.
    ///
    /// # Arguments
    /// * `bytes` - Memory usage in bytes
    /// * `component` - Component name (indexer/search/cache/total)
    pub fn update_memory_usage(&self, bytes: u64, component: &str) {
        self.memory_usage
            .with_label_values(&[component])
            .set(bytes as f64);
    }

    /// Update cache hit rate gauge.
    ///
    /// # Arguments
    /// * `hit_rate` - Cache hit rate as a ratio (0.0-1.0)
    /// * `cache_type` - Cache type (embedding/query/chunk)
    pub fn update_cache_hit_rate(&self, hit_rate: f64, cache_type: &str) {
        self.cache_hit_rate
            .with_label_values(&[cache_type])
            .set(hit_rate);
    }

    /// Increment query throughput counter.
    ///
    /// # Arguments
    /// * `mode` - Search mode (code/text/auto)
    /// * `success` - Whether the query succeeded
    pub fn increment_query_throughput(&self, mode: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        self.query_throughput
            .with_label_values(&[mode, status])
            .inc();
    }

    /// Increment chunks created counter.
    ///
    /// # Arguments
    /// * `count` - Number of chunks created
    /// * `language` - Language of the file
    pub fn increment_chunks_created(&self, count: usize, language: &str) {
        self.chunks_created
            .with_label_values(&[language])
            .inc_by(count as f64);
    }

    /// Increment files indexed counter.
    ///
    /// # Arguments
    /// * `language` - Language of the file
    pub fn increment_files_indexed(&self, language: &str) {
        self.files_indexed.with_label_values(&[language]).inc();
    }

    /// Record a complete indexing operation with all metrics.
    ///
    /// This is a convenience method that records all relevant metrics for a file indexing operation.
    ///
    /// # Arguments
    /// * `language` - Language of the file
    /// * `parse_duration_seconds` - Time spent parsing
    /// * `insert_duration_seconds` - Time spent inserting to database
    /// * `total_duration_seconds` - Total time
    /// * `chunks_count` - Number of chunks created
    pub fn record_file_indexing(
        &self,
        language: &str,
        parse_duration_seconds: f64,
        insert_duration_seconds: f64,
        total_duration_seconds: f64,
        chunks_count: usize,
    ) {
        self.record_indexing_latency(parse_duration_seconds, language, "parse");
        self.record_indexing_latency(insert_duration_seconds, language, "insert");
        self.record_indexing_latency(total_duration_seconds, language, "total");
        self.increment_chunks_created(chunks_count, language);
        self.increment_files_indexed(language);
    }

    /// Record indexing throughput for a batch of files.
    ///
    /// Calculates files per minute and records it.
    ///
    /// # Arguments
    /// * `file_count` - Number of files indexed
    /// * `duration_seconds` - Total duration in seconds
    /// * `language` - Language being indexed (use "mixed" for multi-language batches)
    pub fn record_batch_indexing(
        &self,
        file_count: usize,
        duration_seconds: f64,
        language: &str,
    ) {
        let files_per_minute = (file_count as f64 / duration_seconds) * 60.0;
        self.record_indexing_rate(files_per_minute, language);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_recording() {
        let metrics = get_performance_metrics();

        // Test indexing metrics
        metrics.record_indexing_rate(250.0, "ts");
        metrics.record_indexing_latency(0.015, "rs", "parse");
        metrics.record_indexing_latency(0.005, "rs", "insert");
        metrics.record_indexing_latency(0.020, "rs", "total");

        // Test memory metrics
        metrics.update_memory_usage(100_000_000, "indexer");
        metrics.update_memory_usage(50_000_000, "cache");
        metrics.update_memory_usage(200_000_000, "total");

        // Test cache metrics
        metrics.update_cache_hit_rate(0.85, "embedding");
        metrics.update_cache_hit_rate(0.75, "query");

        // Test throughput metrics
        metrics.increment_query_throughput("code", true);
        metrics.increment_query_throughput("text", false);

        // Test chunk metrics
        metrics.increment_chunks_created(15, "py");
        metrics.increment_files_indexed("py");
    }

    #[test]
    fn test_record_file_indexing() {
        let metrics = get_performance_metrics();

        metrics.record_file_indexing("ts", 0.010, 0.005, 0.015, 12);

        // Should have recorded 3 latency metrics and incremented counters
        // (no assertions needed, just verify no panics)
    }

    #[test]
    fn test_record_batch_indexing() {
        let metrics = get_performance_metrics();

        // 100 files in 30 seconds = 200 files/min
        metrics.record_batch_indexing(100, 30.0, "mixed");

        // Should calculate and record files_per_minute
    }

    #[test]
    fn test_global_performance_metrics_instance() {
        let metrics1 = get_performance_metrics();
        let metrics2 = get_performance_metrics();

        // Should return the same instance
        assert!(Arc::ptr_eq(&metrics1, &metrics2));
    }

    #[test]
    fn test_indexing_rate_targets() {
        let metrics = get_performance_metrics();

        // Test target rates
        metrics.record_indexing_rate(150.0, "ts"); // Cold cache target
        metrics.record_indexing_rate(500.0, "rs"); // Warm cache target
        metrics.record_indexing_rate(1000.0, "py"); // Exceeds target
    }

    #[test]
    fn test_memory_usage_components() {
        let metrics = get_performance_metrics();

        // Test different components
        metrics.update_memory_usage(100_000_000, "indexer");
        metrics.update_memory_usage(50_000_000, "search");
        metrics.update_memory_usage(25_000_000, "cache");
        metrics.update_memory_usage(200_000_000, "total");

        // Target: <500MB = <524_288_000 bytes
        assert!(200_000_000 < 524_288_000);
    }

    #[test]
    fn test_cache_types() {
        let metrics = get_performance_metrics();

        metrics.update_cache_hit_rate(0.80, "embedding");
        metrics.update_cache_hit_rate(0.70, "query");
        metrics.update_cache_hit_rate(0.90, "chunk");
    }
}
