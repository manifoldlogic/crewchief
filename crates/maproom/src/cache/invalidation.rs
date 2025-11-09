//! Cache invalidation logic for maintaining data freshness.
//!
//! Implements smart invalidation strategies:
//! - **File Change**: Invalidate on file modification
//! - **Re-index**: Clear all caches on repository re-index
//! - **Manual**: User-triggered invalidation via CLI
//! - **Pattern**: Invalidate queries matching a pattern

use super::system::CacheSystem;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Cache invalidation trigger.
#[derive(Debug, Clone)]
pub enum InvalidationTrigger {
    /// File was modified
    FileChange(String),
    /// Repository was re-indexed
    ReIndex(i64),
    /// Manual invalidation requested
    Manual,
    /// Pattern-based invalidation
    Pattern(String),
}

/// Cache invalidator for managing cache freshness.
///
/// Ensures caches don't serve stale data by invalidating
/// entries when the underlying data changes.
pub struct CacheInvalidator {
    /// The cache system to invalidate
    cache: Arc<CacheSystem>,
}

impl CacheInvalidator {
    /// Create a new cache invalidator.
    pub fn new(cache: Arc<CacheSystem>) -> Self {
        Self { cache }
    }

    /// Handle file change invalidation.
    ///
    /// When a file changes, we need to:
    /// 1. Invalidate parse tree cache for that file
    /// 2. Invalidate context bundles containing chunks from that file
    /// 3. Optionally clear query cache (conservative approach)
    pub async fn on_file_changed(&self, file_path: &Path) -> Result<InvalidationStats> {
        let path_str = file_path.to_string_lossy();
        info!("Invalidating cache for file change: {}", path_str);

        let mut stats = InvalidationStats::new();

        // Invalidate parse tree cache for this file
        self.cache.invalidate_parse_tree(&path_str).await;
        stats.parse_tree_invalidated += 1;

        // Note: In a real implementation, we would:
        // 1. Query the database to find chunks belonging to this file
        // 2. Invalidate context cache entries containing those chunks
        // For now, we conservatively clear the context cache
        self.cache.clear_l3().await;
        stats.context_invalidated = 1; // Conservative: cleared all

        // Optionally clear query cache (conservative approach)
        // In a real implementation, we might be more selective
        self.cache.clear_l1().await;
        stats.query_invalidated = 1; // Conservative: cleared all

        info!(
            "File change invalidation complete: parse_tree={}, context={}, query={}",
            stats.parse_tree_invalidated, stats.context_invalidated, stats.query_invalidated
        );

        Ok(stats)
    }

    /// Handle re-index invalidation.
    ///
    /// When a repository is re-indexed, all caches should be cleared
    /// to prevent serving stale data.
    pub async fn on_reindex(&self, repo_id: i64) -> Result<InvalidationStats> {
        info!("Invalidating all caches for repo re-index: {}", repo_id);

        let mut stats = InvalidationStats::new();

        // Clear all caches
        self.cache.clear_all().await;

        // Mark all caches as invalidated
        stats.query_invalidated = 1;
        stats.embedding_invalidated = 1;
        stats.context_invalidated = 1;
        stats.parse_tree_invalidated = 1;

        info!("Re-index invalidation complete: all caches cleared");

        Ok(stats)
    }

    /// Handle manual invalidation.
    ///
    /// Clear all caches when user requests it via CLI.
    pub async fn on_manual(&self) -> Result<InvalidationStats> {
        info!("Manual cache invalidation requested");

        let mut stats = InvalidationStats::new();

        // Clear all caches
        self.cache.clear_all().await;

        stats.query_invalidated = 1;
        stats.embedding_invalidated = 1;
        stats.context_invalidated = 1;
        stats.parse_tree_invalidated = 1;

        info!("Manual invalidation complete: all caches cleared");

        Ok(stats)
    }

    /// Handle pattern-based invalidation.
    ///
    /// Invalidate specific cache entries matching a pattern.
    /// Note: Current implementation clears L1 query cache.
    /// In a real implementation, we would selectively invalidate matching entries.
    pub async fn on_pattern(&self, pattern: &str) -> Result<InvalidationStats> {
        info!("Pattern-based cache invalidation: {}", pattern);

        let mut stats = InvalidationStats::new();

        // Note: Real implementation would:
        // 1. Iterate through L1 query cache
        // 2. Check if keys match the pattern
        // 3. Remove only matching entries
        //
        // For now, we conservatively clear the entire L1 cache
        self.cache.clear_l1().await;
        stats.query_invalidated = 1;

        info!("Pattern invalidation complete: query_cache cleared");

        Ok(stats)
    }

    /// Invalidate specific cache layers.
    pub async fn invalidate_layers(&self, layers: &[CacheLayer]) -> Result<InvalidationStats> {
        let mut stats = InvalidationStats::new();

        for layer in layers {
            match layer {
                CacheLayer::L1Query => {
                    self.cache.clear_l1().await;
                    stats.query_invalidated = 1;
                    debug!("Invalidated L1 query cache");
                }
                CacheLayer::L2Embedding => {
                    self.cache.clear_l2().await;
                    stats.embedding_invalidated = 1;
                    debug!("Invalidated L2 embedding cache");
                }
                CacheLayer::L3Context => {
                    self.cache.clear_l3().await;
                    stats.context_invalidated = 1;
                    debug!("Invalidated L3 context cache");
                }
                CacheLayer::ParseTree => {
                    self.cache.clear_parse_tree().await;
                    stats.parse_tree_invalidated = 1;
                    debug!("Invalidated parse tree cache");
                }
                CacheLayer::All => {
                    self.cache.clear_all().await;
                    stats.query_invalidated = 1;
                    stats.embedding_invalidated = 1;
                    stats.context_invalidated = 1;
                    stats.parse_tree_invalidated = 1;
                    debug!("Invalidated all cache layers");
                }
            }
        }

        info!("Layer invalidation complete: {:?}", layers);

        Ok(stats)
    }

    /// Get a reference to the cache system.
    pub fn cache(&self) -> &Arc<CacheSystem> {
        &self.cache
    }
}

/// Cache layer identifier for selective invalidation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLayer {
    /// L1 query result cache
    L1Query,
    /// L2 embedding cache
    L2Embedding,
    /// L3 context bundle cache
    L3Context,
    /// Parse tree cache
    ParseTree,
    /// All cache layers
    All,
}

impl CacheLayer {
    /// Parse a cache layer from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "l1" | "query" | "l1_query" => Some(Self::L1Query),
            "l2" | "embedding" | "l2_embedding" => Some(Self::L2Embedding),
            "l3" | "context" | "l3_context" => Some(Self::L3Context),
            "parse" | "parse_tree" | "parsetree" => Some(Self::ParseTree),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    /// Get the name of the cache layer.
    pub fn name(&self) -> &'static str {
        match self {
            Self::L1Query => "L1 Query Cache",
            Self::L2Embedding => "L2 Embedding Cache",
            Self::L3Context => "L3 Context Cache",
            Self::ParseTree => "Parse Tree Cache",
            Self::All => "All Caches",
        }
    }
}

/// Statistics from a cache invalidation operation.
#[derive(Debug, Default, Clone)]
pub struct InvalidationStats {
    /// Number of query cache entries invalidated
    pub query_invalidated: usize,
    /// Number of embedding cache entries invalidated
    pub embedding_invalidated: usize,
    /// Number of context cache entries invalidated
    pub context_invalidated: usize,
    /// Number of parse tree cache entries invalidated
    pub parse_tree_invalidated: usize,
}

impl InvalidationStats {
    /// Create new invalidation statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total number of cache entries invalidated.
    pub fn total_invalidated(&self) -> usize {
        self.query_invalidated
            + self.embedding_invalidated
            + self.context_invalidated
            + self.parse_tree_invalidated
    }

    /// Check if any caches were invalidated.
    pub fn has_invalidations(&self) -> bool {
        self.total_invalidated() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;

    #[tokio::test]
    async fn test_cache_invalidator_creation() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        // Just verify we can create it
        assert!(Arc::ptr_eq(invalidator.cache(), &cache));
    }

    #[tokio::test]
    async fn test_on_file_changed() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        let stats = invalidator
            .on_file_changed(Path::new("test.rs"))
            .await
            .unwrap();

        assert!(stats.has_invalidations());
        assert!(stats.total_invalidated() > 0);
    }

    #[tokio::test]
    async fn test_on_reindex() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        let stats = invalidator.on_reindex(123).await.unwrap();

        assert_eq!(stats.query_invalidated, 1);
        assert_eq!(stats.embedding_invalidated, 1);
        assert_eq!(stats.context_invalidated, 1);
        assert_eq!(stats.parse_tree_invalidated, 1);
        assert_eq!(stats.total_invalidated(), 4);
    }

    #[tokio::test]
    async fn test_on_manual() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        let stats = invalidator.on_manual().await.unwrap();

        assert_eq!(stats.total_invalidated(), 4);
    }

    #[tokio::test]
    async fn test_on_pattern() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        let stats = invalidator.on_pattern("search_term").await.unwrap();

        assert!(stats.query_invalidated > 0);
    }

    #[tokio::test]
    async fn test_invalidate_specific_layers() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        let layers = vec![CacheLayer::L1Query, CacheLayer::L2Embedding];
        let stats = invalidator.invalidate_layers(&layers).await.unwrap();

        assert_eq!(stats.query_invalidated, 1);
        assert_eq!(stats.embedding_invalidated, 1);
        assert_eq!(stats.context_invalidated, 0);
        assert_eq!(stats.parse_tree_invalidated, 0);
    }

    #[tokio::test]
    async fn test_invalidate_all_layers() {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        let layers = vec![CacheLayer::All];
        let stats = invalidator.invalidate_layers(&layers).await.unwrap();

        assert_eq!(stats.total_invalidated(), 4);
    }

    #[test]
    fn test_cache_layer_from_str() {
        assert_eq!(CacheLayer::from_str("l1"), Some(CacheLayer::L1Query));
        assert_eq!(CacheLayer::from_str("query"), Some(CacheLayer::L1Query));
        assert_eq!(CacheLayer::from_str("l2"), Some(CacheLayer::L2Embedding));
        assert_eq!(
            CacheLayer::from_str("embedding"),
            Some(CacheLayer::L2Embedding)
        );
        assert_eq!(CacheLayer::from_str("l3"), Some(CacheLayer::L3Context));
        assert_eq!(CacheLayer::from_str("context"), Some(CacheLayer::L3Context));
        assert_eq!(CacheLayer::from_str("parse"), Some(CacheLayer::ParseTree));
        assert_eq!(CacheLayer::from_str("all"), Some(CacheLayer::All));
        assert_eq!(CacheLayer::from_str("invalid"), None);
    }

    #[test]
    fn test_cache_layer_name() {
        assert_eq!(CacheLayer::L1Query.name(), "L1 Query Cache");
        assert_eq!(CacheLayer::L2Embedding.name(), "L2 Embedding Cache");
        assert_eq!(CacheLayer::L3Context.name(), "L3 Context Cache");
        assert_eq!(CacheLayer::ParseTree.name(), "Parse Tree Cache");
        assert_eq!(CacheLayer::All.name(), "All Caches");
    }

    #[test]
    fn test_invalidation_stats() {
        let mut stats = InvalidationStats::new();

        assert_eq!(stats.total_invalidated(), 0);
        assert!(!stats.has_invalidations());

        stats.query_invalidated = 5;
        stats.context_invalidated = 3;

        assert_eq!(stats.total_invalidated(), 8);
        assert!(stats.has_invalidations());
    }
}
