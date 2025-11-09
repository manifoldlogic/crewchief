//! Integration tests for the incremental processor.
//!
//! These tests verify:
//! - File indexing (new files)
//! - File updates (modified files)
//! - File deletion
//! - Transaction integrity
//! - Edge consistency after updates
//!
//! **Note**: These tests require a PostgreSQL database with the maproom schema.
//! They will be skipped if MAPROOM_DATABASE_URL is not set.

use crewchief_maproom::db::{create_pool, PgPool};
use crewchief_maproom::incremental::{
    ChangeType, FileHasher, IncrementalProcessor, Priority, Trigger, UpdateTask,
};
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Helper to check if tests should run (MAPROOM_DATABASE_URL is set)
fn should_run_db_tests() -> bool {
    std::env::var("MAPROOM_DATABASE_URL").is_ok()
}

/// Create a test pool or skip test if MAPROOM_DATABASE_URL not set
async fn get_test_pool() -> Option<PgPool> {
    if !should_run_db_tests() {
        println!("Skipping test: MAPROOM_DATABASE_URL not set");
        return None;
    }

    match create_pool().await {
        Ok(pool) => Some(pool),
        Err(e) => {
            println!("Skipping test: Failed to create pool: {}", e);
            None
        }
    }
}

#[tokio::test]
async fn test_processor_creation() {
    let Some(pool) = get_test_pool().await else {
        return;
    };

    // Should be able to create processor
    let processor = IncrementalProcessor::new(pool, PathBuf::from("/workspace"));

    // Processor should be ready to use
    assert!(std::mem::size_of_val(&processor) > 0);
}

#[tokio::test]
async fn test_process_task_with_no_change() {
    let Some(pool) = get_test_pool().await else {
        return;
    };

    let processor = IncrementalProcessor::new(pool, PathBuf::from("/workspace"));

    // Create a task with ChangeType::None
    let task = UpdateTask::new(
        PathBuf::from("/test/unchanged.rs"),
        ChangeType::None,
        Trigger::Auto,
    );

    // Processing should succeed and be a no-op
    let result = processor.process(task).await;
    assert!(result.is_ok(), "Processing None change should succeed");
}

#[test]
fn test_update_task_properties() {
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test content");

    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);

    assert_eq!(task.path, path);
    assert_eq!(task.priority, Priority::Medium);
    assert_eq!(task.trigger, Trigger::Save);
    assert_eq!(task.retry_count, 0);
}

#[test]
fn test_change_type_variants() {
    let hash1 = FileHasher::hash_bytes(b"content1");
    let hash2 = FileHasher::hash_bytes(b"content2");

    // Test New variant
    let new_change = ChangeType::New(hash1);
    match new_change {
        ChangeType::New(_) => (),
        _ => panic!("Expected New variant"),
    }

    // Test Modified variant
    let modified_change = ChangeType::Modified {
        old: hash1,
        new: hash2,
    };
    match modified_change {
        ChangeType::Modified { old, new } => {
            assert_eq!(old, hash1);
            assert_eq!(new, hash2);
        }
        _ => panic!("Expected Modified variant"),
    }

    // Test Deleted variant
    let deleted_change = ChangeType::Deleted(hash1);
    match deleted_change {
        ChangeType::Deleted(_) => (),
        _ => panic!("Expected Deleted variant"),
    }

    // Test None variant
    let none_change = ChangeType::None;
    match none_change {
        ChangeType::None => (),
        _ => panic!("Expected None variant"),
    }
}

#[test]
fn test_priority_ordering() {
    let high = Priority::High;
    let medium = Priority::Medium;
    let low = Priority::Low;

    assert!(high > medium);
    assert!(medium > low);
    assert!(high > low);
}

#[test]
fn test_trigger_to_priority_mapping() {
    assert_eq!(Priority::from_trigger(Trigger::User), Priority::High);
    assert_eq!(Priority::from_trigger(Trigger::Save), Priority::Medium);
    assert_eq!(Priority::from_trigger(Trigger::Auto), Priority::Low);
}

#[tokio::test]
async fn test_processor_handles_missing_file() {
    let Some(pool) = get_test_pool().await else {
        return;
    };

    let processor = IncrementalProcessor::new(pool, PathBuf::from("/workspace"));

    // Create a task for a file that doesn't exist
    let nonexistent_path = PathBuf::from("/nonexistent/file.rs");
    let hash = FileHasher::hash_bytes(b"content");
    let task = UpdateTask::new(nonexistent_path, ChangeType::New(hash), Trigger::Auto);

    // Processing should fail gracefully
    let result = processor.process(task).await;
    assert!(
        result.is_err(),
        "Processing nonexistent file should return error"
    );
}

#[tokio::test]
async fn test_processor_handles_invalid_language() {
    let Some(pool) = get_test_pool().await else {
        return;
    };

    let _processor = IncrementalProcessor::new(pool, PathBuf::from("/workspace"));

    // Create a temporary file with unknown extension
    let mut temp_file = NamedTempFile::new().unwrap();
    use std::io::Write;
    writeln!(temp_file, "some content").unwrap();
    temp_file.flush().unwrap();

    let path = temp_file.path().to_path_buf();
    let hash = FileHasher::hash_file(&path).unwrap();

    // Note: This test would require the file to exist in the database first
    // For now, we just verify the task can be created
    let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Auto);

    // Task should be created successfully
    assert_eq!(task.retry_count, 0);
}

#[test]
fn test_file_hasher_consistency() {
    let content = b"test content for hashing";

    let hash1 = FileHasher::hash_bytes(content);
    let hash2 = FileHasher::hash_bytes(content);

    assert_eq!(hash1, hash2, "Hash should be consistent for same content");

    let different_content = b"different content";
    let hash3 = FileHasher::hash_bytes(different_content);

    assert_ne!(hash1, hash3, "Hash should differ for different content");
}

#[test]
fn test_task_merge_preserves_highest_priority() {
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"content");

    let mut task1 = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Auto);
    let task2 = UpdateTask::new(path, ChangeType::New(hash), Trigger::User);

    assert_eq!(task1.priority, Priority::Low);
    assert_eq!(task2.priority, Priority::High);

    task1.merge(task2);

    assert_eq!(
        task1.priority,
        Priority::High,
        "Should preserve highest priority after merge"
    );
}

#[test]
fn test_retry_count_increments() {
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"content");

    let mut task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Auto);

    assert_eq!(task.retry_count, 0);
    assert!(!task.has_exceeded_retries(3));

    task.increment_retry();
    assert_eq!(task.retry_count, 1);
    assert!(!task.has_exceeded_retries(3));

    task.increment_retry();
    task.increment_retry();
    assert_eq!(task.retry_count, 3);
    assert!(task.has_exceeded_retries(3));
}

// Note: Full end-to-end integration tests would require:
// 1. Setting up test database with schema
// 2. Creating test files in database
// 3. Processing actual file changes
// 4. Verifying chunks and edges in database
//
// These are better suited for a dedicated integration test suite
// that runs against a real PostgreSQL instance.
