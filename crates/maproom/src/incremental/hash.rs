//! File content hashing using blake3.
//!
//! This module provides fast, cryptographically secure hashing for change detection
//! in the incremental indexing pipeline.

use anyhow::{Context, Result};
use blake3;
use std::path::Path;

/// Content hash type based on blake3.
///
/// Blake3 provides fast hashing (multiple GB/s throughput) with cryptographic security.
/// Target performance: <10ms per file for typical source code files.
pub type ContentHash = blake3::Hash;

/// File hasher for computing content hashes.
pub struct FileHasher;

impl FileHasher {
    /// Hash the contents of a file.
    ///
    /// # Arguments
    /// * `path` - Path to the file to hash
    ///
    /// # Returns
    /// * `Ok(ContentHash)` - The blake3 hash of the file contents
    /// * `Err(_)` - If the file cannot be read
    ///
    /// # Performance
    /// Target: <10ms for typical source code files (< 100KB)
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use maproom::incremental::FileHasher;
    ///
    /// let hash = FileHasher::hash_file(Path::new("src/main.rs")).unwrap();
    /// println!("Hash: {}", hash);
    /// ```
    pub fn hash_file(path: &Path) -> Result<ContentHash> {
        let content = std::fs::read(path)
            .with_context(|| format!("Failed to read file for hashing: {}", path.display()))?;

        Ok(blake3::hash(&content))
    }

    /// Hash raw bytes.
    ///
    /// Useful for testing or when file content is already in memory.
    ///
    /// # Arguments
    /// * `content` - The bytes to hash
    ///
    /// # Returns
    /// The blake3 hash of the content
    pub fn hash_bytes(content: &[u8]) -> ContentHash {
        blake3::hash(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_file_basic() {
        // Create a temporary file with known content
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"Hello, World!";
        temp_file.write_all(content).unwrap();
        temp_file.flush().unwrap();

        // Hash the file
        let hash = FileHasher::hash_file(temp_file.path()).unwrap();

        // Hash should be deterministic
        let hash2 = FileHasher::hash_file(temp_file.path()).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_bytes() {
        let content = b"Hello, World!";
        let hash1 = FileHasher::hash_bytes(content);
        let hash2 = FileHasher::hash_bytes(content);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_content() {
        let content1 = b"Hello, World!";
        let content2 = b"Goodbye, World!";

        let hash1 = FileHasher::hash_bytes(content1);
        let hash2 = FileHasher::hash_bytes(content2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_empty_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.flush().unwrap();

        let hash = FileHasher::hash_file(temp_file.path()).unwrap();

        // Should successfully hash empty file
        let empty_hash = FileHasher::hash_bytes(b"");
        assert_eq!(hash, empty_hash);
    }

    #[test]
    fn test_hash_nonexistent_file() {
        let result = FileHasher::hash_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_consistency_with_file_and_bytes() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = b"Test content for consistency check";
        temp_file.write_all(content).unwrap();
        temp_file.flush().unwrap();

        let file_hash = FileHasher::hash_file(temp_file.path()).unwrap();
        let bytes_hash = FileHasher::hash_bytes(content);

        assert_eq!(file_hash, bytes_hash);
    }
}
