//! Change detection for incremental indexing.
//!
//! This module implements three-tier change detection:
//! 1. In-memory cache (fastest)
//! 2. Database lookup (fast)
//! 3. Filesystem hash (accurate)

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use std::path::Path;

use super::cache::HashCache;
use super::hash::{ContentHash, FileHasher};
use crate::db::SqliteStore;
use std::sync::Arc;

/// The type of change detected for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    /// No change detected - file content is the same
    None,
    /// New file - no previous hash exists
    New(ContentHash),
    /// Modified file - content hash changed
    Modified { old: ContentHash, new: ContentHash },
    /// Deleted file - file was removed from filesystem
    Deleted(ContentHash),
}

/// Change detector using three-tier comparison.
///
/// Implements efficient change detection by checking:
/// 1. In-memory cache first (no I/O)
/// 2. Database on cache miss (single query)
/// 3. Filesystem to get current hash (I/O + hashing)
///
/// # Architecture
///
/// The three-tier comparison strategy optimizes for speed:
/// - Cache hit: ~0.1μs (HashMap lookup)
/// - DB hit: ~1-5ms (single query)
/// - Filesystem: ~5-10ms (read + hash)
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use maproom::db::create_pool;
/// use maproom::incremental::ChangeDetector;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let pool = create_pool().await?;
///     let mut detector = ChangeDetector::new(pool);
///
///     // Detect changes for a file
///     let change = detector.detect_change(123, Path::new("src/main.rs")).await?;
///
///     println!("Change detected: {:?}", change);
///     Ok(())
/// }
/// ```
pub struct ChangeDetector {
    cache: HashCache,
    store: Arc<SqliteStore>,
}

impl ChangeDetector {
    /// Create a new change detector.
    ///
    /// # Arguments
    /// * `store` - SqliteStore instance
    ///
    /// # Returns
    /// A new change detector with an empty cache
    pub fn new(store: Arc<SqliteStore>) -> Self {
        Self {
            cache: HashCache::new(),
            store,
        }
    }

    /// Create a change detector with pre-allocated cache capacity.
    ///
    /// Use this when you know approximately how many files will be processed.
    ///
    /// # Arguments
    /// * `store` - SqliteStore instance
    /// * `capacity` - Initial cache capacity
    pub fn with_capacity(store: Arc<SqliteStore>, capacity: usize) -> Self {
        Self {
            cache: HashCache::with_capacity(capacity),
            store,
        }
    }

    /// Detect changes for a file using three-tier comparison.
    ///
    /// # Three-Tier Comparison Strategy
    ///
    /// 1. **Cache check**: Look up hash in in-memory cache
    ///    - Hit: Compare cached hash with filesystem hash
    ///    - Miss: Proceed to step 2
    ///
    /// 2. **Database check**: Query database for stored hash
    ///    - Hit: Compare DB hash with filesystem hash
    ///    - Miss: File is new, proceed to step 3
    ///
    /// 3. **Filesystem hash**: Compute current hash from file
    ///    - Compare with cached/DB hash if available
    ///    - Store new hash in cache
    ///
    /// # Arguments
    /// * `file_id` - Database ID of the file
    /// * `path` - Filesystem path to the file
    ///
    /// # Returns
    /// * `Ok(ChangeType::None)` - File unchanged
    /// * `Ok(ChangeType::New(hash))` - New file
    /// * `Ok(ChangeType::Modified{old, new})` - File modified
    /// * `Err(_)` - I/O error or database error
    ///
    /// # Performance
    ///
    /// - Cache hit: <1ms (no DB/filesystem access)
    /// - Cache miss + DB hit: ~1-5ms (single query + hash)
    /// - Cache miss + DB miss: ~5-10ms (hash only, new file)
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use std::path::Path;
    /// # use maproom::db::create_pool;
    /// # use maproom::incremental::{ChangeDetector, ChangeType};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let pool = create_pool().await?;
    /// let mut detector = ChangeDetector::new(pool);
    /// let path = Path::new("src/main.rs");
    ///
    /// match detector.detect_change(123, path).await? {
    ///     ChangeType::None => println!("No change"),
    ///     ChangeType::New(hash) => println!("New file: {}", hash),
    ///     ChangeType::Modified { old, new } => {
    ///         println!("Modified: {} -> {}", old, new);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn detect_change(&mut self, file_id: i64, path: &Path) -> Result<ChangeType> {
        // Step 3: Always compute current filesystem hash
        let current_hash = FileHasher::hash_file(path)
            .with_context(|| format!("Failed to hash file: {}", path.display()))?;

        // Step 1: Check in-memory cache first
        if let Some(cached_hash) = self.cache.get(path) {
            if *cached_hash == current_hash {
                return Ok(ChangeType::None);
            } else {
                // Cache hit but hash changed - file was modified
                let old_hash = *cached_hash;
                // Update cache with new hash
                self.cache.insert(path.to_path_buf(), current_hash);
                return Ok(ChangeType::Modified {
                    old: old_hash,
                    new: current_hash,
                });
            }
        }

        // Step 2: Check database if cache miss
        let db_hash = get_hash_from_db(&self.store, file_id).await?;

        let change_type = match db_hash {
            Some(old_hash) => {
                if old_hash == current_hash {
                    ChangeType::None
                } else {
                    ChangeType::Modified {
                        old: old_hash,
                        new: current_hash,
                    }
                }
            }
            None => {
                // No hash in database - this is a new file
                ChangeType::New(current_hash)
            }
        };

        // Update cache with current hash
        self.cache.insert(path.to_path_buf(), current_hash);

        Ok(change_type)
    }

    /// Get the hash cache for inspection or manual management.
    ///
    /// # Returns
    /// Reference to the internal hash cache
    pub fn cache(&self) -> &HashCache {
        &self.cache
    }

    /// Get mutable access to the hash cache.
    ///
    /// Useful for clearing the cache or manually inserting hashes.
    ///
    /// # Returns
    /// Mutable reference to the internal hash cache
    pub fn cache_mut(&mut self) -> &mut HashCache {
        &mut self.cache
    }

    /// Clear the in-memory hash cache.
    ///
    /// Useful when:
    /// - Starting a fresh scan
    /// - Freeing memory in long-running processes
    /// - Forcing database lookups for all files
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Detect if a file has been deleted.
    ///
    /// Checks if a file exists on the filesystem. If not, queries the database
    /// to see if it was previously tracked (had a hash).
    ///
    /// # Arguments
    /// * `file_id` - Database ID of the file
    /// * `path` - Filesystem path where the file should exist
    ///
    /// # Returns
    /// * `Ok(Some(ChangeType::Deleted(hash)))` - File was deleted, returns its last hash
    /// * `Ok(None)` - File still exists or was never tracked
    /// * `Err(_)` - Database query error
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use std::path::Path;
    /// # use maproom::db::create_pool;
    /// # use maproom::incremental::{ChangeDetector, ChangeType};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let pool = create_pool().await?;
    /// let detector = ChangeDetector::new(pool);
    /// let path = Path::new("deleted_file.rs");
    ///
    /// if let Some(ChangeType::Deleted(hash)) = detector.detect_deletion(123, path).await? {
    ///     println!("File was deleted, last hash: {}", hash);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn detect_deletion(&self, file_id: i64, path: &Path) -> Result<Option<ChangeType>> {
        // Check if file exists on filesystem
        if path.exists() {
            return Ok(None); // Not deleted
        }

        // File doesn't exist - check if it was in database
        if let Some(old_hash) = get_hash_from_db(&self.store, file_id).await? {
            return Ok(Some(ChangeType::Deleted(old_hash)));
        }

        Ok(None) // Was never tracked
    }

    /// Detect if a file has been moved or renamed.
    ///
    /// Searches the database for files with the same content hash but at a different path.
    /// This indicates the file was moved/renamed rather than being a true "new" file.
    ///
    /// # Arguments
    /// * `new_path` - The new path where the file now exists
    /// * `hash` - The content hash of the file at the new path
    ///
    /// # Returns
    /// * `Ok(Some(old_path))` - File was moved from `old_path` to `new_path`
    /// * `Ok(None)` - No previous file with this hash found (truly new file)
    /// * `Err(_)` - Database query error
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use std::path::Path;
    /// # use maproom::db::create_pool;
    /// # use maproom::incremental::{ChangeDetector, FileHasher};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let pool = create_pool().await?;
    /// let detector = ChangeDetector::new(pool);
    /// let new_path = Path::new("src/new_location.rs");
    /// let hash = FileHasher::hash_file(new_path)?;
    ///
    /// if let Some(old_path) = detector.detect_move(new_path, &hash).await? {
    ///     println!("File moved: {} -> {}", old_path.display(), new_path.display());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn detect_move(
        &self,
        new_path: &Path,
        hash: &ContentHash,
    ) -> Result<Option<std::path::PathBuf>> {
        let hex_str = hash.to_hex().to_string();
        let new_path_str = new_path.to_string_lossy().to_string();

        self.store
            .run(move |conn| {
                // Find a file with the same content_hash but at a different path
                let result: Option<String> = conn
                    .query_row(
                        "SELECT relpath FROM files WHERE content_hash = ?1 AND relpath != ?2 LIMIT 1",
                        rusqlite::params![hex_str, new_path_str],
                        |row| row.get(0),
                    )
                    .optional()?;

                Ok(result.map(std::path::PathBuf::from))
            })
            .await
    }

    /// Detect changes for multiple files in a batch.
    ///
    /// This method efficiently processes multiple files by:
    /// 1. Computing all filesystem hashes
    /// 2. Checking the in-memory cache for all files
    /// 3. Batch querying the database for cache misses
    /// 4. Comparing hashes to determine change types
    ///
    /// # Arguments
    /// * `files` - Slice of (file_id, path) tuples to process
    ///
    /// # Returns
    /// Vector of (file_id, ChangeType) pairs in the same order as input
    ///
    /// # Performance
    ///
    /// Batch processing is more efficient than individual calls because:
    /// - Single database query for all cache misses (vs N queries)
    /// - Better CPU cache utilization for hash computation
    /// - Reduced connection pool contention
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use std::path::PathBuf;
    /// # use maproom::db::create_pool;
    /// # use maproom::incremental::ChangeDetector;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let pool = create_pool().await?;
    /// let mut detector = ChangeDetector::new(pool);
    ///
    /// let files = vec![
    ///     (1, PathBuf::from("src/main.rs")),
    ///     (2, PathBuf::from("src/lib.rs")),
    ///     (3, PathBuf::from("tests/test.rs")),
    /// ];
    ///
    /// let changes = detector.detect_changes_batch(&files).await?;
    /// for (file_id, change) in changes {
    ///     println!("File {}: {:?}", file_id, change);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn detect_changes_batch(
        &mut self,
        files: &[(i64, std::path::PathBuf)],
    ) -> Result<Vec<(i64, ChangeType)>> {
        use std::collections::HashMap;

        if files.is_empty() {
            return Ok(Vec::new());
        }

        // Step 1: Compute all filesystem hashes
        let mut file_hashes: HashMap<i64, ContentHash> = HashMap::with_capacity(files.len());
        for (file_id, path) in files {
            let hash = FileHasher::hash_file(path)
                .with_context(|| format!("Failed to hash file: {}", path.display()))?;
            file_hashes.insert(*file_id, hash);
        }

        // Step 2: Check cache and collect cache misses
        let mut cache_misses = Vec::new();
        let mut results: HashMap<i64, ChangeType> = HashMap::with_capacity(files.len());

        for (file_id, path) in files {
            let current_hash = file_hashes[file_id];

            if let Some(cached_hash) = self.cache.get(path) {
                // Cache hit
                if *cached_hash == current_hash {
                    results.insert(*file_id, ChangeType::None);
                } else {
                    results.insert(
                        *file_id,
                        ChangeType::Modified {
                            old: *cached_hash,
                            new: current_hash,
                        },
                    );
                    // Update cache with new hash
                    self.cache.insert(path.to_path_buf(), current_hash);
                }
            } else {
                // Cache miss - need to query database
                cache_misses.push(*file_id);
            }
        }

        // Step 3: Batch query database for all cache misses
        if !cache_misses.is_empty() {
            let db_hashes = get_hashes_batch_from_db(&self.store, &cache_misses).await?;

            for file_id in cache_misses {
                let current_hash = file_hashes[&file_id];
                let path = &files.iter().find(|(id, _)| *id == file_id).unwrap().1;

                let change_type = match db_hashes.get(&file_id) {
                    Some(old_hash) => {
                        if *old_hash == current_hash {
                            ChangeType::None
                        } else {
                            ChangeType::Modified {
                                old: *old_hash,
                                new: current_hash,
                            }
                        }
                    }
                    None => ChangeType::New(current_hash),
                };

                results.insert(file_id, change_type);
                self.cache.insert(path.to_path_buf(), current_hash);
            }
        }

        // Return results in the same order as input
        Ok(files
            .iter()
            .map(|(file_id, _)| (*file_id, results[file_id].clone()))
            .collect())
    }
}

/// Retrieve a file's blake3 hash from the database.
///
/// # Arguments
/// * `store` - SqliteStore instance
/// * `file_id` - Database ID of the file
///
/// # Returns
/// * `Ok(Some(hash))` - Hash found in database
/// * `Ok(None)` - File exists but has no hash (NULL in database)
/// * `Err(_)` - Database query error or file not found
pub async fn get_hash_from_db(store: &SqliteStore, file_id: i64) -> Result<Option<ContentHash>> {
    store
        .run(move |conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT content_hash FROM files WHERE id = ?1",
                    rusqlite::params![file_id],
                    |row| row.get(0),
                )
                .optional()?;

            match result {
                Some(hex_str) => {
                    // Parse the hex string back to a blake3 hash
                    let hash = blake3::Hash::from_hex(&hex_str)
                        .map_err(|e| anyhow::anyhow!("Invalid hash in database: {}", e))?;
                    Ok(Some(hash))
                }
                None => Ok(None),
            }
        })
        .await
}

/// Store a file's blake3 hash in the database.
///
/// # Arguments
/// * `store` - SqliteStore instance
/// * `file_id` - Database ID of the file
/// * `hash` - Blake3 content hash to store
///
/// # Returns
/// * `Ok(())` - Hash stored successfully
/// * `Err(_)` - Database update error
pub async fn store_hash_in_db(store: &SqliteStore, file_id: i64, hash: ContentHash) -> Result<()> {
    let hex_str = hash.to_hex().to_string();
    store
        .run(move |conn| {
            conn.execute(
                "UPDATE files SET content_hash = ?1 WHERE id = ?2",
                rusqlite::params![hex_str, file_id],
            )?;
            Ok(())
        })
        .await
}

/// Retrieve blake3 hashes for multiple files from the database in a single query.
///
/// # Arguments
/// * `store` - SqliteStore instance
/// * `file_ids` - Slice of file IDs to query
///
/// # Returns
/// * `Ok(HashMap)` - Map of file_id to hash for files that have hashes
/// * `Err(_)` - Database query error
pub async fn get_hashes_batch_from_db(
    store: &SqliteStore,
    file_ids: &[i64],
) -> Result<std::collections::HashMap<i64, ContentHash>> {
    use std::collections::HashMap;

    if file_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let file_ids = file_ids.to_vec();
    store
        .run(move |conn| {
            let mut hashes = HashMap::new();

            // Build placeholder string for IN clause
            let placeholders: String = file_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "SELECT id, content_hash FROM files WHERE id IN ({})",
                placeholders
            );

            let mut stmt = conn.prepare(&sql)?;

            // Convert file_ids to rusqlite params
            let params: Vec<&dyn rusqlite::ToSql> = file_ids
                .iter()
                .map(|id| id as &dyn rusqlite::ToSql)
                .collect();

            let rows = stmt.query_map(params.as_slice(), |row| {
                let id: i64 = row.get(0)?;
                let hex_str: Option<String> = row.get(1)?;
                Ok((id, hex_str))
            })?;

            for row_result in rows {
                let (id, hex_str_opt) = row_result?;
                if let Some(hex_str) = hex_str_opt {
                    if let Ok(hash) = blake3::Hash::from_hex(&hex_str) {
                        hashes.insert(id, hash);
                    }
                }
            }

            Ok(hashes)
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_change_type_equality() {
        let hash1 = FileHasher::hash_bytes(b"content1");
        let hash2 = FileHasher::hash_bytes(b"content2");

        // Test None equality
        assert_eq!(ChangeType::None, ChangeType::None);

        // Test New equality
        assert_eq!(ChangeType::New(hash1), ChangeType::New(hash1));
        assert_ne!(ChangeType::New(hash1), ChangeType::New(hash2));

        // Test Modified equality
        assert_eq!(
            ChangeType::Modified {
                old: hash1,
                new: hash2
            },
            ChangeType::Modified {
                old: hash1,
                new: hash2
            }
        );
        assert_ne!(
            ChangeType::Modified {
                old: hash1,
                new: hash2
            },
            ChangeType::Modified {
                old: hash2,
                new: hash1
            }
        );

        // Test Deleted equality
        assert_eq!(ChangeType::Deleted(hash1), ChangeType::Deleted(hash1));
        assert_ne!(ChangeType::Deleted(hash1), ChangeType::Deleted(hash2));

        // Test cross-variant inequality
        assert_ne!(ChangeType::None, ChangeType::New(hash1));
        assert_ne!(
            ChangeType::New(hash1),
            ChangeType::Modified {
                old: hash1,
                new: hash2
            }
        );
        assert_ne!(ChangeType::New(hash1), ChangeType::Deleted(hash1));
        assert_ne!(
            ChangeType::Modified {
                old: hash1,
                new: hash2
            },
            ChangeType::Deleted(hash1)
        );
    }

    #[test]
    fn test_change_type_clone() {
        let hash1 = FileHasher::hash_bytes(b"content1");
        let hash2 = FileHasher::hash_bytes(b"content2");

        let change = ChangeType::Modified {
            old: hash1,
            new: hash2,
        };
        let cloned = change.clone();

        assert_eq!(change, cloned);
    }

    #[test]
    fn test_detector_new() {
        // Note: This test doesn't actually use the pool, so we can't easily test it
        // without a real database connection. Integration tests will cover this.

        // Just verify the struct can be constructed (requires pool from integration test)
    }

    #[test]
    fn test_hash_file_for_change_detection() {
        // Create temporary files with different content
        let mut file1 = NamedTempFile::new().unwrap();
        let mut file2 = NamedTempFile::new().unwrap();

        file1.write_all(b"content 1").unwrap();
        file1.flush().unwrap();

        file2.write_all(b"content 2").unwrap();
        file2.flush().unwrap();

        // Hash both files
        let hash1 = FileHasher::hash_file(file1.path()).unwrap();
        let hash2 = FileHasher::hash_file(file2.path()).unwrap();

        // Hashes should be different
        assert_ne!(hash1, hash2);

        // Re-hash first file should give same result
        let hash1_again = FileHasher::hash_file(file1.path()).unwrap();
        assert_eq!(hash1, hash1_again);
    }
}
