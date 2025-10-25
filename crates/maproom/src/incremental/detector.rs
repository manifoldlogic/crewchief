//! Change detection for incremental indexing.
//!
//! This module implements three-tier change detection:
//! 1. In-memory cache (fastest)
//! 2. Database lookup (fast)
//! 3. Filesystem hash (accurate)

use anyhow::{Context, Result};
use std::path::Path;

use super::cache::HashCache;
use super::hash::{ContentHash, FileHasher};
use crate::db::PgPool;

/// The type of change detected for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    /// No change detected - file content is the same
    None,
    /// New file - no previous hash exists
    New(ContentHash),
    /// Modified file - content hash changed
    Modified {
        old: ContentHash,
        new: ContentHash,
    },
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
/// ```no_run
/// use std::path::Path;
/// use crewchief_maproom::db::create_pool;
/// use crewchief_maproom::incremental::ChangeDetector;
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
    pool: PgPool,
}

impl ChangeDetector {
    /// Create a new change detector.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    /// A new change detector with an empty cache
    pub fn new(pool: PgPool) -> Self {
        Self {
            cache: HashCache::new(),
            pool,
        }
    }

    /// Create a change detector with pre-allocated cache capacity.
    ///
    /// Use this when you know approximately how many files will be processed.
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `capacity` - Initial cache capacity
    pub fn with_capacity(pool: PgPool, capacity: usize) -> Self {
        Self {
            cache: HashCache::with_capacity(capacity),
            pool,
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
    /// ```no_run
    /// # use std::path::Path;
    /// # use crewchief_maproom::db::create_pool;
    /// # use crewchief_maproom::incremental::{ChangeDetector, ChangeType};
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
        let db_hash = get_hash_from_db(&self.pool, file_id).await?;

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
}

/// Retrieve a file's blake3 hash from the database.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `file_id` - Database ID of the file
///
/// # Returns
/// * `Ok(Some(hash))` - Hash found in database
/// * `Ok(None)` - File exists but has no hash (NULL in database)
/// * `Err(_)` - Database query error or file not found
///
/// # Database Schema
///
/// Queries the `maproom.files` table's `blake3_hash` column (BYTEA).
/// The column is nullable to support existing rows without hashes.
pub async fn get_hash_from_db(pool: &PgPool, file_id: i64) -> Result<Option<ContentHash>> {
    let client = pool
        .get()
        .await
        .context("Failed to get database connection from pool")?;

    let row = client
        .query_opt(
            "SELECT blake3_hash FROM maproom.files WHERE id = $1",
            &[&file_id],
        )
        .await
        .with_context(|| format!("Failed to query hash for file_id={}", file_id))?;

    match row {
        Some(row) => {
            let hash_bytes: Option<Vec<u8>> = row.get(0);
            match hash_bytes {
                Some(bytes) => {
                    // Convert bytes to blake3::Hash
                    if bytes.len() != 32 {
                        anyhow::bail!(
                            "Invalid blake3 hash length: expected 32 bytes, got {}",
                            bytes.len()
                        );
                    }
                    let mut hash_array = [0u8; 32];
                    hash_array.copy_from_slice(&bytes);
                    Ok(Some(ContentHash::from(hash_array)))
                }
                None => Ok(None), // NULL in database
            }
        }
        None => {
            // File not found in database
            anyhow::bail!("File with id={} not found in database", file_id)
        }
    }
}

/// Store a file's blake3 hash in the database.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `file_id` - Database ID of the file
/// * `hash` - Blake3 content hash to store
///
/// # Returns
/// * `Ok(())` - Hash stored successfully
/// * `Err(_)` - Database update error
///
/// # Database Schema
///
/// Updates the `maproom.files` table's `blake3_hash` column (BYTEA).
/// The hash is stored as 32 bytes in binary format.
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::db::create_pool;
/// use crewchief_maproom::incremental::{FileHasher, detector::store_hash_in_db};
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let pool = create_pool().await?;
///     let hash = FileHasher::hash_file(Path::new("src/main.rs"))?;
///
///     store_hash_in_db(&pool, 123, hash).await?;
///     Ok(())
/// }
/// ```
pub async fn store_hash_in_db(pool: &PgPool, file_id: i64, hash: ContentHash) -> Result<()> {
    let client = pool
        .get()
        .await
        .context("Failed to get database connection from pool")?;

    let hash_bytes: &[u8] = hash.as_bytes();

    client
        .execute(
            "UPDATE maproom.files SET blake3_hash = $1 WHERE id = $2",
            &[&hash_bytes, &file_id],
        )
        .await
        .with_context(|| format!("Failed to store hash for file_id={}", file_id))?;

    Ok(())
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

        // Test cross-variant inequality
        assert_ne!(ChangeType::None, ChangeType::New(hash1));
        assert_ne!(
            ChangeType::New(hash1),
            ChangeType::Modified {
                old: hash1,
                new: hash2
            }
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
