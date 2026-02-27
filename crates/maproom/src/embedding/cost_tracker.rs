//! Cost tracking for embedding API usage.
//!
//! This module provides detailed cost monitoring and reporting for embedding generation,
//! tracking tokens consumed and estimating costs based on current API pricing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Cost tracker for embedding API usage.
#[derive(Debug, Clone)]
pub struct CostTracker {
    /// Shared state for cost tracking
    state: Arc<CostTrackerState>,
}

#[derive(Debug)]
struct CostTrackerState {
    /// Total tokens consumed
    total_tokens: AtomicU64,

    /// Total API requests made
    total_requests: AtomicU64,

    /// Price per 1K tokens in USD
    price_per_1k_tokens: f64,
}

impl CostTracker {
    /// Create a new cost tracker.
    ///
    /// # Arguments
    ///
    /// * `price_per_1k_tokens` - Cost per 1,000 tokens in USD (default: $0.00002 for text-embedding-3-small)
    pub fn new(price_per_1k_tokens: f64) -> Self {
        Self {
            state: Arc::new(CostTrackerState {
                total_tokens: AtomicU64::new(0),
                total_requests: AtomicU64::new(0),
                price_per_1k_tokens,
            }),
        }
    }

    /// Create a new cost tracker with default pricing for text-embedding-3-small.
    pub fn default_pricing() -> Self {
        Self::new(0.00002)
    }

    /// Record token usage from an API call.
    pub fn record_usage(&self, tokens: u64) {
        self.state.total_tokens.fetch_add(tokens, Ordering::Relaxed);
        self.state.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total tokens consumed.
    pub fn total_tokens(&self) -> u64 {
        self.state.total_tokens.load(Ordering::Relaxed)
    }

    /// Get total API requests made.
    pub fn total_requests(&self) -> u64 {
        self.state.total_requests.load(Ordering::Relaxed)
    }

    /// Calculate estimated cost in USD.
    pub fn estimated_cost_usd(&self) -> f64 {
        let tokens = self.total_tokens() as f64;
        (tokens / 1000.0) * self.state.price_per_1k_tokens
    }

    /// Get the price per 1K tokens.
    pub fn price_per_1k_tokens(&self) -> f64 {
        self.state.price_per_1k_tokens
    }

    /// Reset all counters.
    pub fn reset(&self) {
        self.state.total_tokens.store(0, Ordering::Relaxed);
        self.state.total_requests.store(0, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics.
    pub fn snapshot(&self) -> CostSnapshot {
        CostSnapshot {
            total_tokens: self.total_tokens(),
            total_requests: self.total_requests(),
            estimated_cost_usd: self.estimated_cost_usd(),
            price_per_1k_tokens: self.price_per_1k_tokens(),
        }
    }
}

/// Snapshot of cost metrics at a point in time.
#[derive(Debug, Clone)]
pub struct CostSnapshot {
    /// Total tokens consumed
    pub total_tokens: u64,

    /// Total API requests made
    pub total_requests: u64,

    /// Estimated cost in USD
    pub estimated_cost_usd: f64,

    /// Price per 1K tokens
    pub price_per_1k_tokens: f64,
}

impl CostSnapshot {
    /// Calculate average tokens per request.
    pub fn avg_tokens_per_request(&self) -> f64 {
        if self.total_requests > 0 {
            self.total_tokens as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }

    /// Calculate cost per request.
    pub fn cost_per_request(&self) -> f64 {
        if self.total_requests > 0 {
            self.estimated_cost_usd / self.total_requests as f64
        } else {
            0.0
        }
    }

    /// Format a detailed cost report.
    pub fn report(&self) -> String {
        format!(
            "Cost Report:\n\
             Total Tokens: {}\n\
             Total Requests: {}\n\
             Avg Tokens/Request: {:.1}\n\
             Estimated Cost: ${:.4}\n\
             Cost/Request: ${:.6}\n\
             Price/1K Tokens: ${:.6}",
            self.total_tokens,
            self.total_requests,
            self.avg_tokens_per_request(),
            self.estimated_cost_usd,
            self.cost_per_request(),
            self.price_per_1k_tokens
        )
    }

    /// Format a compact summary.
    pub fn summary(&self) -> String {
        format!(
            "{} tokens, {} requests, ${:.4}",
            self.total_tokens, self.total_requests, self.estimated_cost_usd
        )
    }
}

/// Cost estimator for planning embedding generation.
pub struct CostEstimator {
    /// Average tokens per chunk
    avg_tokens_per_chunk: f64,

    /// Price per 1K tokens
    price_per_1k_tokens: f64,
}

impl Default for CostEstimator {
    fn default() -> Self {
        Self::new(200.0, 0.00002)
    }
}

impl CostEstimator {
    /// Create a new cost estimator.
    ///
    /// # Arguments
    ///
    /// * `avg_tokens_per_chunk` - Average number of tokens per chunk (default: ~200)
    /// * `price_per_1k_tokens` - Price per 1,000 tokens in USD (default: $0.00002)
    pub fn new(avg_tokens_per_chunk: f64, price_per_1k_tokens: f64) -> Self {
        Self {
            avg_tokens_per_chunk,
            price_per_1k_tokens,
        }
    }

    /// Estimate cost for embedding N chunks.
    ///
    /// This estimates the cost for generating both code_embedding and text_embedding
    /// for each chunk, so 2x the number of embeddings.
    pub fn estimate_cost(&self, num_chunks: usize) -> CostEstimate {
        let num_embeddings = num_chunks * 2; // code + text embeddings
        let total_tokens = num_embeddings as f64 * self.avg_tokens_per_chunk;
        let estimated_cost = (total_tokens / 1000.0) * self.price_per_1k_tokens;

        CostEstimate {
            num_chunks,
            num_embeddings,
            estimated_tokens: total_tokens as u64,
            estimated_cost_usd: estimated_cost,
        }
    }
}

/// Cost estimate for embedding generation.
#[derive(Debug, Clone)]
pub struct CostEstimate {
    /// Number of chunks to process
    pub num_chunks: usize,

    /// Number of embeddings to generate (chunks * 2)
    pub num_embeddings: usize,

    /// Estimated tokens to be consumed
    pub estimated_tokens: u64,

    /// Estimated cost in USD
    pub estimated_cost_usd: f64,
}

impl CostEstimate {
    /// Format the estimate as a string.
    pub fn format(&self) -> String {
        format!(
            "Cost Estimate for {} chunks:\n\
             Embeddings: {} (code + text)\n\
             Estimated Tokens: {}\n\
             Estimated Cost: ${:.4}",
            self.num_chunks, self.num_embeddings, self.estimated_tokens, self.estimated_cost_usd
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_tracker_basic() {
        let tracker = CostTracker::default_pricing();

        assert_eq!(tracker.total_tokens(), 0);
        assert_eq!(tracker.total_requests(), 0);
        assert_eq!(tracker.estimated_cost_usd(), 0.0);

        tracker.record_usage(1000);
        assert_eq!(tracker.total_tokens(), 1000);
        assert_eq!(tracker.total_requests(), 1);
        assert_eq!(tracker.estimated_cost_usd(), 0.00002);

        tracker.record_usage(1000);
        assert_eq!(tracker.total_tokens(), 2000);
        assert_eq!(tracker.total_requests(), 2);
        assert_eq!(tracker.estimated_cost_usd(), 0.00004);
    }

    #[test]
    fn test_cost_tracker_custom_pricing() {
        let tracker = CostTracker::new(0.0001); // Higher price for testing

        tracker.record_usage(10000);
        assert_eq!(tracker.estimated_cost_usd(), 0.001);
    }

    #[test]
    fn test_cost_tracker_reset() {
        let tracker = CostTracker::default_pricing();

        tracker.record_usage(1000);
        assert_eq!(tracker.total_tokens(), 1000);

        tracker.reset();
        assert_eq!(tracker.total_tokens(), 0);
        assert_eq!(tracker.total_requests(), 0);
    }

    #[test]
    fn test_cost_snapshot() {
        let tracker = CostTracker::default_pricing();

        tracker.record_usage(1000);
        tracker.record_usage(2000);
        tracker.record_usage(3000);

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.total_tokens, 6000);
        assert_eq!(snapshot.total_requests, 3);
        assert_eq!(snapshot.avg_tokens_per_request(), 2000.0);
        assert!(snapshot.cost_per_request() > 0.0);

        let report = snapshot.report();
        assert!(report.contains("Total Tokens: 6000"));
        assert!(report.contains("Total Requests: 3"));
    }

    #[test]
    fn test_cost_snapshot_summary() {
        let snapshot = CostSnapshot {
            total_tokens: 50000,
            total_requests: 10,
            estimated_cost_usd: 1.0,
            price_per_1k_tokens: 0.00002,
        };

        let summary = snapshot.summary();
        assert!(summary.contains("50000 tokens"));
        assert!(summary.contains("10 requests"));
        assert!(summary.contains("$1.0000"));
    }

    #[test]
    fn test_cost_estimator() {
        let estimator = CostEstimator::default();

        let estimate = estimator.estimate_cost(1000);
        assert_eq!(estimate.num_chunks, 1000);
        assert_eq!(estimate.num_embeddings, 2000); // 2x for code + text
        assert_eq!(estimate.estimated_tokens, 400000); // 2000 * 200
        assert!((estimate.estimated_cost_usd - 0.008).abs() < 0.0001);
    }

    #[test]
    fn test_cost_estimator_format() {
        let estimator = CostEstimator::default();
        let estimate = estimator.estimate_cost(100);

        let formatted = estimate.format();
        assert!(formatted.contains("100 chunks"));
        assert!(formatted.contains("200 (code + text)"));
    }

    #[test]
    fn test_concurrent_cost_tracking() {
        use std::thread;

        let tracker = CostTracker::default_pricing();
        let mut handles = vec![];

        for _ in 0..10 {
            let tracker_clone = tracker.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    tracker_clone.record_usage(100);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(tracker.total_tokens(), 100000); // 10 threads * 100 iterations * 100 tokens
        assert_eq!(tracker.total_requests(), 1000); // 10 threads * 100 iterations
    }

    #[test]
    fn test_zero_requests_snapshot() {
        let tracker = CostTracker::default_pricing();
        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.avg_tokens_per_request(), 0.0);
        assert_eq!(snapshot.cost_per_request(), 0.0);
    }
}
