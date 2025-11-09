//! Unit tests for file hashing functionality.
//!
//! Tests cover:
//! - Hash generation correctness
//! - Performance characteristics
//! - Error handling
//! - Consistency between file and byte hashing

use crewchief_maproom::incremental::{ContentHash, FileHasher};
use std::io::Write;
use std::time::Instant;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_hash_deterministic() {
    // Same content should always produce the same hash
    let content = b"fn main() { println!(\"Hello, World!\"); }";

    let hash1 = FileHasher::hash_bytes(content);
    let hash2 = FileHasher::hash_bytes(content);

    assert_eq!(
        hash1, hash2,
        "Hashing the same content should produce identical hashes"
    );
}

#[test]
fn test_hash_different_content() {
    let content1 = b"fn main() { println!(\"Hello\"); }";
    let content2 = b"fn main() { println!(\"World\"); }";

    let hash1 = FileHasher::hash_bytes(content1);
    let hash2 = FileHasher::hash_bytes(content2);

    assert_ne!(
        hash1, hash2,
        "Different content should produce different hashes"
    );
}

#[test]
fn test_hash_file_vs_bytes_consistency() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let content = b"This is a test file for hash consistency.";

    temp_file
        .write_all(content)
        .expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");

    let file_hash = FileHasher::hash_file(temp_file.path()).expect("Failed to hash file");
    let bytes_hash = FileHasher::hash_bytes(content);

    assert_eq!(
        file_hash, bytes_hash,
        "Hashing a file should produce the same result as hashing its bytes"
    );
}

#[test]
fn test_hash_empty_file() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.flush().expect("Failed to flush temp file");

    let file_hash = FileHasher::hash_file(temp_file.path()).expect("Failed to hash empty file");
    let empty_hash = FileHasher::hash_bytes(b"");

    assert_eq!(
        file_hash, empty_hash,
        "Empty file should produce same hash as empty bytes"
    );
}

#[test]
fn test_hash_large_file() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");

    // Create a 1MB file
    let content = vec![0u8; 1024 * 1024];
    temp_file
        .write_all(&content)
        .expect("Failed to write large file");
    temp_file.flush().expect("Failed to flush temp file");

    let hash = FileHasher::hash_file(temp_file.path()).expect("Failed to hash large file");

    // Verify consistency
    let bytes_hash = FileHasher::hash_bytes(&content);
    assert_eq!(hash, bytes_hash);
}

#[test]
fn test_hash_nonexistent_file() {
    let result = FileHasher::hash_file(std::path::Path::new("/nonexistent/file.rs"));

    assert!(
        result.is_err(),
        "Hashing a nonexistent file should return an error"
    );
}

#[test]
fn test_hash_performance_small_file() {
    // Test that hashing a typical source file is fast (<10ms)
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");

    // Typical small source file (~5KB)
    let content = vec![b'x'; 5 * 1024];
    temp_file
        .write_all(&content)
        .expect("Failed to write test file");
    temp_file.flush().expect("Failed to flush test file");

    let start = Instant::now();
    let _hash = FileHasher::hash_file(temp_file.path()).expect("Failed to hash file");
    let duration = start.elapsed();

    // Blake3 should be very fast - well under 10ms for small files
    assert!(
        duration.as_millis() < 10,
        "Hashing a 5KB file took {:?}, expected <10ms",
        duration
    );
}

#[test]
fn test_hash_performance_medium_file() {
    // Test performance on a medium-sized file (~100KB)
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");

    let content = vec![b'x'; 100 * 1024];
    temp_file
        .write_all(&content)
        .expect("Failed to write test file");
    temp_file.flush().expect("Failed to flush test file");

    let start = Instant::now();
    let _hash = FileHasher::hash_file(temp_file.path()).expect("Failed to hash file");
    let duration = start.elapsed();

    // Should still be very fast even for 100KB
    assert!(
        duration.as_millis() < 10,
        "Hashing a 100KB file took {:?}, expected <10ms",
        duration
    );
}

#[test]
fn test_hash_whitespace_sensitive() {
    // Hashes should be sensitive to whitespace changes
    let content1 = b"fn main() { }";
    let content2 = b"fn main() {  }"; // Extra space

    let hash1 = FileHasher::hash_bytes(content1);
    let hash2 = FileHasher::hash_bytes(content2);

    assert_ne!(hash1, hash2, "Hashes should differ when whitespace changes");
}

#[test]
fn test_hash_newline_sensitive() {
    // Hashes should be sensitive to newline changes
    let content1 = b"line1\nline2";
    let content2 = b"line1\r\nline2"; // Different line ending

    let hash1 = FileHasher::hash_bytes(content1);
    let hash2 = FileHasher::hash_bytes(content2);

    assert_ne!(
        hash1, hash2,
        "Hashes should differ when line endings change"
    );
}

#[test]
fn test_hash_binary_file() {
    // Test hashing binary content (not just text)
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");

    let binary_content: Vec<u8> = (0..=255).collect();
    temp_file
        .write_all(&binary_content)
        .expect("Failed to write binary file");
    temp_file.flush().expect("Failed to flush temp file");

    let file_hash = FileHasher::hash_file(temp_file.path()).expect("Failed to hash binary file");
    let bytes_hash = FileHasher::hash_bytes(&binary_content);

    assert_eq!(file_hash, bytes_hash);
}

#[test]
fn test_hash_multiple_files_different() {
    // Create multiple files with different content
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let files = vec![
        ("file1.rs", b"fn foo() {}"),
        ("file2.rs", b"fn bar() {}"),
        ("file3.rs", b"fn baz() {}"),
    ];

    let mut hashes = Vec::new();
    for (name, content) in &files {
        let path = temp_dir.path().join(name);
        std::fs::write(&path, content).expect("Failed to write file");

        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        hashes.push(hash);
    }

    // All hashes should be unique
    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            assert_ne!(
                hashes[i], hashes[j],
                "Files {} and {} should have different hashes",
                files[i].0, files[j].0
            );
        }
    }
}

#[test]
fn test_hash_type_is_blake3() {
    // Verify that ContentHash is actually blake3::Hash
    let content = b"test";
    let hash = FileHasher::hash_bytes(content);

    // Should be able to convert to string representation
    let hash_str = hash.to_string();
    assert!(!hash_str.is_empty());

    // Blake3 hashes are 32 bytes
    let hash_bytes = hash.as_bytes();
    assert_eq!(hash_bytes.len(), 32);
}

#[test]
fn test_hash_can_be_compared() {
    // Test that hashes can be compared for equality
    let hash1 = FileHasher::hash_bytes(b"content1");
    let hash2 = FileHasher::hash_bytes(b"content1");
    let hash3 = FileHasher::hash_bytes(b"content2");

    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_hash_serialization() {
    // Test that hashes can be converted to/from bytes
    let original_content = b"test content for serialization";
    let hash = FileHasher::hash_bytes(original_content);

    // Convert to bytes
    let hash_bytes = hash.as_bytes();
    assert_eq!(hash_bytes.len(), 32);

    // Convert back from bytes
    let reconstructed = ContentHash::from(*hash_bytes);
    assert_eq!(hash, reconstructed);
}
