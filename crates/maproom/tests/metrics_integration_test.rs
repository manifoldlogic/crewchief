//! Integration tests for metrics collection and monitoring.
//!
//! These tests verify that metrics are correctly recorded and exposed
//! through the Prometheus endpoint.

use maproom::metrics::{get_metrics, metrics_handler};
use std::time::Duration;

#[test]
fn test_metrics_singleton() {
    // Verify that get_metrics returns the same instance
    let metrics1 = get_metrics();
    let metrics2 = get_metrics();

    assert!(std::sync::Arc::ptr_eq(&metrics1, &metrics2));
}

#[test]
fn test_query_latency_recording() {
    let metrics = get_metrics();

    // Record some query latencies
    metrics.record_query_latency(0.025, "code", true);
    metrics.record_query_latency(0.030, "text", true);
    metrics.record_query_latency(0.045, "auto", true);
    metrics.record_query_latency(0.100, "code", false); // Error case

    // Verify metrics are exposed (basic smoke test)
    let response = metrics_handler();
    assert!(response.contains("maproom_search_query_latency_seconds"));
    assert!(response.contains("HTTP/1.1 200 OK"));
}

#[test]
fn test_fusion_time_recording() {
    let metrics = get_metrics();

    // Record fusion times
    metrics.record_fusion_time(0.003, "basic_weighted");
    metrics.record_fusion_time(0.005, "rrf");
    metrics.record_fusion_time(0.002, "basic_weighted");

    // Verify metric is exposed
    let response = metrics_handler();
    assert!(response.contains("maproom_search_fusion_time_seconds"));
}

#[test]
fn test_cache_hit_rate_update() {
    let metrics = get_metrics();

    // Update cache hit rate
    metrics.update_cache_hit_rate(0.75);
    metrics.update_cache_hit_rate(0.80);
    metrics.update_cache_hit_rate(0.65);

    // Verify metric is exposed
    let response = metrics_handler();
    assert!(response.contains("maproom_search_cache_hit_rate"));
}

#[test]
fn test_result_count_recording() {
    let metrics = get_metrics();

    // Record result counts
    metrics.record_result_count(10, "code");
    metrics.record_result_count(5, "text");
    metrics.record_result_count(0, "auto");
    metrics.record_result_count(50, "code");

    // Verify metric is exposed
    let response = metrics_handler();
    assert!(response.contains("maproom_search_result_count"));
}

#[test]
fn test_error_recording() {
    let metrics = get_metrics();

    // Record errors of different types
    metrics.record_error("query_processing");
    metrics.record_error("search_execution");
    metrics.record_error("fusion");
    metrics.record_error("database");
    metrics.record_error("query_processing"); // Duplicate

    // Verify metric is exposed
    let response = metrics_handler();
    assert!(response.contains("maproom_search_errors_total"));
}

#[test]
fn test_queries_counter() {
    let metrics = get_metrics();

    // Increment query counter
    metrics.increment_queries("code", true);
    metrics.increment_queries("text", true);
    metrics.increment_queries("auto", true);
    metrics.increment_queries("code", false); // Error case

    // Verify metric is exposed
    let response = metrics_handler();
    assert!(response.contains("maproom_search_queries_total"));
}

#[test]
fn test_metrics_handler_format() {
    // Record some metrics to ensure they appear in the output
    let metrics = get_metrics();
    metrics.record_query_latency(0.025, "test", true);
    metrics.record_fusion_time(0.003, "basic_weighted");
    metrics.update_cache_hit_rate(0.75);
    metrics.record_result_count(10, "test");
    metrics.record_error("query_processing");
    metrics.increment_queries("test", true);

    let response = metrics_handler();

    // Verify HTTP response format
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("Content-Type:"));
    assert!(response.contains("Content-Length:"));

    // Verify metrics are present (may appear with different prefixes due to Prometheus format)
    assert!(
        response.contains("maproom_search")
            || response.contains("# HELP")
            || response.contains("# TYPE")
    );
}

#[test]
fn test_metrics_handler_prometheus_format() {
    // Record some metrics first
    let metrics = get_metrics();
    metrics.record_query_latency(0.025, "test", true);
    metrics.increment_queries("test", true);

    let response = metrics_handler();

    // Extract body (after "\r\n\r\n")
    let body_start = response.find("\r\n\r\n").unwrap() + 4;
    let body = &response[body_start..];

    // Verify Prometheus text format
    // Should contain metric type declarations
    assert!(body.contains("# HELP") || body.contains("# TYPE") || body.contains("maproom_search"));
}

#[test]
fn test_comprehensive_metrics_workflow() {
    let metrics = get_metrics();

    // Simulate a complete search workflow
    let latency = 0.035; // 35ms
    let fusion_time = 0.003; // 3ms
    let result_count = 10;
    let cache_hit_rate = 0.70;
    let mode = "code";

    // Record all metrics
    metrics.record_query_latency(latency, mode, true);
    metrics.record_fusion_time(fusion_time, "basic_weighted");
    metrics.record_result_count(result_count, mode);
    metrics.update_cache_hit_rate(cache_hit_rate);
    metrics.increment_queries(mode, true);

    // Verify all metrics are exposed
    let response = metrics_handler();
    assert!(response.contains("maproom_search_query_latency_seconds"));
    assert!(response.contains("maproom_search_fusion_time_seconds"));
    assert!(response.contains("maproom_search_cache_hit_rate"));
    assert!(response.contains("maproom_search_result_count"));
    assert!(response.contains("maproom_search_queries_total"));
}

#[test]
fn test_metrics_labels() {
    let metrics = get_metrics();

    // Record metrics with different labels
    metrics.record_query_latency(0.025, "code", true);
    metrics.record_query_latency(0.030, "text", true);
    metrics.record_query_latency(0.100, "auto", false);

    let response = metrics_handler();

    // Verify labels are present (format varies, but should contain label info)
    // The exact format depends on Prometheus client implementation
    assert!(response.contains("maproom_search_query_latency_seconds"));
}

#[test]
fn test_histogram_buckets() {
    let metrics = get_metrics();

    // Record values in different histogram buckets
    metrics.record_query_latency(0.001, "code", true); // 1ms
    metrics.record_query_latency(0.010, "code", true); // 10ms
    metrics.record_query_latency(0.050, "code", true); // 50ms
    metrics.record_query_latency(0.100, "code", true); // 100ms
    metrics.record_query_latency(0.500, "code", true); // 500ms

    let response = metrics_handler();

    // Verify histogram metric is present
    assert!(response.contains("maproom_search_query_latency_seconds"));
    // Histogram should have bucket entries
    assert!(
        response.contains("bucket") || response.contains("_sum") || response.contains("_count")
    );
}

#[test]
fn test_error_type_labels() {
    let metrics = get_metrics();

    // Record different error types
    let error_types = vec!["query_processing", "search_execution", "fusion", "database"];

    for error_type in error_types {
        metrics.record_error(error_type);
    }

    let response = metrics_handler();
    assert!(response.contains("maproom_search_errors_total"));
}

#[test]
fn test_metrics_persistence_across_calls() {
    let metrics = get_metrics();

    // Record initial metrics
    metrics.increment_queries("code", true);
    let response1 = metrics_handler();

    // Record more metrics
    metrics.increment_queries("code", true);
    let response2 = metrics_handler();

    // Both responses should contain the metric
    assert!(response1.contains("maproom_search_queries_total"));
    assert!(response2.contains("maproom_search_queries_total"));
}

#[tokio::test]
async fn test_concurrent_metrics_recording() {
    let metrics = get_metrics();

    // Spawn multiple tasks recording metrics concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let metrics_clone = metrics.clone();
        let handle = tokio::spawn(async move {
            let mode = if i % 3 == 0 {
                "code"
            } else if i % 3 == 1 {
                "text"
            } else {
                "auto"
            };

            metrics_clone.record_query_latency(0.025 + (i as f64 * 0.001), mode, true);
            metrics_clone.increment_queries(mode, true);
            tokio::time::sleep(Duration::from_millis(10)).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify metrics are recorded
    let response = metrics_handler();
    assert!(response.contains("maproom_search_query_latency_seconds"));
    assert!(response.contains("maproom_search_queries_total"));
}
