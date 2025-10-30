//! String interning for memory deduplication.
//!
//! String interning deduplicates repeated strings by storing only one copy
//! of each unique string value and returning Arc references to it.
//!
//! # Benefits
//!
//! - **Memory Reduction**: Shared strings eliminate duplication
//! - **Fast Comparison**: Arc pointer equality is O(1)
//! - **Thread-Safe**: Arc provides safe shared ownership
//!
//! # Use Cases
//!
//! - File paths (many chunks from same file)
//! - Symbol names (functions, classes, variables)
//! - Language identifiers (limited set: ts/js/rs/py/md/json/yaml/toml)
//! - Repository names and worktree paths
//!
//! # Performance
//!
//! - Intern: O(1) average (HashMap lookup)
//! - Compare: O(1) (pointer equality)
//! - Memory: ~48 bytes overhead per unique string (HashMap entry + Arc)
//!
//! # Example
//!
//! ```no_run
//! use crewchief_maproom::memory::StringInterner;
//! use std::sync::Arc;
//!
//! let interner = StringInterner::new();
//!
//! let path1 = interner.intern("src/main.rs");
//! let path2 = interner.intern("src/main.rs");
//!
//! // Same Arc pointer - no duplication
//! assert!(Arc::ptr_eq(&path1, &path2));
//!
//! // Stats show deduplication
//! let stats = interner.stats();
//! assert_eq!(stats.unique_strings, 1);
//! assert_eq!(stats.total_interns, 2);
//! ```

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

lazy_static! {
    /// Global string interner instance.
    ///
    /// This is used for interning strings that are shared across the entire
    /// application, such as language identifiers and common paths.
    static ref GLOBAL_INTERNER: Arc<StringInterner> = Arc::new(StringInterner::new());
}

/// Get the global StringInterner instance.
///
/// Use this for application-wide string interning.
pub fn get_global_interner() -> Arc<StringInterner> {
    GLOBAL_INTERNER.clone()
}

/// String interner that deduplicates strings using Arc.
///
/// StringInterner maintains a HashMap of unique strings and returns
/// Arc references to them, ensuring each unique string is stored only once.
///
/// # Thread Safety
///
/// This struct uses RwLock for thread-safe concurrent access:
/// - Multiple threads can read simultaneously
/// - Writes are exclusive
///
/// # Memory Model
///
/// ```text
/// HashMap<String, Arc<str>>
///    │
///    ├─ "src/main.rs" → Arc<str> ────┐
///    │                               │
///    ├─ "main" → Arc<str> ────────┐  │
///    │                            │  │
///    └─ "ts" → Arc<str> ─────┐    │  │
///                            │    │  │
/// Multiple references can point to same Arc:
///                            ↓    ↓  ↓
///                         [Shared Memory]
/// ```
pub struct StringInterner {
    /// Map from string values to Arc references
    map: RwLock<HashMap<String, Arc<str>>>,

    /// Statistics counters
    stats: RwLock<InternerStats>,
}

impl StringInterner {
    /// Create a new StringInterner.
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
            stats: RwLock::new(InternerStats::default()),
        }
    }

    /// Create a new StringInterner with expected capacity.
    ///
    /// Pre-allocating capacity can improve performance when the number
    /// of unique strings is known in advance.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: RwLock::new(HashMap::with_capacity(capacity)),
            stats: RwLock::new(InternerStats::default()),
        }
    }

    /// Intern a string, returning an Arc reference.
    ///
    /// If the string has been interned before, returns the existing Arc.
    /// Otherwise, creates a new Arc and stores it.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::memory::StringInterner;
    ///
    /// let interner = StringInterner::new();
    /// let s1 = interner.intern("hello");
    /// let s2 = interner.intern("hello");
    ///
    /// assert!(std::sync::Arc::ptr_eq(&s1, &s2));
    /// ```
    pub fn intern(&self, s: &str) -> Arc<str> {
        // Fast path: check if already interned (read lock)
        {
            let map = self.map.read().unwrap();
            if let Some(arc) = map.get(s) {
                let mut stats = self.stats.write().unwrap();
                stats.total_interns += 1;
                stats.hits += 1;
                return arc.clone();
            }
        }

        // Slow path: need to insert (write lock)
        let mut map = self.map.write().unwrap();

        // Double-check in case another thread inserted while we waited
        if let Some(arc) = map.get(s) {
            let mut stats = self.stats.write().unwrap();
            stats.total_interns += 1;
            stats.hits += 1;
            return arc.clone();
        }

        // Create new Arc and insert
        let arc: Arc<str> = Arc::from(s);
        map.insert(s.to_string(), arc.clone());

        let mut stats = self.stats.write().unwrap();
        stats.total_interns += 1;
        stats.misses += 1;
        stats.unique_strings = map.len();

        arc
    }

    /// Intern a String, consuming it.
    ///
    /// This is more efficient than intern() when you already have a String,
    /// as it avoids an extra allocation on cache miss.
    pub fn intern_owned(&self, s: String) -> Arc<str> {
        // Fast path: check if already interned
        {
            let map = self.map.read().unwrap();
            if let Some(arc) = map.get(s.as_str()) {
                let mut stats = self.stats.write().unwrap();
                stats.total_interns += 1;
                stats.hits += 1;
                return arc.clone();
            }
        }

        // Slow path: insert owned string
        let mut map = self.map.write().unwrap();

        // Double-check
        if let Some(arc) = map.get(s.as_str()) {
            let mut stats = self.stats.write().unwrap();
            stats.total_interns += 1;
            stats.hits += 1;
            return arc.clone();
        }

        // Create Arc from the owned String
        let arc: Arc<str> = Arc::from(s.as_str());
        map.insert(s, arc.clone());

        let mut stats = self.stats.write().unwrap();
        stats.total_interns += 1;
        stats.misses += 1;
        stats.unique_strings = map.len();

        arc
    }

    /// Get interning statistics.
    pub fn stats(&self) -> InternerStats {
        self.stats.read().unwrap().clone()
    }

    /// Get the number of unique strings interned.
    pub fn len(&self) -> usize {
        self.map.read().unwrap().len()
    }

    /// Check if the interner is empty.
    pub fn is_empty(&self) -> bool {
        self.map.read().unwrap().is_empty()
    }

    /// Clear all interned strings.
    ///
    /// This removes all entries from the interner. Existing Arc references
    /// remain valid but are no longer shared through the interner.
    pub fn clear(&self) {
        let mut map = self.map.write().unwrap();
        map.clear();

        let mut stats = self.stats.write().unwrap();
        stats.unique_strings = 0;
    }

    /// Estimate memory usage in bytes.
    ///
    /// This is an approximation based on:
    /// - HashMap overhead: ~48 bytes per entry
    /// - String data: actual string bytes
    /// - Arc overhead: ~16 bytes per Arc
    pub fn estimated_memory_bytes(&self) -> usize {
        let map = self.map.read().unwrap();
        let mut total = 0;

        for (key, _) in map.iter() {
            // HashMap entry overhead
            total += 48;
            // String key data
            total += key.len();
            // Arc overhead
            total += 16;
        }

        total
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for string interning.
#[derive(Debug, Clone, Default)]
pub struct InternerStats {
    /// Total number of intern() calls
    pub total_interns: usize,

    /// Number of cache hits (string was already interned)
    pub hits: usize,

    /// Number of cache misses (new string interned)
    pub misses: usize,

    /// Number of unique strings currently interned
    pub unique_strings: usize,
}

impl InternerStats {
    /// Calculate the hit rate (0.0-1.0).
    pub fn hit_rate(&self) -> f64 {
        if self.total_interns == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_interns as f64
        }
    }

    /// Calculate the deduplication ratio.
    ///
    /// This shows how many references share each unique string on average.
    /// Higher is better (more deduplication).
    pub fn deduplication_ratio(&self) -> f64 {
        if self.unique_strings == 0 {
            0.0
        } else {
            self.total_interns as f64 / self.unique_strings as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() {
        let interner = StringInterner::new();

        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");
        let s3 = interner.intern("world");

        // Same string should return same Arc
        assert!(Arc::ptr_eq(&s1, &s2));

        // Different string should return different Arc
        assert!(!Arc::ptr_eq(&s1, &s3));

        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn test_intern_owned() {
        let interner = StringInterner::new();

        let s1 = interner.intern_owned("hello".to_string());
        let s2 = interner.intern("hello");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_stats() {
        let interner = StringInterner::new();

        interner.intern("a");
        interner.intern("a");
        interner.intern("b");
        interner.intern("a");

        let stats = interner.stats();
        assert_eq!(stats.total_interns, 4);
        assert_eq!(stats.hits, 2); // "a" hit twice
        assert_eq!(stats.misses, 2); // "a" and "b" missed once each
        assert_eq!(stats.unique_strings, 2);
        assert_eq!(stats.hit_rate(), 0.5);
        assert_eq!(stats.deduplication_ratio(), 2.0);
    }

    #[test]
    fn test_clear() {
        let interner = StringInterner::new();

        let s1 = interner.intern("hello");
        assert_eq!(interner.len(), 1);

        interner.clear();
        assert_eq!(interner.len(), 0);

        // Old Arc is still valid
        assert_eq!(&*s1, "hello");

        // New intern creates new Arc
        let s2 = interner.intern("hello");
        assert!(!Arc::ptr_eq(&s1, &s2));
    }

    #[test]
    fn test_with_capacity() {
        let interner = StringInterner::with_capacity(100);
        assert_eq!(interner.len(), 0);

        for i in 0..50 {
            interner.intern(&format!("string_{}", i));
        }

        assert_eq!(interner.len(), 50);
    }

    #[test]
    fn test_memory_estimation() {
        let interner = StringInterner::new();

        interner.intern("short");
        interner.intern("a_longer_string");

        let memory = interner.estimated_memory_bytes();
        assert!(memory > 0);
        assert!(memory > 100); // Should account for overhead and string data
    }

    #[test]
    fn test_global_interner() {
        let interner1 = get_global_interner();
        let interner2 = get_global_interner();

        assert!(Arc::ptr_eq(&interner1, &interner2));

        let s1 = interner1.intern("global");
        let s2 = interner2.intern("global");

        assert!(Arc::ptr_eq(&s1, &s2));
    }

    #[test]
    fn test_empty_string() {
        let interner = StringInterner::new();

        let s1 = interner.intern("");
        let s2 = interner.intern("");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(&*s1, "");
    }

    #[test]
    fn test_unicode_strings() {
        let interner = StringInterner::new();

        let s1 = interner.intern("你好世界");
        let s2 = interner.intern("你好世界");
        let s3 = interner.intern("こんにちは");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert!(!Arc::ptr_eq(&s1, &s3));
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let interner = Arc::new(StringInterner::new());
        let mut handles = vec![];

        for i in 0..10 {
            let interner_clone = interner.clone();
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    interner_clone.intern(&format!("thread_{}_{}", i, j % 10));
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have deduplicated many strings
        let stats = interner.stats();
        assert!(stats.unique_strings <= 100); // 10 threads * 10 unique strings each
        assert!(stats.total_interns == 1000); // 10 threads * 100 calls each
    }

    #[test]
    fn test_deduplication_ratio() {
        let interner = StringInterner::new();

        // Intern same string multiple times
        for _ in 0..100 {
            interner.intern("repeated");
        }

        let stats = interner.stats();
        assert_eq!(stats.unique_strings, 1);
        assert_eq!(stats.deduplication_ratio(), 100.0);
    }
}
