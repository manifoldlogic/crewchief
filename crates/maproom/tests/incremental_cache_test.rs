//! Unit tests for hash cache functionality.
//!
//! Tests cover:
//! - Cache hit/miss behavior
//! - Insert, get, remove operations
//! - Cache clearing
//! - Multiple entries
//! - Iterator functionality

use crewchief_maproom::incremental::{FileHasher, HashCache};
use std::path::PathBuf;

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
fn test_insert_and_get_single_entry() {
    let mut cache = HashCache::new();
    let path = PathBuf::from("src/main.rs");
    let hash = FileHasher::hash_bytes(b"fn main() {}");

    cache.insert(path.clone(), hash);

    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());
    assert_eq!(cache.get(&path), Some(&hash));
}

#[test]
fn test_get_nonexistent_entry() {
    let cache = HashCache::new();
    let path = PathBuf::from("nonexistent.rs");

    assert_eq!(cache.get(&path), None);
}

#[test]
fn test_cache_miss() {
    let mut cache = HashCache::new();
    let path1 = PathBuf::from("src/main.rs");
    let path2 = PathBuf::from("src/lib.rs");
    let hash1 = FileHasher::hash_bytes(b"main");

    cache.insert(path1.clone(), hash1);

    // Cache hit for path1
    assert!(cache.get(&path1).is_some());

    // Cache miss for path2
    assert!(cache.get(&path2).is_none());
}

#[test]
fn test_cache_hit() {
    let mut cache = HashCache::new();
    let path = PathBuf::from("src/main.rs");
    let hash = FileHasher::hash_bytes(b"fn main() {}");

    cache.insert(path.clone(), hash);

    // Multiple gets should all hit
    assert_eq!(cache.get(&path), Some(&hash));
    assert_eq!(cache.get(&path), Some(&hash));
    assert_eq!(cache.get(&path), Some(&hash));
}

#[test]
fn test_insert_overwrites_existing() {
    let mut cache = HashCache::new();
    let path = PathBuf::from("src/main.rs");
    let hash1 = FileHasher::hash_bytes(b"version 1");
    let hash2 = FileHasher::hash_bytes(b"version 2");

    cache.insert(path.clone(), hash1);
    assert_eq!(cache.get(&path), Some(&hash1));

    cache.insert(path.clone(), hash2);
    assert_eq!(cache.get(&path), Some(&hash2));
    assert_eq!(cache.len(), 1, "Overwrite should not increase size");
}

#[test]
fn test_remove_existing_entry() {
    let mut cache = HashCache::new();
    let path = PathBuf::from("src/main.rs");
    let hash = FileHasher::hash_bytes(b"fn main() {}");

    cache.insert(path.clone(), hash);
    assert_eq!(cache.len(), 1);

    let removed = cache.remove(&path);
    assert_eq!(removed, Some(hash));
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
    assert_eq!(cache.get(&path), None);
}

#[test]
fn test_remove_nonexistent_entry() {
    let mut cache = HashCache::new();
    let path = PathBuf::from("nonexistent.rs");

    let removed = cache.remove(&path);
    assert_eq!(removed, None);
}

#[test]
fn test_contains() {
    let mut cache = HashCache::new();
    let path = PathBuf::from("src/main.rs");
    let hash = FileHasher::hash_bytes(b"fn main() {}");

    assert!(!cache.contains(&path));

    cache.insert(path.clone(), hash);
    assert!(cache.contains(&path));

    cache.remove(&path);
    assert!(!cache.contains(&path));
}

#[test]
fn test_clear() {
    let mut cache = HashCache::new();

    // Add multiple entries
    let entries = vec![
        (PathBuf::from("src/main.rs"), FileHasher::hash_bytes(b"main")),
        (PathBuf::from("src/lib.rs"), FileHasher::hash_bytes(b"lib")),
        (
            PathBuf::from("tests/test.rs"),
            FileHasher::hash_bytes(b"test"),
        ),
    ];

    for (path, hash) in &entries {
        cache.insert(path.clone(), *hash);
    }

    assert_eq!(cache.len(), 3);

    cache.clear();

    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());

    for (path, _) in &entries {
        assert_eq!(cache.get(path), None);
    }
}

#[test]
fn test_multiple_entries() {
    let mut cache = HashCache::new();

    let entries = vec![
        (PathBuf::from("src/main.rs"), FileHasher::hash_bytes(b"main")),
        (PathBuf::from("src/lib.rs"), FileHasher::hash_bytes(b"lib")),
        (
            PathBuf::from("src/utils.rs"),
            FileHasher::hash_bytes(b"utils"),
        ),
        (
            PathBuf::from("tests/test.rs"),
            FileHasher::hash_bytes(b"test"),
        ),
    ];

    for (path, hash) in &entries {
        cache.insert(path.clone(), *hash);
    }

    assert_eq!(cache.len(), entries.len());

    // All entries should be retrievable
    for (path, hash) in &entries {
        assert_eq!(cache.get(path), Some(hash));
        assert!(cache.contains(path));
    }
}

#[test]
fn test_different_paths_same_hash() {
    let mut cache = HashCache::new();
    let path1 = PathBuf::from("src/file1.rs");
    let path2 = PathBuf::from("src/file2.rs");
    let hash = FileHasher::hash_bytes(b"same content");

    cache.insert(path1.clone(), hash);
    cache.insert(path2.clone(), hash);

    assert_eq!(cache.len(), 2);
    assert_eq!(cache.get(&path1), Some(&hash));
    assert_eq!(cache.get(&path2), Some(&hash));
}

#[test]
fn test_iter() {
    let mut cache = HashCache::new();

    let entries = vec![
        (PathBuf::from("src/main.rs"), FileHasher::hash_bytes(b"main")),
        (PathBuf::from("src/lib.rs"), FileHasher::hash_bytes(b"lib")),
        (
            PathBuf::from("tests/test.rs"),
            FileHasher::hash_bytes(b"test"),
        ),
    ];

    for (path, hash) in &entries {
        cache.insert(path.clone(), *hash);
    }

    let collected: Vec<_> = cache.iter().collect();
    assert_eq!(collected.len(), entries.len());

    // Verify all entries are in the iterator
    for (path, hash) in &entries {
        assert!(collected.contains(&(path, hash)));
    }
}

#[test]
fn test_iter_empty_cache() {
    let cache = HashCache::new();
    let collected: Vec<_> = cache.iter().collect();
    assert_eq!(collected.len(), 0);
}

#[test]
fn test_default_trait() {
    let cache = HashCache::default();
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_path_normalization() {
    // Test that different path representations are treated as different keys
    let mut cache = HashCache::new();

    let path1 = PathBuf::from("src/main.rs");
    let path2 = PathBuf::from("./src/main.rs");

    let hash = FileHasher::hash_bytes(b"content");

    cache.insert(path1.clone(), hash);

    // Different path representations are different keys
    assert_eq!(cache.get(&path1), Some(&hash));
    assert_eq!(cache.get(&path2), None); // Different path string
}

#[test]
fn test_large_cache() {
    let mut cache = HashCache::with_capacity(1000);

    // Add many entries
    for i in 0..1000 {
        let path = PathBuf::from(format!("src/file{}.rs", i));
        let hash = FileHasher::hash_bytes(format!("content {}", i).as_bytes());
        cache.insert(path, hash);
    }

    assert_eq!(cache.len(), 1000);

    // Verify random access works
    for i in (0..1000).step_by(100) {
        let path = PathBuf::from(format!("src/file{}.rs", i));
        assert!(cache.contains(&path));
    }
}

#[test]
fn test_cache_after_remove() {
    let mut cache = HashCache::new();

    let path1 = PathBuf::from("src/main.rs");
    let path2 = PathBuf::from("src/lib.rs");
    let hash1 = FileHasher::hash_bytes(b"main");
    let hash2 = FileHasher::hash_bytes(b"lib");

    cache.insert(path1.clone(), hash1);
    cache.insert(path2.clone(), hash2);

    assert_eq!(cache.len(), 2);

    cache.remove(&path1);

    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get(&path1), None);
    assert_eq!(cache.get(&path2), Some(&hash2));
}

#[test]
fn test_cache_reinsert_after_remove() {
    let mut cache = HashCache::new();
    let path = PathBuf::from("src/main.rs");
    let hash1 = FileHasher::hash_bytes(b"version 1");
    let hash2 = FileHasher::hash_bytes(b"version 2");

    cache.insert(path.clone(), hash1);
    cache.remove(&path);
    cache.insert(path.clone(), hash2);

    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get(&path), Some(&hash2));
}

#[test]
fn test_absolute_vs_relative_paths() {
    let mut cache = HashCache::new();

    let relative = PathBuf::from("src/main.rs");
    let absolute = PathBuf::from("/workspace/src/main.rs");

    let hash = FileHasher::hash_bytes(b"content");

    cache.insert(relative.clone(), hash);

    // Absolute and relative paths are different keys
    assert_eq!(cache.get(&relative), Some(&hash));
    assert_eq!(cache.get(&absolute), None);
}

#[test]
fn test_nested_paths() {
    let mut cache = HashCache::new();

    let paths = vec![
        PathBuf::from("src/main.rs"),
        PathBuf::from("src/lib/mod.rs"),
        PathBuf::from("src/lib/utils/helpers.rs"),
        PathBuf::from("tests/integration/test.rs"),
    ];

    for (i, path) in paths.iter().enumerate() {
        let hash = FileHasher::hash_bytes(format!("content {}", i).as_bytes());
        cache.insert(path.clone(), hash);
    }

    assert_eq!(cache.len(), paths.len());

    for path in &paths {
        assert!(cache.contains(path));
    }
}
