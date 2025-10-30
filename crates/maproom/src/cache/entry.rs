//! Cache entry wrapper with TTL and access tracking.

use std::time::{Duration, Instant};

/// Cache entry wrapper with TTL support and access tracking.
///
/// Each cache entry tracks:
/// - The cached value
/// - Creation timestamp
/// - Last access timestamp
/// - Total access count
///
/// This enables both TTL-based expiration and LRU eviction strategies.
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// The cached value
    pub value: T,
    /// When the entry was created
    created_at: Instant,
    /// When the entry was last accessed
    last_accessed: Instant,
    /// Number of times accessed
    access_count: u64,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry with the given value.
    pub fn new(value: T) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// Get the age of this entry (time since creation).
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Check if this entry has expired based on the given TTL.
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.age() > ttl
    }

    /// Get the time since last access.
    pub fn time_since_access(&self) -> Duration {
        self.last_accessed.elapsed()
    }

    /// Get the access count.
    pub fn access_count(&self) -> u64 {
        self.access_count
    }

    /// Update access tracking (called on cache hit).
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }

    /// Get a reference to the value and update access tracking.
    pub fn get(&mut self) -> &T {
        self.touch();
        &self.value
    }

    /// Get a mutable reference to the value and update access tracking.
    pub fn get_mut(&mut self) -> &mut T {
        self.touch();
        &mut self.value
    }

    /// Clone the value and update access tracking.
    pub fn clone_value(&mut self) -> T
    where
        T: Clone,
    {
        self.touch();
        self.value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new(42);
        assert_eq!(entry.value, 42);
        assert_eq!(entry.access_count(), 0);
    }

    #[test]
    fn test_cache_entry_age() {
        let entry = CacheEntry::new("test");
        thread::sleep(Duration::from_millis(10));
        assert!(entry.age() >= Duration::from_millis(10));
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("test");

        // Not expired with long TTL
        assert!(!entry.is_expired(Duration::from_secs(60)));

        // Expired with zero TTL
        assert!(entry.is_expired(Duration::from_secs(0)));
    }

    #[test]
    fn test_cache_entry_access_tracking() {
        let mut entry = CacheEntry::new(100);
        assert_eq!(entry.access_count(), 0);

        entry.touch();
        assert_eq!(entry.access_count(), 1);

        entry.touch();
        assert_eq!(entry.access_count(), 2);
    }

    #[test]
    fn test_cache_entry_get() {
        let mut entry = CacheEntry::new(42);
        assert_eq!(entry.access_count(), 0);

        let value = entry.get();
        assert_eq!(*value, 42);
        assert_eq!(entry.access_count(), 1);

        let value = entry.get();
        assert_eq!(*value, 42);
        assert_eq!(entry.access_count(), 2);
    }

    #[test]
    fn test_cache_entry_get_mut() {
        let mut entry = CacheEntry::new(42);

        {
            let value = entry.get_mut();
            *value = 100;
        }

        assert_eq!(entry.value, 100);
        assert_eq!(entry.access_count(), 1);
    }

    #[test]
    fn test_cache_entry_clone_value() {
        let mut entry = CacheEntry::new(vec![1, 2, 3]);

        let cloned = entry.clone_value();
        assert_eq!(cloned, vec![1, 2, 3]);
        assert_eq!(entry.access_count(), 1);
    }

    #[test]
    fn test_time_since_access() {
        let mut entry = CacheEntry::new("test");
        entry.touch();

        thread::sleep(Duration::from_millis(10));
        assert!(entry.time_since_access() >= Duration::from_millis(10));
    }
}
