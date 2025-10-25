//! Integration tests for incremental indexing change detection.
//!
//! These tests verify the complete change detection flow:
//! - Three-tier comparison (cache → database → filesystem)
//! - Hash storage and retrieval from database
//! - Cache hit/miss scenarios
//! - Detecting new, modified, and unchanged files

use anyhow::Result;
use crewchief_maproom::db::{create_pool, PgPool};
use crewchief_maproom::incremental::{
    detector::{get_hash_from_db, store_hash_in_db},
    ChangeDetector, ChangeType, FileHasher,
};
use std::io::{Seek, Write};
use tempfile::NamedTempFile;
use tokio;

/// Helper to create a test database pool.
async fn setup_pool() -> Result<PgPool> {
    // Load .env if present
    let _ = dotenvy::dotenv();

    create_pool().await
}

/// Helper to insert a test file into the database.
///
/// Returns the file_id for testing.
async fn insert_test_file(
    pool: &PgPool,
    repo_name: &str,
    worktree_name: &str,
    relpath: &str,
) -> Result<i64> {
    let client = pool.get().await?;

    // Create test repo
    let repo_row = client
        .query_one(
            "INSERT INTO maproom.repos(name, root_path)
             VALUES ($1, '/test/repo')
             ON CONFLICT(name) DO UPDATE SET root_path = EXCLUDED.root_path
             RETURNING id",
            &[&repo_name],
        )
        .await?;
    let repo_id: i64 = repo_row.get(0);

    // Create test worktree
    let worktree_row = client
        .query_one(
            "INSERT INTO maproom.worktrees(repo_id, name, abs_path)
             VALUES ($1, $2, '/test/worktree')
             ON CONFLICT(repo_id, name) DO UPDATE SET abs_path = EXCLUDED.abs_path
             RETURNING id",
            &[&repo_id, &worktree_name],
        )
        .await?;
    let worktree_id: i64 = worktree_row.get(0);

    // Create test commit
    let commit_row = client
        .query_one(
            "INSERT INTO maproom.commits(repo_id, sha, committed_at)
             VALUES ($1, 'test_sha_123', NOW())
             ON CONFLICT(repo_id, sha) DO UPDATE SET committed_at = EXCLUDED.committed_at
             RETURNING id",
            &[&repo_id],
        )
        .await?;
    let commit_id: i64 = commit_row.get(0);

    // Create test file
    let file_row = client
        .query_one(
            "INSERT INTO maproom.files(repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes, last_modified)
             VALUES ($1, $2, $3, $4, 'rust', 'test_hash', 100, NOW())
             ON CONFLICT(commit_id, relpath, content_hash) DO UPDATE SET size_bytes = EXCLUDED.size_bytes
             RETURNING id",
            &[&repo_id, &worktree_id, &commit_id, &relpath],
        )
        .await?;
    let file_id: i64 = file_row.get(0);

    Ok(file_id)
}

/// Helper to clean up test data.
async fn cleanup_test_file(pool: &PgPool, file_id: i64) -> Result<()> {
    let client = pool.get().await?;

    // Delete test file (cascades should handle related data)
    client
        .execute("DELETE FROM maproom.files WHERE id = $1", &[&file_id])
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_get_hash_from_db_null() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file without a hash
    let file_id = insert_test_file(&pool, "test_repo_null", "main", "test_null.rs").await?;

    // Should return None for NULL hash
    let hash = get_hash_from_db(&pool, file_id).await?;
    assert_eq!(hash, None);

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_store_and_retrieve_hash() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file
    let file_id = insert_test_file(&pool, "test_repo_store", "main", "test_store.rs").await?;

    // Hash some test content
    let test_content = b"fn main() { println!(\"Hello, World!\"); }";
    let hash = FileHasher::hash_bytes(test_content);

    // Store hash in database
    store_hash_in_db(&pool, file_id, hash).await?;

    // Retrieve hash from database
    let retrieved_hash = get_hash_from_db(&pool, file_id).await?;
    assert_eq!(retrieved_hash, Some(hash));

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_store_hash_overwrites() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file
    let file_id =
        insert_test_file(&pool, "test_repo_overwrite", "main", "test_overwrite.rs").await?;

    // Store first hash
    let hash1 = FileHasher::hash_bytes(b"content 1");
    store_hash_in_db(&pool, file_id, hash1).await?;

    // Store second hash (should overwrite)
    let hash2 = FileHasher::hash_bytes(b"content 2");
    store_hash_in_db(&pool, file_id, hash2).await?;

    // Should retrieve the second hash
    let retrieved_hash = get_hash_from_db(&pool, file_id).await?;
    assert_eq!(retrieved_hash, Some(hash2));
    assert_ne!(retrieved_hash, Some(hash1));

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_detect_new_file() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file in database (no hash)
    let file_id = insert_test_file(&pool, "test_repo_new", "main", "test_new.rs").await?;

    // Create temporary file on filesystem
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(b"new file content")?;
    temp_file.flush()?;

    // Detect changes
    let mut detector = ChangeDetector::new(pool.clone());
    let change = detector.detect_change(file_id, temp_file.path()).await?;

    // Should detect as new file
    match change {
        ChangeType::New(hash) => {
            let expected_hash = FileHasher::hash_bytes(b"new file content");
            assert_eq!(hash, expected_hash);
        }
        _ => panic!("Expected ChangeType::New, got {:?}", change),
    }

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_detect_modified_file() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file in database
    let file_id = insert_test_file(&pool, "test_repo_modified", "main", "test_modified.rs").await?;

    // Store initial hash
    let old_content = b"original content";
    let old_hash = FileHasher::hash_bytes(old_content);
    store_hash_in_db(&pool, file_id, old_hash).await?;

    // Create temporary file with different content
    let mut temp_file = NamedTempFile::new()?;
    let new_content = b"modified content";
    temp_file.write_all(new_content)?;
    temp_file.flush()?;

    // Detect changes
    let mut detector = ChangeDetector::new(pool.clone());
    let change = detector.detect_change(file_id, temp_file.path()).await?;

    // Should detect as modified
    match change {
        ChangeType::Modified { old, new } => {
            assert_eq!(old, old_hash);
            let expected_new_hash = FileHasher::hash_bytes(new_content);
            assert_eq!(new, expected_new_hash);
        }
        _ => panic!("Expected ChangeType::Modified, got {:?}", change),
    }

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_detect_unchanged_file() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file in database
    let file_id =
        insert_test_file(&pool, "test_repo_unchanged", "main", "test_unchanged.rs").await?;

    // Create temporary file
    let mut temp_file = NamedTempFile::new()?;
    let content = b"unchanged content";
    temp_file.write_all(content)?;
    temp_file.flush()?;

    // Store hash matching the file
    let hash = FileHasher::hash_file(temp_file.path())?;
    store_hash_in_db(&pool, file_id, hash).await?;

    // Detect changes
    let mut detector = ChangeDetector::new(pool.clone());
    let change = detector.detect_change(file_id, temp_file.path()).await?;

    // Should detect no change
    assert_eq!(change, ChangeType::None);

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_cache_hit_scenario() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file in database
    let file_id = insert_test_file(&pool, "test_repo_cache", "main", "test_cache.rs").await?;

    // Create temporary file
    let mut temp_file = NamedTempFile::new()?;
    let content = b"cached content";
    temp_file.write_all(content)?;
    temp_file.flush()?;

    let mut detector = ChangeDetector::new(pool.clone());

    // First detection: cache miss, database lookup
    let change1 = detector.detect_change(file_id, temp_file.path()).await?;
    assert!(matches!(change1, ChangeType::New(_)));

    // Verify hash is now in cache
    assert!(detector.cache().contains(temp_file.path()));

    // Second detection: cache hit, no database query
    let change2 = detector.detect_change(file_id, temp_file.path()).await?;
    assert_eq!(change2, ChangeType::None);

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_cache_detects_modification() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file in database
    let file_id =
        insert_test_file(&pool, "test_repo_cache_mod", "main", "test_cache_mod.rs").await?;

    // Create temporary file with initial content
    let mut temp_file = NamedTempFile::new()?;
    let content1 = b"initial content";
    temp_file.write_all(content1)?;
    temp_file.flush()?;

    let mut detector = ChangeDetector::new(pool.clone());

    // First detection: populate cache
    let hash1 = FileHasher::hash_bytes(content1);
    detector.cache_mut().insert(temp_file.path().to_path_buf(), hash1);

    // Modify file content
    temp_file.rewind()?;
    let content2 = b"modified content here";
    temp_file.write_all(content2)?;
    temp_file.flush()?;

    // Second detection: cache hit but different hash
    let change = detector.detect_change(file_id, temp_file.path()).await?;

    match change {
        ChangeType::Modified { old, new } => {
            assert_eq!(old, hash1);
            let expected_new = FileHasher::hash_bytes(content2);
            assert_eq!(new, expected_new);
        }
        _ => panic!("Expected ChangeType::Modified, got {:?}", change),
    }

    // Verify cache was updated with new hash
    let cached_hash = detector.cache().get(temp_file.path()).unwrap();
    let expected_new_hash = FileHasher::hash_bytes(content2);
    assert_eq!(*cached_hash, expected_new_hash);

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_db_hit_after_cache_miss() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file in database
    let file_id = insert_test_file(&pool, "test_repo_db_hit", "main", "test_db_hit.rs").await?;

    // Create temporary file
    let mut temp_file = NamedTempFile::new()?;
    let content = b"database stored content";
    temp_file.write_all(content)?;
    temp_file.flush()?;

    // Store hash in database (simulating previous run)
    let hash = FileHasher::hash_file(temp_file.path())?;
    store_hash_in_db(&pool, file_id, hash).await?;

    // Create detector with empty cache
    let mut detector = ChangeDetector::new(pool.clone());
    assert!(!detector.cache().contains(temp_file.path()));

    // Detect change: cache miss → database hit → no change
    let change = detector.detect_change(file_id, temp_file.path()).await?;
    assert_eq!(change, ChangeType::None);

    // Verify cache was populated
    assert!(detector.cache().contains(temp_file.path()));

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_clear_cache() -> Result<()> {
    let pool = setup_pool().await?;

    let mut detector = ChangeDetector::with_capacity(pool.clone(), 10);

    // Manually populate cache
    let hash = FileHasher::hash_bytes(b"test");
    detector
        .cache_mut()
        .insert("/test/path.rs".into(), hash);

    assert_eq!(detector.cache().len(), 1);

    // Clear cache
    detector.clear_cache();

    assert_eq!(detector.cache().len(), 0);
    assert!(detector.cache().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_multiple_files_with_cache() -> Result<()> {
    let pool = setup_pool().await?;

    // Create multiple test files
    let file_id1 = insert_test_file(&pool, "test_repo_multi", "main", "test_multi1.rs").await?;
    let file_id2 = insert_test_file(&pool, "test_repo_multi", "main", "test_multi2.rs").await?;
    let file_id3 = insert_test_file(&pool, "test_repo_multi", "main", "test_multi3.rs").await?;

    // Create temporary files
    let mut temp_file1 = NamedTempFile::new()?;
    let mut temp_file2 = NamedTempFile::new()?;
    let mut temp_file3 = NamedTempFile::new()?;

    temp_file1.write_all(b"file 1 content")?;
    temp_file1.flush()?;
    temp_file2.write_all(b"file 2 content")?;
    temp_file2.flush()?;
    temp_file3.write_all(b"file 3 content")?;
    temp_file3.flush()?;

    let mut detector = ChangeDetector::new(pool.clone());

    // First pass: all new files
    let change1 = detector
        .detect_change(file_id1, temp_file1.path())
        .await?;
    let change2 = detector
        .detect_change(file_id2, temp_file2.path())
        .await?;
    let change3 = detector
        .detect_change(file_id3, temp_file3.path())
        .await?;

    assert!(matches!(change1, ChangeType::New(_)));
    assert!(matches!(change2, ChangeType::New(_)));
    assert!(matches!(change3, ChangeType::New(_)));

    // Verify all cached
    assert_eq!(detector.cache().len(), 3);

    // Second pass: all unchanged (cache hits)
    let change1 = detector
        .detect_change(file_id1, temp_file1.path())
        .await?;
    let change2 = detector
        .detect_change(file_id2, temp_file2.path())
        .await?;
    let change3 = detector
        .detect_change(file_id3, temp_file3.path())
        .await?;

    assert_eq!(change1, ChangeType::None);
    assert_eq!(change2, ChangeType::None);
    assert_eq!(change3, ChangeType::None);

    cleanup_test_file(&pool, file_id1).await?;
    cleanup_test_file(&pool, file_id2).await?;
    cleanup_test_file(&pool, file_id3).await?;

    Ok(())
}

#[tokio::test]
async fn test_hash_storage_32_bytes() -> Result<()> {
    let pool = setup_pool().await?;

    // Create test file
    let file_id = insert_test_file(&pool, "test_repo_32bytes", "main", "test_32bytes.rs").await?;

    // Store hash
    let hash = FileHasher::hash_bytes(b"test content for 32 byte verification");
    store_hash_in_db(&pool, file_id, hash).await?;

    // Retrieve and verify it's exactly 32 bytes
    let client = pool.get().await?;
    let row = client
        .query_one(
            "SELECT blake3_hash FROM maproom.files WHERE id = $1",
            &[&file_id],
        )
        .await?;

    let hash_bytes: Option<Vec<u8>> = row.get(0);
    assert!(hash_bytes.is_some());
    assert_eq!(hash_bytes.unwrap().len(), 32);

    cleanup_test_file(&pool, file_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_get_hash_nonexistent_file() -> Result<()> {
    let pool = setup_pool().await?;

    // Use a file_id that doesn't exist
    let result = get_hash_from_db(&pool, 999999999).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found in database"));

    Ok(())
}
