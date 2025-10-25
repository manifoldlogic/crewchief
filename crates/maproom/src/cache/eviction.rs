//! Cache eviction policies and strategies.
//!
//! Provides multiple eviction strategies to maintain cache effectiveness:
//! - **LRU**: Least Recently Used (default for all caches)
//! - **TTL**: Time-To-Live based expiration
//! - **Size**: Memory-based eviction when approaching limits
//! - **AccessCount**: Evict entries with low access counts

use super::entry::CacheEntry;
use std::time::Duration;
use tracing::debug;

/// Cache eviction policy.
///
/// Determines which entries should be removed from the cache
/// to maintain effectiveness and stay within resource limits.
#[derive(Debug, Clone)]
pub enum EvictionPolicy {
    /// Evict least recently used entries (handled by LruCache)
    Lru,
    /// Evict entries older than the specified TTL
    Ttl(Duration),
    /// Evict entries when total size exceeds the limit (in bytes)
    Size(usize),
    /// Evict entries accessed fewer than the specified count
    AccessCount(u64),
}

/// Eviction strategy for a cache layer.
///
/// Provides methods to determine which entries should be evicted
/// based on various criteria.
pub struct EvictionStrategy {
    /// Primary eviction policy
    policy: EvictionPolicy,
    /// Memory limit in bytes (500MB default, aligned with PERF_OPT-4001)
    memory_limit: usize,
}

impl EvictionStrategy {
    /// Create a new eviction strategy with the given policy.
    pub fn new(policy: EvictionPolicy) -> Self {
        Self {
            policy,
            memory_limit: 500 * 1024 * 1024, // 500MB default
        }
    }

    /// Create an eviction strategy with a custom memory limit.
    pub fn with_memory_limit(policy: EvictionPolicy, memory_limit: usize) -> Self {
        Self {
            policy,
            memory_limit,
        }
    }

    /// Get the eviction policy.
    pub fn policy(&self) -> &EvictionPolicy {
        &self.policy
    }

    /// Get the memory limit in bytes.
    pub fn memory_limit(&self) -> usize {
        self.memory_limit
    }

    /// Check if an entry should be evicted based on TTL.
    pub fn should_evict_by_ttl<T>(&self, entry: &CacheEntry<T>, ttl: Duration) -> bool {
        if ttl.is_zero() {
            // TTL of 0 means never expire
            return false;
        }
        entry.is_expired(ttl)
    }

    /// Check if an entry should be evicted based on access count.
    pub fn should_evict_by_access<T>(&self, entry: &CacheEntry<T>, min_count: u64) -> bool {
        entry.access_count() < min_count
    }

    /// Check if memory-based eviction should occur.
    pub fn should_evict_by_memory(&self, current_size: usize) -> bool {
        current_size >= self.memory_limit
    }
}

impl Default for EvictionStrategy {
    fn default() -> Self {
        Self::new(EvictionPolicy::Lru)
    }
}

/// Eviction statistics for monitoring.
#[derive(Debug, Default, Clone)]
pub struct EvictionStats {
    /// Number of TTL-based evictions
    pub ttl_evictions: u64,
    /// Number of size-based evictions
    pub size_evictions: u64,
    /// Number of access-count-based evictions
    pub access_evictions: u64,
    /// Number of LRU evictions
    pub lru_evictions: u64,
}

impl EvictionStats {
    /// Create new eviction statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total evictions across all policies.
    pub fn total_evictions(&self) -> u64 {
        self.ttl_evictions + self.size_evictions + self.access_evictions + self.lru_evictions
    }

    /// Record a TTL-based eviction.
    pub fn record_ttl_eviction(&mut self) {
        self.ttl_evictions += 1;
        debug!("Recorded TTL eviction (total: {})", self.ttl_evictions);
    }

    /// Record a size-based eviction.
    pub fn record_size_eviction(&mut self) {
        self.size_evictions += 1;
        debug!("Recorded size eviction (total: {})", self.size_evictions);
    }

    /// Record an access-count-based eviction.
    pub fn record_access_eviction(&mut self) {
        self.access_evictions += 1;
        debug!("Recorded access eviction (total: {})", self.access_evictions);
    }

    /// Record an LRU eviction.
    pub fn record_lru_eviction(&mut self) {
        self.lru_evictions += 1;
        debug!("Recorded LRU eviction (total: {})", self.lru_evictions);
    }

    /// Reset all statistics.
    pub fn reset(&mut self) {
        self.ttl_evictions = 0;
        self.size_evictions = 0;
        self.access_evictions = 0;
        self.lru_evictions = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eviction_strategy_ttl() {
        let strategy = EvictionStrategy::new(EvictionPolicy::Ttl(Duration::from_secs(60)));

        // Test with long TTL - should not evict immediately
        let entry = CacheEntry::new("test");
        assert!(!strategy.should_evict_by_ttl(&entry, Duration::from_secs(60)));

        // Create an entry and wait long enough to guarantee expiration
        let old_entry = CacheEntry::new("old_test");
        std::thread::sleep(Duration::from_secs(1));

        // Entry is now >1000ms old, TTL is 500ms, so should be evicted
        assert!(strategy.should_evict_by_ttl(&old_entry, Duration::from_millis(500)));
    }

    #[test]
    fn test_eviction_strategy_access_count() {
        let strategy = EvictionStrategy::new(EvictionPolicy::AccessCount(5));
        let mut entry = CacheEntry::new("test");

        // New entry with 0 accesses should be evicted if min is 5
        assert!(strategy.should_evict_by_access(&entry, 5));

        // After 5 accesses, should not be evicted
        for _ in 0..5 {
            entry.touch();
        }
        assert!(!strategy.should_evict_by_access(&entry, 5));
    }

    #[test]
    fn test_eviction_strategy_memory() {
        let strategy = EvictionStrategy::with_memory_limit(
            EvictionPolicy::Size(100 * 1024 * 1024), // 100MB
            100 * 1024 * 1024,
        );

        // Under limit
        assert!(!strategy.should_evict_by_memory(50 * 1024 * 1024));

        // At limit
        assert!(strategy.should_evict_by_memory(100 * 1024 * 1024));

        // Over limit
        assert!(strategy.should_evict_by_memory(150 * 1024 * 1024));
    }

    #[test]
    fn test_eviction_strategy_zero_ttl() {
        let strategy = EvictionStrategy::new(EvictionPolicy::Ttl(Duration::from_secs(0)));
        let entry = CacheEntry::new("test");

        // TTL of 0 means never expire
        assert!(!strategy.should_evict_by_ttl(&entry, Duration::from_secs(0)));
    }

    #[test]
    fn test_eviction_stats() {
        let mut stats = EvictionStats::new();

        assert_eq!(stats.total_evictions(), 0);

        stats.record_ttl_eviction();
        stats.record_size_eviction();
        stats.record_access_eviction();
        stats.record_lru_eviction();

        assert_eq!(stats.total_evictions(), 4);
        assert_eq!(stats.ttl_evictions, 1);
        assert_eq!(stats.size_evictions, 1);
        assert_eq!(stats.access_evictions, 1);
        assert_eq!(stats.lru_evictions, 1);

        stats.reset();
        assert_eq!(stats.total_evictions(), 0);
    }

    #[test]
    fn test_default_eviction_strategy() {
        let strategy = EvictionStrategy::default();
        assert_eq!(strategy.memory_limit(), 500 * 1024 * 1024); // 500MB
    }
}
