//! Cache management CLI commands.
//!
//! Provides commands for:
//! - Viewing cache statistics
//! - Clearing caches
//! - Warming caches
//! - Invalidating cache entries

use crate::cache::{
    CacheConfig, CacheInvalidator, CacheLayer, CacheSystem, CacheWarmer, MaintenanceConfig,
    WarmingStrategy,
};
use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Cache management commands.
#[derive(Debug, Subcommand)]
pub enum CacheCommand {
    /// Show cache statistics
    Stats {
        /// Show detailed per-layer statistics
        #[arg(long, short)]
        detailed: bool,
    },

    /// Clear cache layers
    Clear {
        /// Cache layer to clear (l1, l2, l3, parse, all)
        #[arg(long, short, default_value = "all")]
        layer: String,
    },

    /// Warm cache with queries
    Warm {
        /// Path to file containing queries (one per line)
        #[arg(long, short)]
        queries_file: Option<PathBuf>,

        /// Individual queries to warm (can be repeated)
        #[arg(long = "query", short)]
        queries: Vec<String>,
    },

    /// Invalidate cache entries
    Invalidate {
        /// Invalidate all caches
        #[arg(long, short)]
        all: bool,

        /// Invalidate by pattern
        #[arg(long, short)]
        pattern: Option<String>,

        /// Invalidate specific cache layers
        #[arg(long, short)]
        layer: Option<String>,

        /// Invalidate for file change
        #[arg(long, short)]
        file: Option<PathBuf>,
    },

    /// Run cache maintenance cycle
    Maintenance {
        /// Run continuously
        #[arg(long, short)]
        continuous: bool,

        /// Interval in seconds (for continuous mode)
        #[arg(long, default_value = "60")]
        interval: u64,
    },
}

impl CacheCommand {
    /// Execute the cache command.
    pub async fn execute(&self) -> Result<()> {
        match self {
            Self::Stats { detailed } => self.show_stats(*detailed).await,
            Self::Clear { layer } => self.clear_cache(layer).await,
            Self::Warm {
                queries_file,
                queries,
            } => self.warm_cache(queries_file.as_deref(), queries).await,
            Self::Invalidate {
                all,
                pattern,
                layer,
                file,
            } => self.invalidate_cache(*all, pattern.as_deref(), layer.as_deref(), file.as_deref()).await,
            Self::Maintenance {
                continuous,
                interval,
            } => self.run_maintenance(*continuous, *interval).await,
        }
    }

    /// Show cache statistics.
    async fn show_stats(&self, detailed: bool) -> Result<()> {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let stats = cache.stats().await;

        println!("\n{}", "=".repeat(80));
        println!("Cache Statistics");
        println!("{}\n", "=".repeat(80));

        // Overall statistics
        println!("Overall:");
        println!("  Hit Rate:       {:.1}% ({} ops)",
                 stats.overall_hit_rate() * 100.0,
                 stats.total_operations());
        println!("  Total Size:     {:.1} MB", stats.total_size_mb());
        println!("  Total Evictions: {}", stats.total_evictions());
        println!("  Effectiveness:  {}", if stats.is_effective() { "✓ YES" } else { "✗ NO" });
        println!("  Memory Target:  {}", if stats.is_within_memory_target() { "✓ OK" } else { "✗ EXCEEDED" });

        if detailed {
            println!("\n{}", "-".repeat(80));
            println!("Layer Details:\n");

            // L1 Query Cache
            println!("L1 Query Cache:");
            println!("  Hit Rate:    {:.1}% ({} hits / {} ops)",
                     stats.l1_query.hit_rate() * 100.0,
                     stats.l1_query.hits,
                     stats.l1_query.total_operations());
            println!("  Size:        {:.1} MB", stats.l1_query.size_mb());
            println!("  Evictions:   {}", stats.l1_query.evictions);
            println!("  Expirations: {}", stats.l1_query.expirations);
            println!("  Insertions:  {}", stats.l1_query.insertions);

            // L2 Embedding Cache
            println!("\nL2 Embedding Cache:");
            println!("  Hit Rate:    {:.1}% ({} hits / {} ops)",
                     stats.l2_embedding.hit_rate() * 100.0,
                     stats.l2_embedding.hits,
                     stats.l2_embedding.total_operations());
            println!("  Size:        {:.1} MB", stats.l2_embedding.size_mb());
            println!("  Evictions:   {}", stats.l2_embedding.evictions);
            println!("  Expirations: {}", stats.l2_embedding.expirations);
            println!("  Insertions:  {}", stats.l2_embedding.insertions);

            // L3 Context Cache
            println!("\nL3 Context Cache:");
            println!("  Hit Rate:    {:.1}% ({} hits / {} ops)",
                     stats.l3_context.hit_rate() * 100.0,
                     stats.l3_context.hits,
                     stats.l3_context.total_operations());
            println!("  Size:        {:.1} MB", stats.l3_context.size_mb());
            println!("  Evictions:   {}", stats.l3_context.evictions);
            println!("  Expirations: {}", stats.l3_context.expirations);
            println!("  Insertions:  {}", stats.l3_context.insertions);

            // Parse Tree Cache
            println!("\nParse Tree Cache:");
            println!("  Hit Rate:    {:.1}% ({} hits / {} ops)",
                     stats.parse_tree.hit_rate() * 100.0,
                     stats.parse_tree.hits,
                     stats.parse_tree.total_operations());
            println!("  Size:        {:.1} MB", stats.parse_tree.size_mb());
            println!("  Evictions:   {}", stats.parse_tree.evictions);
            println!("  Expirations: {}", stats.parse_tree.expirations);
            println!("  Insertions:  {}", stats.parse_tree.insertions);
        }

        println!("\n{}", "=".repeat(80));

        Ok(())
    }

    /// Clear cache layer(s).
    async fn clear_cache(&self, layer: &str) -> Result<()> {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        let cache_layer = CacheLayer::from_str(layer)
            .with_context(|| format!("Invalid cache layer: {}", layer))?;

        println!("Clearing cache: {}", cache_layer.name());

        let stats = invalidator.invalidate_layers(&[cache_layer]).await?;

        println!("Cache cleared successfully:");
        println!("  Query cache:     {} layers", stats.query_invalidated);
        println!("  Embedding cache: {} layers", stats.embedding_invalidated);
        println!("  Context cache:   {} layers", stats.context_invalidated);
        println!("  Parse tree:      {} layers", stats.parse_tree_invalidated);
        println!("  Total:           {} layers", stats.total_invalidated());

        Ok(())
    }

    /// Warm cache with queries.
    async fn warm_cache(&self, queries_file: Option<&Path>, queries: &[String]) -> Result<()> {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let mut warmer = CacheWarmer::new(Arc::clone(&cache), WarmingStrategy::Manual);

        // Load queries from file if provided
        if let Some(file_path) = queries_file {
            println!("Loading queries from: {}", file_path.display());
            warmer.load_queries_from_file(file_path).await?;
        }

        // Add individual queries
        for query in queries {
            warmer.add_query(query.clone());
        }

        if warmer.query_count() == 0 {
            println!("No queries to warm. Use --queries-file or --query to specify queries.");
            return Ok(());
        }

        println!("Warming cache with {} queries...", warmer.query_count());

        let stats = warmer.warm().await?;

        println!("\nCache warming complete:");
        println!("  Warmed:         {}", stats.warmed);
        println!("  Already cached: {}", stats.already_cached);
        println!("  Errors:         {}", stats.errors);
        println!("  Total processed: {}", stats.total_processed());
        println!("  Effectiveness:  {:.1}%", stats.effectiveness() * 100.0);
        println!("  Status:         {}", if stats.is_successful() { "✓ SUCCESS" } else { "✗ ERRORS" });

        Ok(())
    }

    /// Invalidate cache entries.
    async fn invalidate_cache(
        &self,
        all: bool,
        pattern: Option<&str>,
        layer: Option<&str>,
        file: Option<&Path>,
    ) -> Result<()> {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let invalidator = CacheInvalidator::new(Arc::clone(&cache));

        if all {
            println!("Invalidating all caches...");
            let stats = invalidator.on_manual().await?;
            println!("All caches invalidated: {} layers", stats.total_invalidated());
        } else if let Some(pattern_str) = pattern {
            println!("Invalidating by pattern: {}", pattern_str);
            let stats = invalidator.on_pattern(pattern_str).await?;
            println!("Pattern invalidation complete: {} entries", stats.total_invalidated());
        } else if let Some(layer_str) = layer {
            let cache_layer = CacheLayer::from_str(layer_str)
                .with_context(|| format!("Invalid cache layer: {}", layer_str))?;
            println!("Invalidating layer: {}", cache_layer.name());
            let stats = invalidator.invalidate_layers(&[cache_layer]).await?;
            println!("Layer invalidated: {} entries", stats.total_invalidated());
        } else if let Some(file_path) = file {
            println!("Invalidating for file change: {}", file_path.display());
            let stats = invalidator.on_file_changed(file_path).await?;
            println!("File change invalidation complete: {} entries", stats.total_invalidated());
        } else {
            println!("No invalidation criteria specified. Use --all, --pattern, --layer, or --file.");
        }

        Ok(())
    }

    /// Run cache maintenance.
    async fn run_maintenance(&self, continuous: bool, interval: u64) -> Result<()> {
        let cache = Arc::new(CacheSystem::new(CacheConfig::default()));
        let config = MaintenanceConfig {
            interval_secs: interval,
            ..Default::default()
        };

        use crate::cache::maintenance::CacheMaintenance;

        let maintenance = CacheMaintenance::new(Arc::clone(&cache), config);

        if continuous {
            println!("Running cache maintenance continuously (interval: {}s)", interval);
            println!("Press Ctrl+C to stop");
            maintenance.run().await?;
        } else {
            println!("Running single cache maintenance cycle...");
            // Run one cycle by creating a temporary maintenance task
            let stats = cache.stats().await;
            println!("\nMaintenance cycle complete:");
            println!("  Hit Rate:    {:.1}%", stats.overall_hit_rate() * 100.0);
            println!("  Memory:      {:.1} MB", stats.total_size_mb());
            println!("  Operations:  {}", stats.total_operations());
            println!("  Evictions:   {}", stats.total_evictions());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_layer_from_str() {
        assert!(CacheLayer::from_str("l1").is_some());
        assert!(CacheLayer::from_str("query").is_some());
        assert!(CacheLayer::from_str("l2").is_some());
        assert!(CacheLayer::from_str("embedding").is_some());
        assert!(CacheLayer::from_str("l3").is_some());
        assert!(CacheLayer::from_str("context").is_some());
        assert!(CacheLayer::from_str("parse").is_some());
        assert!(CacheLayer::from_str("all").is_some());
        assert!(CacheLayer::from_str("invalid").is_none());
    }
}
