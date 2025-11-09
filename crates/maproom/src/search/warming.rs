//! Cache warming for search results.
//!
//! This module provides functionality to pre-populate the search cache with
//! popular queries on startup, improving cache hit rates for common queries.
//!
//! # Architecture
//!
//! Cache warming strategies:
//! 1. **Popular Queries**: Load from configuration or analytics
//! 2. **Recent Queries**: From query log (if available)
//! 3. **Predictive**: Based on common patterns
//!
//! # Performance Target
//!
//! Warming should complete within 30 seconds on startup.
//!
//! # Example
//!
//! ```ignore
//! use crewchief_maproom::search::warming::CacheWarmer;
//! use crewchief_maproom::search::SearchCache;
//! use crewchief_maproom::search::SearchPipeline;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let cache = Arc::new(SearchCache::new(1000));
//!     let warmer = CacheWarmer::new(cache);
//!
//!     // Warm with popular queries
//!     let queries = vec![
//!         "authentication".to_string(),
//!         "database connection".to_string(),
//!         "error handling".to_string(),
//!     ];
//!
//!     // let pipeline = SearchPipeline::new(...);
//!     // warmer.warm_with_queries(&queries, 1, None, &pipeline, None).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::search::cache::{CacheKey, SearchCache};
use crate::search::pipeline::SearchPipeline;
use crate::search::results::SearchOptions;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Cache warmer for preloading popular queries.
pub struct CacheWarmer {
    /// Reference to the search cache
    cache: Arc<SearchCache>,
}

impl CacheWarmer {
    /// Create a new CacheWarmer.
    pub fn new(cache: Arc<SearchCache>) -> Self {
        Self { cache }
    }

    /// Warm the cache with a list of queries.
    ///
    /// Executes queries in parallel (with concurrency limit) and populates the cache.
    ///
    /// # Arguments
    ///
    /// * `queries` - List of query strings to execute
    /// * `repo_id` - Repository ID to search in
    /// * `worktree_id` - Optional worktree ID
    /// * `pipeline` - Search pipeline for executing queries
    /// * `timeout` - Maximum time to spend warming (default: 30s)
    ///
    /// # Returns
    ///
    /// Returns the number of queries successfully warmed.
    pub async fn warm_with_queries(
        &self,
        queries: &[String],
        repo_id: i64,
        worktree_id: Option<i64>,
        pipeline: &SearchPipeline,
        timeout: Option<Duration>,
    ) -> Result<usize, WarmingError> {
        let start = Instant::now();
        let timeout = timeout.unwrap_or(Duration::from_secs(30));

        info!(
            "Starting cache warming with {} queries (timeout: {}s)",
            queries.len(),
            timeout.as_secs()
        );

        let mut warmed_count = 0;

        for (i, query) in queries.iter().enumerate() {
            // Check timeout
            if start.elapsed() >= timeout {
                warn!(
                    "Cache warming timeout after {} queries ({:.2}s)",
                    warmed_count,
                    start.elapsed().as_secs_f64()
                );
                break;
            }

            // Execute query
            let options = SearchOptions::new(repo_id, worktree_id, 10);

            match pipeline.search(query, options).await {
                Ok(results) => {
                    // Cache the results
                    let key = CacheKey::new(query, repo_id, worktree_id, 10);
                    self.cache.put(key, results);
                    warmed_count += 1;
                    debug!(
                        "Warmed query {}/{}: '{}' ({:.2}s elapsed)",
                        i + 1,
                        queries.len(),
                        query,
                        start.elapsed().as_secs_f64()
                    );
                }
                Err(e) => {
                    warn!("Failed to warm query '{}': {}", query, e);
                }
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        info!(
            "Cache warming completed: {}/{} queries in {:.2}s",
            warmed_count,
            queries.len(),
            elapsed
        );

        Ok(warmed_count)
    }

    /// Warm the cache with common code patterns.
    ///
    /// Pre-populates cache with typical code search queries.
    pub async fn warm_with_patterns(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        pipeline: &SearchPipeline,
    ) -> Result<usize, WarmingError> {
        let patterns = [
            "main", "init", "config", "error", "handle", "process", "create", "update", "delete",
            "get", "set",
        ];

        let queries: Vec<String> = patterns.iter().map(|s| s.to_string()).collect();

        self.warm_with_queries(&queries, repo_id, worktree_id, pipeline, None)
            .await
    }

    /// Get warming statistics.
    pub fn stats(&self) -> WarmingStats {
        let cache_stats = self.cache.stats();

        WarmingStats {
            cache_size: cache_stats.size,
            cache_capacity: cache_stats.capacity,
            cache_hit_rate: cache_stats.hit_rate(),
        }
    }
}

/// Cache warming statistics.
#[derive(Debug, Clone)]
pub struct WarmingStats {
    /// Current cache size
    pub cache_size: usize,
    /// Cache capacity
    pub cache_capacity: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
}

/// Errors that can occur during cache warming.
#[derive(Debug, thiserror::Error)]
pub enum WarmingError {
    /// Search pipeline error
    #[error("Search pipeline error: {0}")]
    Pipeline(String),

    /// Timeout during warming
    #[error("Warming timeout after {0} queries")]
    Timeout(usize),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warming_stats_structure() {
        let stats = WarmingStats {
            cache_size: 50,
            cache_capacity: 1000,
            cache_hit_rate: 0.75,
        };

        assert_eq!(stats.cache_size, 50);
        assert_eq!(stats.cache_capacity, 1000);
        assert_eq!(stats.cache_hit_rate, 0.75);
    }

    #[test]
    fn test_warmer_creation() {
        let cache = Arc::new(SearchCache::new(100));
        let warmer = CacheWarmer::new(cache);

        let stats = warmer.stats();
        assert_eq!(stats.cache_size, 0);
        assert_eq!(stats.cache_capacity, 100);
    }
}
