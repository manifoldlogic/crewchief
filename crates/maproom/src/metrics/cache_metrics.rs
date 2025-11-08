//! Cache metrics tracking for embedding deduplication.
//!
//! Tracks cache hits/misses and cost savings from blob SHA-based deduplication.
//! Used by the cache-aware upsert logic to measure effectiveness of content-addressed storage.

use std::sync::atomic::{AtomicU64, Ordering};

/// Metrics for embedding cache effectiveness.
///
/// Tracks cache hits (blob SHA exists in code_embeddings) vs cache misses
/// (new blob SHA, need to generate embedding). Uses atomic counters for
/// thread-safe updates during concurrent chunk processing.
#[derive(Debug, Default)]
pub struct CacheMetrics {
    /// Number of cache hits (existing blob_sha found in code_embeddings)
    cache_hits: AtomicU64,

    /// Number of cache misses (new blob_sha, embedding generated)
    cache_misses: AtomicU64,
}

impl CacheMetrics {
    /// Create a new CacheMetrics instance.
    pub fn new() -> Self {
        Self {
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    /// Record a cache hit (blob SHA exists, reusing existing embedding).
    pub fn record_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss (blob SHA is new, generating embedding).
    pub fn record_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the total number of cache hits.
    pub fn hits(&self) -> u64 {
        self.cache_hits.load(Ordering::Relaxed)
    }

    /// Get the total number of cache misses.
    pub fn misses(&self) -> u64 {
        self.cache_misses.load(Ordering::Relaxed)
    }

    /// Calculate cache hit rate (0.0 to 1.0).
    ///
    /// Returns 0.0 if no cache operations have been performed.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits();
        let misses = self.misses();
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Calculate estimated cost savings based on OpenAI embedding pricing.
    ///
    /// Cost per embedding: $0.00002 (text-embedding-3-small)
    /// Savings = cache_hits × $0.00002
    pub fn estimated_savings_usd(&self) -> f64 {
        const COST_PER_EMBEDDING: f64 = 0.00002;
        self.hits() as f64 * COST_PER_EMBEDDING
    }

    /// Calculate total embeddings generated (cache misses).
    pub fn embeddings_generated(&self) -> u64 {
        self.misses()
    }

    /// Calculate estimated cost for embeddings generated.
    pub fn estimated_cost_usd(&self) -> f64 {
        const COST_PER_EMBEDDING: f64 = 0.00002;
        self.embeddings_generated() as f64 * COST_PER_EMBEDDING
    }

    /// Generate a formatted report of cache metrics.
    ///
    /// Format matches specification from planning/architecture.md lines 457-465:
    /// ```text
    /// [INFO] Indexing complete:
    ///   - Chunks processed: 10,000
    ///   - Cache hits: 8,000 (80%)
    ///   - Cache misses: 2,000 (20%)
    ///   - Embeddings generated: 2,000
    ///   - Estimated cost: $0.04
    /// ```
    pub fn report(&self) -> String {
        let hits = self.hits();
        let misses = self.misses();
        let total = hits + misses;
        let hit_rate = self.hit_rate() * 100.0;
        let miss_rate = if total > 0 {
            (misses as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let cost = self.estimated_cost_usd();

        format!(
            "Cache metrics:\n  \
            - Chunks processed: {total}\n  \
            - Cache hits: {hits} ({hit_rate:.1}%)\n  \
            - Cache misses: {misses} ({miss_rate:.1}%)\n  \
            - Embeddings generated: {misses}\n  \
            - Estimated cost: ${cost:.4}"
        )
    }

    /// Reset all metrics to zero.
    ///
    /// Useful for starting a new scan/indexing operation.
    pub fn reset(&self) {
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metrics() {
        let metrics = CacheMetrics::new();
        assert_eq!(metrics.hits(), 0);
        assert_eq!(metrics.misses(), 0);
        assert_eq!(metrics.hit_rate(), 0.0);
    }

    #[test]
    fn test_record_hit() {
        let metrics = CacheMetrics::new();
        metrics.record_hit();
        assert_eq!(metrics.hits(), 1);
        assert_eq!(metrics.misses(), 0);

        metrics.record_hit();
        assert_eq!(metrics.hits(), 2);
    }

    #[test]
    fn test_record_miss() {
        let metrics = CacheMetrics::new();
        metrics.record_miss();
        assert_eq!(metrics.hits(), 0);
        assert_eq!(metrics.misses(), 1);

        metrics.record_miss();
        assert_eq!(metrics.misses(), 2);
    }

    #[test]
    fn test_hit_rate() {
        let metrics = CacheMetrics::new();

        // 0 operations = 0.0 hit rate
        assert_eq!(metrics.hit_rate(), 0.0);

        // 8 hits, 2 misses = 80% hit rate
        for _ in 0..8 {
            metrics.record_hit();
        }
        for _ in 0..2 {
            metrics.record_miss();
        }

        assert_eq!(metrics.hits(), 8);
        assert_eq!(metrics.misses(), 2);
        assert_eq!(metrics.hit_rate(), 0.8);
    }

    #[test]
    fn test_estimated_cost() {
        let metrics = CacheMetrics::new();

        // 2000 misses = 2000 embeddings generated
        for _ in 0..2000 {
            metrics.record_miss();
        }

        // Cost = 2000 * $0.00002 = $0.04
        let cost = metrics.estimated_cost_usd();
        assert!((cost - 0.04).abs() < 0.0001);
    }

    #[test]
    fn test_estimated_savings() {
        let metrics = CacheMetrics::new();

        // 8000 hits = $0.16 saved
        for _ in 0..8000 {
            metrics.record_hit();
        }

        let savings = metrics.estimated_savings_usd();
        assert!((savings - 0.16).abs() < 0.0001);
    }

    #[test]
    fn test_report_format() {
        let metrics = CacheMetrics::new();

        // 8000 hits, 2000 misses
        for _ in 0..8000 {
            metrics.record_hit();
        }
        for _ in 0..2000 {
            metrics.record_miss();
        }

        let report = metrics.report();
        assert!(report.contains("10000")); // Total chunks
        assert!(report.contains("8000")); // Cache hits
        assert!(report.contains("80.0%")); // Hit rate
        assert!(report.contains("2000")); // Cache misses
        assert!(report.contains("20.0%")); // Miss rate
        assert!(report.contains("$0.0400")); // Cost
    }

    #[test]
    fn test_reset() {
        let metrics = CacheMetrics::new();

        metrics.record_hit();
        metrics.record_miss();
        assert_eq!(metrics.hits(), 1);
        assert_eq!(metrics.misses(), 1);

        metrics.reset();
        assert_eq!(metrics.hits(), 0);
        assert_eq!(metrics.misses(), 0);
        assert_eq!(metrics.hit_rate(), 0.0);
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(CacheMetrics::new());
        let mut handles = vec![];

        // Spawn 10 threads, each recording 100 hits
        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics_clone.record_hit();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have exactly 1000 hits
        assert_eq!(metrics.hits(), 1000);
    }
}
