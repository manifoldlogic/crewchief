//! Cache warming strategies to pre-populate caches with critical data.
//!
//! Implements multiple warming strategies:
//! - **Startup**: Warm common queries on application startup
//! - **Predictive**: Analyze patterns and warm likely queries
//! - **Scheduled**: Refresh popular entries before TTL expiration
//! - **Manual**: User-defined warming via CLI

use super::system::CacheSystem;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::{debug, info};

/// Cache warming strategy.
#[derive(Debug, Clone)]
pub enum WarmingStrategy {
    /// Warm on application startup
    Startup,
    /// Warm based on predicted usage patterns
    Predictive,
    /// Warm on a schedule to refresh before TTL expiration
    Scheduled,
    /// Manual warming from a list of queries
    Manual,
}

/// Cache warmer for pre-populating caches with critical data.
///
/// Warming caches improves performance by ensuring frequently-used
/// data is available before it's requested.
pub struct CacheWarmer {
    /// The cache system to warm
    cache: Arc<CacheSystem>,
    /// Warming strategy
    strategy: WarmingStrategy,
    /// Queries to warm (for manual/startup warming)
    warm_queries: Vec<String>,
}

impl CacheWarmer {
    /// Create a new cache warmer.
    pub fn new(cache: Arc<CacheSystem>, strategy: WarmingStrategy) -> Self {
        Self {
            cache,
            strategy,
            warm_queries: Vec::new(),
        }
    }

    /// Create a cache warmer with predefined queries.
    pub fn with_queries(
        cache: Arc<CacheSystem>,
        strategy: WarmingStrategy,
        queries: Vec<String>,
    ) -> Self {
        Self {
            cache,
            strategy,
            warm_queries: queries,
        }
    }

    /// Load warming queries from a file.
    ///
    /// File format: one query per line, empty lines and lines starting with '#' are ignored.
    pub async fn load_queries_from_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read warming queries from {}", path.display()))?;

        let queries: Vec<String> = content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
            .map(|line| line.trim().to_string())
            .collect();

        info!(
            "Loaded {} warming queries from {}",
            queries.len(),
            path.display()
        );
        self.warm_queries = queries;

        Ok(())
    }

    /// Warm the cache with the configured strategy.
    ///
    /// Note: This is a placeholder for warming. In a real implementation,
    /// you would execute searches and populate the cache. This requires
    /// access to the search infrastructure which is not included in this
    /// cache management ticket.
    pub async fn warm(&self) -> Result<WarmingStats> {
        match self.strategy {
            WarmingStrategy::Startup => self.warm_startup().await,
            WarmingStrategy::Predictive => self.warm_predictive().await,
            WarmingStrategy::Scheduled => self.warm_scheduled().await,
            WarmingStrategy::Manual => self.warm_manual().await,
        }
    }

    /// Warm cache on startup with predefined queries.
    async fn warm_startup(&self) -> Result<WarmingStats> {
        info!(
            "Starting cache warming on startup ({} queries)",
            self.warm_queries.len()
        );

        let mut stats = WarmingStats::new();

        for query in &self.warm_queries {
            // Check if already cached
            if self.cache.get_query(query).await.is_some() {
                stats.already_cached += 1;
                debug!("Query already cached: {}", query);
                continue;
            }

            // Note: Actual search execution would happen here
            // For now, we just log that we would warm this query
            debug!("Would warm query: {}", query);
            stats.warmed += 1;
        }

        info!(
            "Startup warming complete: {} warmed, {} already cached",
            stats.warmed, stats.already_cached
        );

        Ok(stats)
    }

    /// Warm cache based on predicted usage patterns.
    async fn warm_predictive(&self) -> Result<WarmingStats> {
        info!("Starting predictive cache warming");

        let stats = WarmingStats::new();

        // Note: Real implementation would analyze query logs,
        // access patterns, and predict likely queries.
        // For now, this is a placeholder.

        debug!("Predictive warming: analyzing query patterns...");

        // Placeholder: In a real implementation, we would:
        // 1. Analyze recent query logs
        // 2. Identify patterns and frequently co-occurring queries
        // 3. Predict likely next queries based on current cache state
        // 4. Execute and cache those queries

        info!(
            "Predictive warming complete: {} queries predicted and warmed",
            stats.warmed
        );

        Ok(stats)
    }

    /// Warm cache on schedule to refresh before TTL expiration.
    async fn warm_scheduled(&self) -> Result<WarmingStats> {
        info!("Starting scheduled cache warming");

        let stats = WarmingStats::new();

        // Note: Real implementation would:
        // 1. Identify high-value cache entries near expiration
        // 2. Re-execute queries to refresh them
        // 3. Update cache with fresh results

        debug!("Scheduled warming: refreshing high-value entries...");

        info!(
            "Scheduled warming complete: {} entries refreshed",
            stats.warmed
        );

        Ok(stats)
    }

    /// Warm cache manually with provided queries.
    async fn warm_manual(&self) -> Result<WarmingStats> {
        info!(
            "Starting manual cache warming ({} queries)",
            self.warm_queries.len()
        );

        let mut stats = WarmingStats::new();

        for query in &self.warm_queries {
            if self.cache.get_query(query).await.is_some() {
                stats.already_cached += 1;
                debug!("Query already cached: {}", query);
                continue;
            }

            debug!("Would manually warm query: {}", query);
            stats.warmed += 1;
        }

        info!(
            "Manual warming complete: {} warmed, {} already cached",
            stats.warmed, stats.already_cached
        );

        Ok(stats)
    }

    /// Get the number of queries configured for warming.
    pub fn query_count(&self) -> usize {
        self.warm_queries.len()
    }

    /// Get a reference to the warming queries.
    pub fn queries(&self) -> &[String] {
        &self.warm_queries
    }

    /// Add a query to the warming list.
    pub fn add_query(&mut self, query: String) {
        if !self.warm_queries.contains(&query) {
            self.warm_queries.push(query);
        }
    }

    /// Clear all warming queries.
    pub fn clear_queries(&mut self) {
        self.warm_queries.clear();
    }
}

/// Statistics from a cache warming operation.
#[derive(Debug, Default, Clone)]
pub struct WarmingStats {
    /// Number of entries warmed
    pub warmed: usize,
    /// Number of entries already cached
    pub already_cached: usize,
    /// Number of errors encountered
    pub errors: usize,
}

impl WarmingStats {
    /// Create new warming statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total number of entries processed.
    pub fn total_processed(&self) -> usize {
        self.warmed + self.already_cached + self.errors
    }

    /// Get warming effectiveness (warmed / total processed).
    pub fn effectiveness(&self) -> f64 {
        let total = self.total_processed();
        if total > 0 {
            self.warmed as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Check if warming was successful (no errors).
    pub fn is_successful(&self) -> bool {
        self.errors == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;

    #[tokio::test]
    async fn test_cache_warmer_creation() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let warmer = CacheWarmer::new(Arc::clone(&cache), WarmingStrategy::Startup);

        assert_eq!(warmer.query_count(), 0);
        assert!(warmer.queries().is_empty());
    }

    #[tokio::test]
    async fn test_cache_warmer_with_queries() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let queries = vec!["query1".to_string(), "query2".to_string()];
        let warmer =
            CacheWarmer::with_queries(Arc::clone(&cache), WarmingStrategy::Manual, queries.clone());

        assert_eq!(warmer.query_count(), 2);
        assert_eq!(warmer.queries(), &queries);
    }

    #[tokio::test]
    async fn test_cache_warmer_add_query() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let mut warmer = CacheWarmer::new(Arc::clone(&cache), WarmingStrategy::Manual);

        warmer.add_query("query1".to_string());
        assert_eq!(warmer.query_count(), 1);

        warmer.add_query("query2".to_string());
        assert_eq!(warmer.query_count(), 2);

        // Duplicate should not be added
        warmer.add_query("query1".to_string());
        assert_eq!(warmer.query_count(), 2);
    }

    #[tokio::test]
    async fn test_cache_warmer_clear_queries() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let queries = vec!["query1".to_string(), "query2".to_string()];
        let mut warmer =
            CacheWarmer::with_queries(Arc::clone(&cache), WarmingStrategy::Manual, queries);

        assert_eq!(warmer.query_count(), 2);

        warmer.clear_queries();
        assert_eq!(warmer.query_count(), 0);
    }

    #[tokio::test]
    async fn test_warming_stats() {
        let mut stats = WarmingStats::new();

        stats.warmed = 10;
        stats.already_cached = 5;
        stats.errors = 1;

        assert_eq!(stats.total_processed(), 16);
        assert!((stats.effectiveness() - 0.625).abs() < 0.01); // 10/16 = 0.625
        assert!(!stats.is_successful()); // Has errors
    }

    #[tokio::test]
    async fn test_warming_stats_successful() {
        let mut stats = WarmingStats::new();

        stats.warmed = 10;
        stats.already_cached = 5;

        assert!(stats.is_successful());
    }

    #[tokio::test]
    async fn test_load_queries_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let mut warmer = CacheWarmer::new(Arc::clone(&cache), WarmingStrategy::Startup);

        // Create a temporary file with queries
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "query1").unwrap();
        writeln!(file, "# This is a comment").unwrap();
        writeln!(file, "").unwrap(); // Empty line
        writeln!(file, "query2").unwrap();
        writeln!(file, "query3").unwrap();
        file.flush().unwrap();

        // Load queries
        warmer.load_queries_from_file(file.path()).await.unwrap();

        assert_eq!(warmer.query_count(), 3);
        assert_eq!(warmer.queries(), &["query1", "query2", "query3"]);
    }
}
