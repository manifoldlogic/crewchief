//! Content-addressed storage using Git-compatible blob SHA-256 hashes.
//!
//! This module provides utilities for computing SHA-256 hashes of code chunk content
//! using Git's blob object format. This enables content-based deduplication and
//! compatibility with Git's object storage system.

use sha2::{Digest, Sha256};

/// Compute Git-compatible blob SHA-256 for content.
///
/// This function computes a SHA-256 hash using Git's blob object format:
/// `SHA256("blob <size>\0<content>")`
///
/// The format is compatible with `git hash-object --stdin` (when Git uses SHA-256).
///
/// # Arguments
///
/// * `content` - The string content to hash
///
/// # Returns
///
/// A lowercase hexadecimal string representation of the SHA-256 hash (64 characters).
///
/// # Examples
///
/// ```
/// use maproom::content_hash::compute_blob_sha;
///
/// let content = "function foo() { return 1; }";
/// let sha = compute_blob_sha(content);
/// assert_eq!(sha.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
/// ```
///
/// # Performance
///
/// This function is called for every chunk during indexing. SHA-256 computation
/// is approximately 1μs per chunk on modern hardware.
pub fn compute_blob_sha(content: &str) -> String {
    let mut hasher = Sha256::new();

    // Git blob header: "blob <size>\0"
    hasher.update(b"blob ");
    hasher.update(content.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(content.as_bytes());

    // Return lowercase hex string
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_sha_deterministic() {
        // Same content should always produce the same SHA
        let content = "function foo() { return 1; }";
        let sha1 = compute_blob_sha(content);
        let sha2 = compute_blob_sha(content);
        assert_eq!(sha1, sha2);
        assert_eq!(sha1.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_blob_sha_different_content() {
        // Different content should produce different SHAs
        let content1 = "function foo() { return 1; }";
        let content2 = "function bar() { return 2; }";
        let sha1 = compute_blob_sha(content1);
        let sha2 = compute_blob_sha(content2);
        assert_ne!(sha1, sha2);
    }

    #[test]
    fn test_blob_sha_whitespace_sensitive() {
        // Content addressing is bit-for-bit identical
        // Even a single extra space should change the SHA
        let content1 = "function foo() { return 1; }";
        let content2 = "function foo() { return 1;  }"; // Extra space
        let sha1 = compute_blob_sha(content1);
        let sha2 = compute_blob_sha(content2);
        assert_ne!(sha1, sha2);
    }

    #[test]
    fn test_blob_sha_empty_content() {
        // Empty content should produce a valid SHA
        let content = "";
        let sha = compute_blob_sha(content);
        assert_eq!(sha.len(), 64);

        // Should be deterministic even for empty content
        let sha2 = compute_blob_sha(content);
        assert_eq!(sha, sha2);
    }

    #[test]
    fn test_blob_sha_unicode() {
        // Unicode content should be handled correctly
        let content = "函数 foo() { return 'привет'; } // こんにちは";
        let sha1 = compute_blob_sha(content);
        let sha2 = compute_blob_sha(content);
        assert_eq!(sha1, sha2);
        assert_eq!(sha1.len(), 64);

        // Different unicode should produce different hashes
        let different = "函数 bar() { return 'привет'; } // こんにちは";
        assert_ne!(sha1, compute_blob_sha(different));
    }

    #[test]
    fn test_blob_sha_git_compatibility() {
        // Test against known Git SHA-256 hashes for specific content
        // These values were verified using: echo -n "blob <size>"$'\0'"<content>" | sha256sum
        // Note: Git's default is SHA-1, but the format is the same for SHA-256

        // For "test" content:
        // blob 4\0test
        let content = "test";
        let sha = compute_blob_sha(content);

        // Verify it's a valid 64-character hex string
        assert_eq!(sha.len(), 64);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));

        // Verify deterministic
        assert_eq!(sha, compute_blob_sha(content));

        // Manually computed expected value for "blob 4\0test"
        // Using: printf 'blob 4\0test' | sha256sum
        let expected = "aa19560d465e7d43915547490a1f6b73eb55702e3d12cb82fb577df60bad4928";
        assert_eq!(sha, expected);

        // Test empty content (blob 0\0)
        let empty = "";
        let empty_sha = compute_blob_sha(empty);
        // Using: printf 'blob 0\0' | sha256sum
        let expected_empty = "473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813";
        assert_eq!(empty_sha, expected_empty);
    }

    #[test]
    fn test_blob_sha_large_content() {
        // Test with larger content to ensure no issues with size
        let large_content = "x".repeat(100_000);
        let sha1 = compute_blob_sha(&large_content);
        let sha2 = compute_blob_sha(&large_content);
        assert_eq!(sha1, sha2);
        assert_eq!(sha1.len(), 64);
    }

    #[test]
    fn test_blob_sha_newlines() {
        // Newlines should be preserved in hashing
        let content1 = "line1\nline2\nline3";
        let content2 = "line1line2line3";
        assert_ne!(compute_blob_sha(content1), compute_blob_sha(content2));

        // Different newline types should produce different hashes
        let content_lf = "line1\nline2";
        let content_crlf = "line1\r\nline2";
        assert_ne!(compute_blob_sha(content_lf), compute_blob_sha(content_crlf));
    }
}
