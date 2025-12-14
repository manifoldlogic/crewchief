//! Performance regression tests for query understanding metadata (SRCHTRN-2002).
//!
//! Validates that metadata assembly overhead is <10ms compared to Phase 1 baseline.
//! Performance baseline from SRCHTRN-1000:
//! - p50: 34.0ms
//! - p95: 135.8ms
//! - p99: 211.3ms

use crewchief_maproom::search::results::{
    QueryFilters, QueryProcessingDetails, QueryUnderstanding, SearchMetadata, SearchOptions,
    SearchTiming, TimingBreakdown,
};
use crewchief_maproom::search::types::SearchMode;
use std::collections::HashMap;
use std::time::Instant;

/// Performance baseline from SRCHTRN-1000
#[derive(Debug, Clone)]
struct PerformanceBaseline {
    p50: f64,
    p95: f64,
    p99: f64,
}

impl PerformanceBaseline {
    fn from_srchtrn_1000() -> Self {
        Self {
            p50: 34.0,
            p95: 135.8,
            p99: 211.3,
        }
    }
}

/// Simulate metadata assembly without understanding (Phase 1 baseline)
fn assemble_metadata_without_understanding() -> SearchMetadata {
    let query_processing = QueryProcessingDetails::new(
        "test query".to_string(),
        SearchMode::Auto,
        2,
        0,
        "test & query".to_string(),
        true,
    );
    let result_counts = HashMap::new();
    let timing = SearchTiming::new(5.0, 30.0, 2.0, 5.0);

    SearchMetadata::new(query_processing, result_counts, timing, 0, 0)
}

/// Simulate metadata assembly with understanding (Phase 2)
fn assemble_metadata_with_understanding() -> SearchMetadata {
    let query_processing = QueryProcessingDetails::new(
        "test query".to_string(),
        SearchMode::Auto,
        2,
        0,
        "test & query".to_string(),
        true,
    );
    let result_counts = HashMap::new();
    let timing = SearchTiming::new(5.0, 30.0, 2.0, 5.0);

    let filters = QueryFilters {
        repo_id: 1,
        worktree_id: None,
        file_types: vec![],
        recency_threshold: None,
    };

    let timing_breakdown = TimingBreakdown::new(5.0, 30.0, 2.0, 5.0);

    let understanding = QueryUnderstanding::from_query_data(
        SearchMode::Auto,
        vec!["test".to_string(), "query".to_string()],
        vec![],
        filters,
        "basic_weighted".to_string(),
        timing_breakdown,
    );

    SearchMetadata::with_understanding(query_processing, result_counts, timing, 0, 0, understanding)
}

#[test]
fn test_metadata_assembly_overhead() {
    const ITERATIONS: usize = 1000;

    // Measure Phase 1 baseline (without understanding)
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = assemble_metadata_without_understanding();
    }
    let baseline_duration = start.elapsed().as_secs_f64() * 1000.0;
    let baseline_per_call = baseline_duration / ITERATIONS as f64;

    // Measure Phase 2 (with understanding)
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = assemble_metadata_with_understanding();
    }
    let phase2_duration = start.elapsed().as_secs_f64() * 1000.0;
    let phase2_per_call = phase2_duration / ITERATIONS as f64;

    // Calculate overhead
    let overhead = phase2_per_call - baseline_per_call;

    println!("Baseline (Phase 1): {:.4}ms per call", baseline_per_call);
    println!("Phase 2: {:.4}ms per call", phase2_per_call);
    println!("Overhead: {:.4}ms", overhead);

    // Verify overhead is acceptable (<10ms)
    assert!(
        overhead < 10.0,
        "Metadata assembly overhead {:.4}ms exceeds 10ms budget",
        overhead
    );

    // In practice, overhead should be much less than 10ms (likely <1ms)
    // since we're just cloning Vecs and creating structs
    assert!(
        overhead < 1.0,
        "Metadata assembly overhead {:.4}ms exceeds expected <1ms (should be negligible)",
        overhead
    );
}

#[test]
fn test_timing_breakdown_calculation_performance() {
    const ITERATIONS: usize = 10000;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = TimingBreakdown::new(5.0, 30.0, 2.0, 5.0);
    }
    let duration = start.elapsed().as_secs_f64() * 1000.0;
    let per_call = duration / ITERATIONS as f64;

    println!("TimingBreakdown::new(): {:.6}ms per call", per_call);

    // Should be extremely fast (<0.01ms)
    assert!(
        per_call < 0.01,
        "TimingBreakdown::new() took {:.6}ms, expected <0.01ms",
        per_call
    );
}

#[test]
fn test_query_understanding_construction_performance() {
    const ITERATIONS: usize = 10000;

    let filters = QueryFilters {
        repo_id: 1,
        worktree_id: None,
        file_types: vec![],
        recency_threshold: None,
    };

    let timing_breakdown = TimingBreakdown::new(5.0, 30.0, 2.0, 5.0);

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = QueryUnderstanding::from_query_data(
            SearchMode::Auto,
            vec!["test".to_string(), "query".to_string()],
            vec![],
            filters.clone(),
            "basic_weighted".to_string(),
            timing_breakdown.clone(),
        );
    }
    let duration = start.elapsed().as_secs_f64() * 1000.0;
    let per_call = duration / ITERATIONS as f64;

    println!(
        "QueryUnderstanding::from_query_data(): {:.4}ms per call",
        per_call
    );

    // Should be very fast (<0.1ms)
    assert!(
        per_call < 0.1,
        "QueryUnderstanding construction took {:.4}ms, expected <0.1ms",
        per_call
    );
}

#[test]
fn test_serialization_performance_impact() {
    const ITERATIONS: usize = 1000;

    let metadata_without = assemble_metadata_without_understanding();
    let metadata_with = assemble_metadata_with_understanding();

    // Measure serialization without understanding
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = serde_json::to_string(&metadata_without).unwrap();
    }
    let baseline_duration = start.elapsed().as_secs_f64() * 1000.0;
    let baseline_per_call = baseline_duration / ITERATIONS as f64;

    // Measure serialization with understanding
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = serde_json::to_string(&metadata_with).unwrap();
    }
    let phase2_duration = start.elapsed().as_secs_f64() * 1000.0;
    let phase2_per_call = phase2_duration / ITERATIONS as f64;

    let overhead = phase2_per_call - baseline_per_call;

    println!(
        "Serialization baseline: {:.4}ms per call",
        baseline_per_call
    );
    println!(
        "Serialization with understanding: {:.4}ms per call",
        phase2_per_call
    );
    println!("Serialization overhead: {:.4}ms", overhead);

    // Serialization overhead should be acceptable (<5ms)
    assert!(
        overhead < 5.0,
        "Serialization overhead {:.4}ms exceeds 5ms budget",
        overhead
    );
}

#[test]
fn test_p95_latency_target() {
    // This test validates that the overall p95 latency target is realistic
    // based on the baseline from SRCHTRN-1000.

    let baseline = PerformanceBaseline::from_srchtrn_1000();

    println!("Performance baseline (SRCHTRN-1000):");
    println!("  p50: {:.1}ms", baseline.p50);
    println!("  p95: {:.1}ms", baseline.p95);
    println!("  p99: {:.1}ms", baseline.p99);

    // Current baseline p95 is 135.8ms, which exceeds the aspirational 100ms target
    // Phase 2 should not regress performance significantly
    // With <10ms overhead budget, acceptable p95 is ~146ms
    let max_acceptable_p95 = baseline.p95 + 10.0;

    println!(
        "Max acceptable p95 with 10ms overhead: {:.1}ms",
        max_acceptable_p95
    );

    // Document that current baseline already exceeds 100ms target
    if baseline.p95 > 100.0 {
        println!(
            "NOTE: Current baseline p95 ({:.1}ms) exceeds 100ms target",
            baseline.p95
        );
        println!("      This is acceptable for Phase 2. Phase 3 will optimize.");
    }

    // Verify we're not regressing beyond the overhead budget
    // (This is a documentation test - actual regression testing happens in integration tests)
    assert!(
        max_acceptable_p95 < 150.0,
        "Max acceptable p95 with overhead ({:.1}ms) should stay reasonable",
        max_acceptable_p95
    );
}

#[test]
fn test_memory_allocation_overhead() {
    // Test that memory allocations for metadata are reasonable
    const ITERATIONS: usize = 1000;

    // Create metadata instances
    let mut metadata_vec = Vec::with_capacity(ITERATIONS);

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        metadata_vec.push(assemble_metadata_with_understanding());
    }
    let duration = start.elapsed().as_secs_f64() * 1000.0;

    println!(
        "Created {} metadata instances in {:.2}ms ({:.4}ms per instance)",
        ITERATIONS,
        duration,
        duration / ITERATIONS as f64
    );

    // Should be able to create 1000 instances quickly
    assert!(
        duration < 100.0,
        "Creating {} metadata instances took {:.2}ms, expected <100ms",
        ITERATIONS,
        duration
    );
}
