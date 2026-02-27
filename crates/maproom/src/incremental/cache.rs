//! In-memory hash cache for fast change detection.
//!
//! This module provides a simple HashMap-based cache for recently computed file hashes.
//! The cache enables fast change detection by avoiding redundant hashing and database queries.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::hash::ContentHash;

/// In-memory cache for file content hashes.
///
/// Stores recently computed hashes to enable fast change detection without
/// re-hashing files or querying the database.
///
/// # Design
/// - Simple HashMap implementation (no LRU eviction for Phase 1)
/// - Keyed by absolute file path
/// - Thread-safe access requires external synchronization (e.g., RwLock or Mutex)
///
/// # Future Enhancements
/// - LRU eviction policy for bounded memory usage
/// - TTL-based expiration for long-running watch mode
/// - Statistics tracking (hit rate, size, etc.)
pub struct HashCache {
    cache: HashMap<PathBuf, ContentHash>,
}

impl HashCache {
    /// Create a new empty hash cache.
    ///
    /// # Examples
    /// ```
    /// use maproom::incremental::HashCache;
    ///
    /// let cache = HashCache::new();
    /// ```
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Create a new hash cache with pre-allocated capacity.
    ///
    /// Use this when you know approximately how many files will be cached.
    ///
    /// # Arguments
    /// * `capacity` - Initial capacity for the internal HashMap
    ///
    /// # Examples
    /// ```
    /// use maproom::incremental::HashCache;
    ///
    /// let cache = HashCache::with_capacity(1000);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
        }
    }

    /// Get the cached hash for a file path.
    ///
    /// # Arguments
    /// * `path` - The file path to look up
    ///
    /// # Returns
    /// * `Some(&ContentHash)` - If the hash is cached
    /// * `None` - If the hash is not in the cache
    ///
    /// # Examples
    /// ```
    /// use std::path::Path;
    /// use maproom::incremental::HashCache;
    ///
    /// let cache = HashCache::new();
    /// let hash = cache.get(Path::new("src/main.rs"));
    /// assert!(hash.is_none());
    /// ```
    pub fn get(&self, path: &Path) -> Option<&ContentHash> {
        self.cache.get(path)
    }

    /// Insert or update a hash in the cache.
    ///
    /// # Arguments
    /// * `path` - The file path
    /// * `hash` - The content hash for the file
    ///
    /// # Examples
    /// ```
    /// use std::path::PathBuf;
    /// use maproom::incremental::{HashCache, FileHasher};
    ///
    /// let mut cache = HashCache::new();
    /// let path = PathBuf::from("src/main.rs");
    /// let hash = FileHasher::hash_bytes(b"fn main() {}");
    /// cache.insert(path, hash);
    /// ```
    pub fn insert(&mut self, path: PathBuf, hash: ContentHash) {
        self.cache.insert(path, hash);
    }

    /// Remove a path from the cache.
    ///
    /// Useful when a file is deleted or moved.
    ///
    /// # Arguments
    /// * `path` - The file path to remove
    ///
    /// # Returns
    /// * `Some(ContentHash)` - The hash that was removed
    /// * `None` - If the path was not in the cache
    pub fn remove(&mut self, path: &Path) -> Option<ContentHash> {
        self.cache.remove(path)
    }

    /// Check if a path is in the cache.
    ///
    /// # Arguments
    /// * `path` - The file path to check
    ///
    /// # Returns
    /// `true` if the path is cached, `false` otherwise
    pub fn contains(&self, path: &Path) -> bool {
        self.cache.contains_key(path)
    }

    /// Clear all entries from the cache.
    ///
    /// # Examples
    /// ```
    /// use maproom::incremental::HashCache;
    ///
    /// let mut cache = HashCache::new();
    /// cache.clear();
    /// assert_eq!(cache.len(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get the number of entries in the cache.
    ///
    /// # Returns
    /// The number of cached file hashes
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty.
    ///
    /// # Returns
    /// `true` if the cache has no entries
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get an iterator over the cache entries.
    ///
    /// Returns an iterator yielding `(&PathBuf, &ContentHash)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &ContentHash)> {
        self.cache.iter()
    }
}

impl Default for HashCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incremental::FileHasher;

    #[test]
    fn test_new_cache_is_empty() {
        let cache = HashCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_with_capacity() {
        let cache = HashCache::with_capacity(100);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_insert_and_get() {
        let mut cache = HashCache::new();
        let path = PathBuf::from("src/main.rs");
        let hash = FileHasher::hash_bytes(b"test content");

        cache.insert(path.clone(), hash);

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&path), Some(&hash));
    }

    #[test]
    fn test_get_nonexistent() {
        let cache = HashCache::new();
        let path = PathBuf::from("nonexistent.rs");

        assert_eq!(cache.get(&path), None);
    }

    #[test]
    fn test_insert_overwrites() {
        let mut cache = HashCache::new();
        let path = PathBuf::from("src/main.rs");
        let hash1 = FileHasher::hash_bytes(b"content 1");
        let hash2 = FileHasher::hash_bytes(b"content 2");

        cache.insert(path.clone(), hash1);
        cache.insert(path.clone(), hash2);

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&path), Some(&hash2));
    }

    #[test]
    fn test_remove() {
        let mut cache = HashCache::new();
        let path = PathBuf::from("src/main.rs");
        let hash = FileHasher::hash_bytes(b"test content");

        cache.insert(path.clone(), hash);
        let removed = cache.remove(&path);

        assert_eq!(removed, Some(hash));
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.get(&path), None);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut cache = HashCache::new();
        let path = PathBuf::from("nonexistent.rs");

        let removed = cache.remove(&path);
        assert_eq!(removed, None);
    }

    #[test]
    fn test_contains() {
        let mut cache = HashCache::new();
        let path = PathBuf::from("src/main.rs");
        let hash = FileHasher::hash_bytes(b"test content");

        assert!(!cache.contains(&path));

        cache.insert(path.clone(), hash);
        assert!(cache.contains(&path));
    }

    #[test]
    fn test_clear() {
        let mut cache = HashCache::new();
        let path1 = PathBuf::from("src/main.rs");
        let path2 = PathBuf::from("src/lib.rs");
        let hash1 = FileHasher::hash_bytes(b"content 1");
        let hash2 = FileHasher::hash_bytes(b"content 2");

        cache.insert(path1, hash1);
        cache.insert(path2, hash2);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_multiple_paths() {
        let mut cache = HashCache::new();
        let paths_and_hashes: Vec<(PathBuf, ContentHash)> = vec![
            (
                PathBuf::from("src/main.rs"),
                FileHasher::hash_bytes(b"main"),
            ),
            (PathBuf::from("src/lib.rs"), FileHasher::hash_bytes(b"lib")),
            (
                PathBuf::from("tests/test.rs"),
                FileHasher::hash_bytes(b"test"),
            ),
        ];

        for (path, hash) in &paths_and_hashes {
            cache.insert(path.clone(), *hash);
        }

        assert_eq!(cache.len(), 3);

        for (path, hash) in &paths_and_hashes {
            assert_eq!(cache.get(path), Some(hash));
        }
    }

    #[test]
    fn test_iter() {
        let mut cache = HashCache::new();
        let path1 = PathBuf::from("src/main.rs");
        let path2 = PathBuf::from("src/lib.rs");
        let hash1 = FileHasher::hash_bytes(b"main");
        let hash2 = FileHasher::hash_bytes(b"lib");

        cache.insert(path1.clone(), hash1);
        cache.insert(path2.clone(), hash2);

        let entries: Vec<_> = cache.iter().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_default() {
        let cache = HashCache::default();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }
}
