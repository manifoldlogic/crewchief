//! Memory allocation and usage tracking.
//!
//! This module provides detailed memory metrics for monitoring and optimizing
//! memory usage across the Maproom system.
//!
//! # Metrics Tracked
//!
//! - **Allocations**: Total number of allocations
//! - **Deallocations**: Total number of deallocations
//! - **Current Bytes**: Current memory usage
//! - **Peak Bytes**: Maximum memory usage observed
//! - **Allocation Counts by Component**: Per-component tracking
//!
//! # Integration with Performance Metrics
//!
//! This module integrates with the existing PerformanceMetrics system
//! (PERF_OPT-1001) to expose memory metrics via Prometheus.
//!
//! # Performance Target
//!
//! Memory usage <500MB for 100k chunks with optimizations:
//! - String interning
//! - Vector quantization
//! - Buffer pooling
//!
//! # Example
//!
//! ```no_run
//! use maproom::memory::get_memory_metrics;
//!
//! let metrics = get_memory_metrics();
//!
//! // Track an allocation
//! metrics.record_allocation("indexer", 1024);
//!
//! // Track a deallocation
//! metrics.record_deallocation("indexer", 1024);
//!
//! // Get current stats
//! println!("Current usage: {} bytes", metrics.current_bytes());
//! println!("Peak usage: {} bytes", metrics.peak_bytes());
//! println!("Total allocations: {}", metrics.allocation_count());
//! ```

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

lazy_static! {
    /// Global memory metrics instance.
    static ref MEMORY_METRICS: Arc<MemoryMetrics> = Arc::new(MemoryMetrics::new());
}

/// Get the global MemoryMetrics instance.
///
/// Use this for application-wide memory tracking.
pub fn get_memory_metrics() -> Arc<MemoryMetrics> {
    MEMORY_METRICS.clone()
}

/// Memory metrics tracker.
///
/// Tracks allocations, deallocations, and memory usage across components.
/// All operations are thread-safe using atomic operations.
///
/// # Thread Safety
///
/// All counters use atomic operations for lock-free updates.
/// Per-component tracking uses RwLock for thread-safe map access.
pub struct MemoryMetrics {
    /// Total number of allocations
    allocation_count: AtomicUsize,

    /// Total number of deallocations
    deallocation_count: AtomicUsize,

    /// Current memory usage in bytes
    current_bytes: AtomicU64,

    /// Peak memory usage in bytes
    peak_bytes: AtomicU64,

    /// Total bytes allocated (lifetime)
    total_allocated_bytes: AtomicU64,

    /// Total bytes deallocated (lifetime)
    total_deallocated_bytes: AtomicU64,

    /// Per-component allocation counts
    component_allocations: RwLock<HashMap<String, ComponentMetrics>>,
}

impl MemoryMetrics {
    /// Create a new MemoryMetrics instance.
    pub fn new() -> Self {
        Self {
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
            current_bytes: AtomicU64::new(0),
            peak_bytes: AtomicU64::new(0),
            total_allocated_bytes: AtomicU64::new(0),
            total_deallocated_bytes: AtomicU64::new(0),
            component_allocations: RwLock::new(HashMap::new()),
        }
    }

    /// Record an allocation.
    ///
    /// # Arguments
    ///
    /// * `component` - Component name (indexer/search/cache/embedding/etc)
    /// * `bytes` - Number of bytes allocated
    ///
    /// # Example
    ///
    /// ```no_run
    /// use maproom::memory::get_memory_metrics;
    ///
    /// let metrics = get_memory_metrics();
    /// metrics.record_allocation("indexer", 4096);
    /// ```
    pub fn record_allocation(&self, component: &str, bytes: u64) {
        // Update global counters
        self.allocation_count.fetch_add(1, Ordering::Relaxed);
        self.total_allocated_bytes
            .fetch_add(bytes, Ordering::Relaxed);

        let current = self.current_bytes.fetch_add(bytes, Ordering::Relaxed) + bytes;

        // Update peak if needed
        let mut peak = self.peak_bytes.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_bytes.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }

        // Update component metrics
        let mut components = self.component_allocations.write().unwrap();
        let entry = components.entry(component.to_string()).or_default();

        entry.allocations += 1;
        entry.current_bytes += bytes;
        entry.total_allocated_bytes += bytes;
        entry.peak_bytes = entry.peak_bytes.max(entry.current_bytes);
    }

    /// Record a deallocation.
    ///
    /// # Arguments
    ///
    /// * `component` - Component name
    /// * `bytes` - Number of bytes deallocated
    ///
    /// # Example
    ///
    /// ```no_run
    /// use maproom::memory::get_memory_metrics;
    ///
    /// let metrics = get_memory_metrics();
    /// metrics.record_deallocation("indexer", 4096);
    /// ```
    pub fn record_deallocation(&self, component: &str, bytes: u64) {
        // Update global counters
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);
        self.total_deallocated_bytes
            .fetch_add(bytes, Ordering::Relaxed);
        self.current_bytes.fetch_sub(bytes, Ordering::Relaxed);

        // Update component metrics
        let mut components = self.component_allocations.write().unwrap();
        if let Some(entry) = components.get_mut(component) {
            entry.deallocations += 1;
            entry.current_bytes = entry.current_bytes.saturating_sub(bytes);
            entry.total_deallocated_bytes += bytes;
        }
    }

    /// Get the total number of allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::Relaxed)
    }

    /// Get the total number of deallocations.
    pub fn deallocation_count(&self) -> usize {
        self.deallocation_count.load(Ordering::Relaxed)
    }

    /// Get the current memory usage in bytes.
    pub fn current_bytes(&self) -> u64 {
        self.current_bytes.load(Ordering::Relaxed)
    }

    /// Get the peak memory usage in bytes.
    pub fn peak_bytes(&self) -> u64 {
        self.peak_bytes.load(Ordering::Relaxed)
    }

    /// Get the total bytes allocated (lifetime).
    pub fn total_allocated_bytes(&self) -> u64 {
        self.total_allocated_bytes.load(Ordering::Relaxed)
    }

    /// Get the total bytes deallocated (lifetime).
    pub fn total_deallocated_bytes(&self) -> u64 {
        self.total_deallocated_bytes.load(Ordering::Relaxed)
    }

    /// Get current memory usage in megabytes.
    pub fn current_mb(&self) -> f64 {
        self.current_bytes() as f64 / (1024.0 * 1024.0)
    }

    /// Get peak memory usage in megabytes.
    pub fn peak_mb(&self) -> f64 {
        self.peak_bytes() as f64 / (1024.0 * 1024.0)
    }

    /// Get metrics for a specific component.
    pub fn component_metrics(&self, component: &str) -> Option<ComponentMetrics> {
        let components = self.component_allocations.read().unwrap();
        components.get(component).cloned()
    }

    /// Get metrics for all components.
    pub fn all_component_metrics(&self) -> HashMap<String, ComponentMetrics> {
        let components = self.component_allocations.read().unwrap();
        components.clone()
    }

    /// Get a summary snapshot of current metrics.
    pub fn snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            allocation_count: self.allocation_count(),
            deallocation_count: self.deallocation_count(),
            current_bytes: self.current_bytes(),
            peak_bytes: self.peak_bytes(),
            total_allocated_bytes: self.total_allocated_bytes(),
            total_deallocated_bytes: self.total_deallocated_bytes(),
            component_metrics: self.all_component_metrics(),
        }
    }

    /// Check if memory usage is within target (<500MB).
    pub fn is_within_target(&self, target_mb: f64) -> bool {
        self.current_mb() < target_mb
    }

    /// Check if memory usage is approaching target (>80%).
    pub fn is_approaching_target(&self, target_mb: f64) -> bool {
        self.current_mb() / target_mb > 0.8
    }

    /// Reset all metrics.
    ///
    /// This clears all counters and component tracking.
    /// Use with caution - typically only for testing.
    pub fn reset(&self) {
        self.allocation_count.store(0, Ordering::Relaxed);
        self.deallocation_count.store(0, Ordering::Relaxed);
        self.current_bytes.store(0, Ordering::Relaxed);
        self.peak_bytes.store(0, Ordering::Relaxed);
        self.total_allocated_bytes.store(0, Ordering::Relaxed);
        self.total_deallocated_bytes.store(0, Ordering::Relaxed);

        let mut components = self.component_allocations.write().unwrap();
        components.clear();
    }

    /// Update Prometheus metrics.
    ///
    /// This syncs memory metrics to the PerformanceMetrics system.
    pub fn update_prometheus(&self) {
        let perf_metrics = crate::metrics::get_performance_metrics();

        // Update total memory
        perf_metrics.update_memory_usage(self.current_bytes(), "total");

        // Update per-component memory
        let components = self.component_allocations.read().unwrap();
        for (name, metrics) in components.iter() {
            perf_metrics.update_memory_usage(metrics.current_bytes, name);
        }
    }
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics for a specific component.
#[derive(Debug, Clone, Default)]
pub struct ComponentMetrics {
    /// Number of allocations
    pub allocations: usize,

    /// Number of deallocations
    pub deallocations: usize,

    /// Current memory usage in bytes
    pub current_bytes: u64,

    /// Peak memory usage in bytes
    pub peak_bytes: u64,

    /// Total bytes allocated (lifetime)
    pub total_allocated_bytes: u64,

    /// Total bytes deallocated (lifetime)
    pub total_deallocated_bytes: u64,
}

impl ComponentMetrics {
    /// Get current memory usage in megabytes.
    pub fn current_mb(&self) -> f64 {
        self.current_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get peak memory usage in megabytes.
    pub fn peak_mb(&self) -> f64 {
        self.peak_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get the number of currently live allocations.
    pub fn live_allocations(&self) -> usize {
        self.allocations.saturating_sub(self.deallocations)
    }

    /// Get average bytes per allocation.
    pub fn avg_allocation_bytes(&self) -> f64 {
        if self.allocations == 0 {
            0.0
        } else {
            self.total_allocated_bytes as f64 / self.allocations as f64
        }
    }
}

/// Snapshot of memory metrics at a point in time.
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    /// Total allocations
    pub allocation_count: usize,

    /// Total deallocations
    pub deallocation_count: usize,

    /// Current memory usage in bytes
    pub current_bytes: u64,

    /// Peak memory usage in bytes
    pub peak_bytes: u64,

    /// Total bytes allocated (lifetime)
    pub total_allocated_bytes: u64,

    /// Total bytes deallocated (lifetime)
    pub total_deallocated_bytes: u64,

    /// Per-component metrics
    pub component_metrics: HashMap<String, ComponentMetrics>,
}

impl MemorySnapshot {
    /// Get current memory in megabytes.
    pub fn current_mb(&self) -> f64 {
        self.current_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get peak memory in megabytes.
    pub fn peak_mb(&self) -> f64 {
        self.peak_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get number of live allocations.
    pub fn live_allocations(&self) -> usize {
        self.allocation_count
            .saturating_sub(self.deallocation_count)
    }

    /// Check if within memory target.
    pub fn is_within_target(&self, target_mb: f64) -> bool {
        self.current_mb() < target_mb
    }

    /// Get memory utilization percentage against target.
    pub fn utilization_percent(&self, target_mb: f64) -> f64 {
        (self.current_mb() / target_mb) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("test", 1024);

        assert_eq!(metrics.allocation_count(), 1);
        assert_eq!(metrics.current_bytes(), 1024);
        assert_eq!(metrics.peak_bytes(), 1024);
        assert_eq!(metrics.total_allocated_bytes(), 1024);
    }

    #[test]
    fn test_allocation_and_deallocation() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("test", 1024);
        metrics.record_deallocation("test", 512);

        assert_eq!(metrics.allocation_count(), 1);
        assert_eq!(metrics.deallocation_count(), 1);
        assert_eq!(metrics.current_bytes(), 512);
        assert_eq!(metrics.peak_bytes(), 1024); // Peak remains
    }

    #[test]
    fn test_peak_tracking() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("test", 1000);
        metrics.record_allocation("test", 500);
        assert_eq!(metrics.peak_bytes(), 1500);

        metrics.record_deallocation("test", 1000);
        assert_eq!(metrics.current_bytes(), 500);
        assert_eq!(metrics.peak_bytes(), 1500); // Peak doesn't decrease
    }

    #[test]
    fn test_component_tracking() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("indexer", 1024);
        metrics.record_allocation("search", 2048);

        let indexer_metrics = metrics.component_metrics("indexer").unwrap();
        assert_eq!(indexer_metrics.allocations, 1);
        assert_eq!(indexer_metrics.current_bytes, 1024);

        let search_metrics = metrics.component_metrics("search").unwrap();
        assert_eq!(search_metrics.allocations, 1);
        assert_eq!(search_metrics.current_bytes, 2048);
    }

    #[test]
    fn test_component_deallocation() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("indexer", 1024);
        metrics.record_deallocation("indexer", 512);

        let indexer_metrics = metrics.component_metrics("indexer").unwrap();
        assert_eq!(indexer_metrics.allocations, 1);
        assert_eq!(indexer_metrics.deallocations, 1);
        assert_eq!(indexer_metrics.current_bytes, 512);
        assert_eq!(indexer_metrics.peak_bytes, 1024);
    }

    #[test]
    fn test_megabyte_conversions() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("test", 5 * 1024 * 1024); // 5MB

        assert_eq!(metrics.current_mb(), 5.0);
        assert_eq!(metrics.peak_mb(), 5.0);
    }

    #[test]
    fn test_snapshot() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("test", 1024);
        metrics.record_allocation("test", 2048);

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.allocation_count, 2);
        assert_eq!(snapshot.current_bytes, 3072);
        assert_eq!(snapshot.current_mb(), 3072.0 / (1024.0 * 1024.0));
        assert_eq!(snapshot.live_allocations(), 2);
    }

    #[test]
    fn test_target_checking() {
        let metrics = MemoryMetrics::new();

        // 300MB usage
        metrics.record_allocation("test", 300 * 1024 * 1024);

        assert!(metrics.is_within_target(500.0)); // Within 500MB
        assert!(!metrics.is_approaching_target(500.0)); // Not approaching (60%)

        // Add more to approach target
        metrics.record_allocation("test", 150 * 1024 * 1024); // Total 450MB

        assert!(metrics.is_approaching_target(500.0)); // Now approaching (90%)
    }

    #[test]
    fn test_snapshot_target_checking() {
        let metrics = MemoryMetrics::new();
        metrics.record_allocation("test", 400 * 1024 * 1024);

        let snapshot = metrics.snapshot();

        assert!(snapshot.is_within_target(500.0));
        assert_eq!(snapshot.utilization_percent(500.0), 80.0);
    }

    #[test]
    fn test_component_metrics_calculations() {
        let mut component = ComponentMetrics::default();

        component.allocations = 10;
        component.deallocations = 3;
        component.total_allocated_bytes = 10240;
        component.current_bytes = 7168;
        component.peak_bytes = 10 * 1024 * 1024; // 10MB in bytes

        assert_eq!(component.live_allocations(), 7);
        assert_eq!(component.avg_allocation_bytes(), 1024.0);
        assert_eq!(component.current_mb(), 7168.0 / (1024.0 * 1024.0));
        assert_eq!(component.peak_mb(), 10.0);
    }

    #[test]
    fn test_reset() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("test", 1024);
        assert_eq!(metrics.allocation_count(), 1);

        metrics.reset();

        assert_eq!(metrics.allocation_count(), 0);
        assert_eq!(metrics.current_bytes(), 0);
        assert_eq!(metrics.peak_bytes(), 0);
        assert!(metrics.component_metrics("test").is_none());
    }

    #[test]
    fn test_global_metrics() {
        let metrics1 = get_memory_metrics();
        let metrics2 = get_memory_metrics();

        assert!(Arc::ptr_eq(&metrics1, &metrics2));
    }

    #[test]
    fn test_concurrent_allocations() {
        use std::thread;

        let metrics = Arc::new(MemoryMetrics::new());
        let mut handles = vec![];

        for i in 0..10 {
            let metrics = metrics.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    metrics.record_allocation(&format!("thread_{}", i), 1024);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 10 threads * 100 allocations = 1000 total
        assert_eq!(metrics.allocation_count(), 1000);

        // 10 threads * 100 * 1024 bytes = 1024000 bytes
        assert_eq!(metrics.current_bytes(), 1024000);
    }

    #[test]
    fn test_all_component_metrics() {
        let metrics = MemoryMetrics::new();

        metrics.record_allocation("indexer", 1024);
        metrics.record_allocation("search", 2048);
        metrics.record_allocation("cache", 4096);

        let all_metrics = metrics.all_component_metrics();

        assert_eq!(all_metrics.len(), 3);
        assert!(all_metrics.contains_key("indexer"));
        assert!(all_metrics.contains_key("search"));
        assert!(all_metrics.contains_key("cache"));
    }

    #[test]
    fn test_zero_division_safety() {
        let component = ComponentMetrics::default();
        assert_eq!(component.avg_allocation_bytes(), 0.0);
    }
}
