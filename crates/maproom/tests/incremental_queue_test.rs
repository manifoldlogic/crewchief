//! Comprehensive unit tests for the incremental indexing queue system.
//!
//! This test module covers:
//! - Basic queue operations (enqueue, dequeue)
//! - Priority ordering (High > Medium > Low)
//! - Task deduplication and merging
//! - Error handling and retry logic
//! - Dead letter queue behavior
//! - Exponential backoff calculation
//! - Queue statistics

use maproom::incremental::{
    ChangeType, FileHasher, Priority, Trigger, UpdateQueue, UpdateTask,
};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn test_queue_creation() {
    let queue = UpdateQueue::new();
    assert_eq!(queue.queue_size(), 0);
    assert_eq!(queue.processing_size(), 0);
    assert_eq!(queue.dead_letter_size(), 0);
    assert!(queue.is_empty());
}

#[test]
fn test_queue_with_capacity() {
    let queue = UpdateQueue::with_capacity(100);
    assert_eq!(queue.queue_size(), 0);
    assert!(queue.is_empty());
}

#[test]
fn test_basic_enqueue_dequeue() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test content");

    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);

    queue.enqueue(task);
    assert_eq!(queue.queue_size(), 1);
    assert!(!queue.is_empty());

    let dequeued = queue.dequeue();
    assert!(dequeued.is_some());

    let dequeued = dequeued.unwrap();
    assert_eq!(dequeued.path, path);
    assert_eq!(dequeued.priority, Priority::Medium);
    assert_eq!(dequeued.trigger, Trigger::Save);

    assert_eq!(queue.queue_size(), 0);
    assert_eq!(queue.processing_size(), 1);
}

#[test]
fn test_priority_ordering_high_medium_low() {
    let mut queue = UpdateQueue::new();
    let hash = FileHasher::hash_bytes(b"test");

    // Enqueue tasks in non-priority order
    let low_task = UpdateTask::new(
        PathBuf::from("/test/low.rs"),
        ChangeType::New(hash),
        Trigger::Auto,
    );
    let high_task = UpdateTask::new(
        PathBuf::from("/test/high.rs"),
        ChangeType::New(hash),
        Trigger::User,
    );
    let medium_task = UpdateTask::new(
        PathBuf::from("/test/medium.rs"),
        ChangeType::New(hash),
        Trigger::Save,
    );

    queue.enqueue(low_task);
    queue.enqueue(high_task);
    queue.enqueue(medium_task);

    assert_eq!(queue.queue_size(), 3);

    // Dequeue should return High first
    let task1 = queue.dequeue().unwrap();
    assert_eq!(task1.priority, Priority::High);
    assert_eq!(task1.path, PathBuf::from("/test/high.rs"));

    // Then Medium
    let task2 = queue.dequeue().unwrap();
    assert_eq!(task2.priority, Priority::Medium);
    assert_eq!(task2.path, PathBuf::from("/test/medium.rs"));

    // Then Low
    let task3 = queue.dequeue().unwrap();
    assert_eq!(task3.priority, Priority::Low);
    assert_eq!(task3.path, PathBuf::from("/test/low.rs"));

    assert!(queue.is_empty());
}

#[test]
fn test_task_deduplication_same_path() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash1 = FileHasher::hash_bytes(b"version 1");
    let hash2 = FileHasher::hash_bytes(b"version 2");

    // Enqueue first task
    let task1 = UpdateTask::new(path.clone(), ChangeType::New(hash1), Trigger::Auto);
    queue.enqueue(task1);

    assert_eq!(queue.queue_size(), 1);

    // Enqueue second task for same path
    let task2 = UpdateTask::new(
        path.clone(),
        ChangeType::Modified {
            old: hash1,
            new: hash2,
        },
        Trigger::User,
    );
    queue.enqueue(task2);

    // Should still have only 1 task (deduplicated)
    assert_eq!(queue.queue_size(), 1);

    // Should have merged to higher priority
    let dequeued = queue.dequeue().unwrap();
    assert_eq!(dequeued.priority, Priority::High);
    assert_eq!(dequeued.trigger, Trigger::User);
}

#[test]
fn test_task_merging_priority_upgrade() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test");

    // Enqueue low priority task
    let low_task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Auto);
    queue.enqueue(low_task);

    // Enqueue medium priority task for same path
    let medium_task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
    queue.enqueue(medium_task);

    // Enqueue high priority task for same path
    let high_task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::User);
    queue.enqueue(high_task);

    // Should have only 1 task with High priority
    assert_eq!(queue.queue_size(), 1);
    let dequeued = queue.dequeue().unwrap();
    assert_eq!(dequeued.priority, Priority::High);
}

#[test]
fn test_mark_completed() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test");

    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
    queue.enqueue(task);

    let dequeued = queue.dequeue().unwrap();
    assert_eq!(queue.processing_size(), 1);

    queue.mark_completed(&dequeued.path);
    assert_eq!(queue.processing_size(), 0);
}

#[test]
fn test_mark_failed_retry_once() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test");

    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
    queue.enqueue(task);

    let dequeued = queue.dequeue().unwrap();
    assert_eq!(queue.queue_size(), 0);
    assert_eq!(queue.processing_size(), 1);

    // Mark as failed - should be re-enqueued
    queue.mark_failed(dequeued, "network error");

    assert_eq!(queue.queue_size(), 1);
    assert_eq!(queue.processing_size(), 0);
    assert_eq!(queue.dead_letter_size(), 0);

    // Dequeue again and check retry count
    let retry_task = queue.dequeue().unwrap();
    assert_eq!(retry_task.retry_count, 1);
    assert_eq!(retry_task.path, path);
}

#[test]
fn test_mark_failed_multiple_retries() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test");

    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
    queue.enqueue(task);

    // First attempt
    let task1 = queue.dequeue().unwrap();
    assert_eq!(task1.retry_count, 0);
    queue.mark_failed(task1, "error 1");

    // Second attempt
    let task2 = queue.dequeue().unwrap();
    assert_eq!(task2.retry_count, 1);
    queue.mark_failed(task2, "error 2");

    // Third attempt
    let task3 = queue.dequeue().unwrap();
    assert_eq!(task3.retry_count, 2);
    queue.mark_failed(task3, "error 3");

    // After 3 retries (max), should be in dead letter queue
    assert_eq!(queue.queue_size(), 0);
    assert_eq!(queue.dead_letter_size(), 1);

    let dead_letter = queue.get_dead_letter_queue();
    assert_eq!(dead_letter[0].path, path);
    assert_eq!(dead_letter[0].retry_count, 3);
}

#[test]
fn test_dead_letter_queue_workflow() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/persistent_error.rs");
    let hash = FileHasher::hash_bytes(b"test");

    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::User);
    queue.enqueue(task);

    // Fail the task 3 times (MAX_RETRIES)
    for i in 0..3 {
        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.retry_count, i);
        queue.mark_failed(dequeued, "persistent error");
    }

    // Should be in dead letter queue now
    assert_eq!(queue.queue_size(), 0);
    assert_eq!(queue.dead_letter_size(), 1);

    let dead_letter = queue.get_dead_letter_queue();
    assert_eq!(dead_letter.len(), 1);
    assert_eq!(dead_letter[0].path, path);
}

#[test]
fn test_retry_dead_letter_queue() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test");

    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
    queue.enqueue(task);

    // Move to dead letter queue
    for _ in 0..3 {
        let dequeued = queue.dequeue().unwrap();
        queue.mark_failed(dequeued, "error");
    }

    assert_eq!(queue.dead_letter_size(), 1);
    assert_eq!(queue.queue_size(), 0);

    // Retry dead letter queue
    let count = queue.retry_dead_letter();
    assert_eq!(count, 1);
    assert_eq!(queue.dead_letter_size(), 0);
    assert_eq!(queue.queue_size(), 1);

    // Verify retry count was reset
    let task = queue.dequeue().unwrap();
    assert_eq!(task.retry_count, 0);
}

#[test]
fn test_calculate_backoff() {
    assert_eq!(UpdateQueue::calculate_backoff(0), Duration::from_secs(1));
    assert_eq!(UpdateQueue::calculate_backoff(1), Duration::from_secs(1));
    assert_eq!(UpdateQueue::calculate_backoff(2), Duration::from_secs(2));
    assert_eq!(UpdateQueue::calculate_backoff(3), Duration::from_secs(4));
    assert_eq!(UpdateQueue::calculate_backoff(4), Duration::from_secs(8));
    assert_eq!(UpdateQueue::calculate_backoff(5), Duration::from_secs(16));
}

#[test]
fn test_clear_queue() {
    let mut queue = UpdateQueue::new();
    let hash = FileHasher::hash_bytes(b"test");

    // Add 5 tasks
    for i in 0..5 {
        let task = UpdateTask::new(
            PathBuf::from(format!("/test/file{}.rs", i)),
            ChangeType::New(hash),
            Trigger::Auto,
        );
        queue.enqueue(task);
    }

    assert_eq!(queue.queue_size(), 5);

    queue.clear();
    assert_eq!(queue.queue_size(), 0);
    assert!(queue.is_empty());
}

#[test]
fn test_clear_dead_letter() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test");

    let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);
    queue.enqueue(task);

    // Move to dead letter queue
    for _ in 0..3 {
        let dequeued = queue.dequeue().unwrap();
        queue.mark_failed(dequeued, "error");
    }

    assert_eq!(queue.dead_letter_size(), 1);

    queue.clear_dead_letter();
    assert_eq!(queue.dead_letter_size(), 0);
}

#[test]
fn test_queue_stats() {
    let mut queue = UpdateQueue::new();
    let hash = FileHasher::hash_bytes(b"test");

    // Add 3 tasks to queue
    for i in 0..3 {
        let task = UpdateTask::new(
            PathBuf::from(format!("/test/pending{}.rs", i)),
            ChangeType::New(hash),
            Trigger::Auto,
        );
        queue.enqueue(task);
    }

    // Dequeue 1 (goes to processing)
    let processing_task = queue.dequeue().unwrap();

    // Add and fail a task until it goes to dead letter
    // Use User trigger so it has high priority and gets dequeued first
    let failed_task = UpdateTask::new(
        PathBuf::from("/test/failed.rs"),
        ChangeType::New(hash),
        Trigger::User, // High priority
    );
    queue.enqueue(failed_task);

    // Fail the high-priority task 3 times
    for _ in 0..3 {
        let task = queue.dequeue().unwrap();
        // Should be the failed.rs task due to high priority
        assert_eq!(task.path, PathBuf::from("/test/failed.rs"));
        queue.mark_failed(task, "error");
    }

    let stats = queue.stats();
    assert_eq!(stats.pending, 2); // 3 original - 1 dequeued to processing
    assert_eq!(stats.processing, 1); // processing_task still there
    assert_eq!(stats.dead_letter, 1); // failed task after 3 retries

    // Clean up
    queue.mark_completed(&processing_task.path);
}

#[test]
fn test_multiple_tasks_different_priorities() {
    let mut queue = UpdateQueue::new();
    let hash = FileHasher::hash_bytes(b"test");

    // Add 10 tasks with mixed priorities
    for i in 0..10 {
        let trigger = match i % 3 {
            0 => Trigger::User,
            1 => Trigger::Save,
            _ => Trigger::Auto,
        };

        let task = UpdateTask::new(
            PathBuf::from(format!("/test/file{}.rs", i)),
            ChangeType::New(hash),
            trigger,
        );
        queue.enqueue(task);
    }

    assert_eq!(queue.queue_size(), 10);

    // All User tasks should come first
    let mut dequeued_priorities = Vec::new();
    while let Some(task) = queue.dequeue() {
        dequeued_priorities.push(task.priority);
        queue.mark_completed(&task.path);
    }

    // Verify priorities are in descending order
    for i in 1..dequeued_priorities.len() {
        assert!(dequeued_priorities[i - 1] >= dequeued_priorities[i]);
    }
}

#[test]
fn test_empty_queue_dequeue() {
    let mut queue = UpdateQueue::new();
    assert!(queue.dequeue().is_none());
}

#[test]
fn test_change_type_merging_in_queue() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash1 = FileHasher::hash_bytes(b"v1");
    let hash2 = FileHasher::hash_bytes(b"v2");

    // Enqueue New task
    let task1 = UpdateTask::new(path.clone(), ChangeType::New(hash1), Trigger::Auto);
    queue.enqueue(task1);

    // Enqueue Modified task for same file
    let task2 = UpdateTask::new(
        path.clone(),
        ChangeType::Modified {
            old: hash1,
            new: hash2,
        },
        Trigger::Save,
    );
    queue.enqueue(task2);

    // Should merge to New with updated hash
    assert_eq!(queue.queue_size(), 1);
    let dequeued = queue.dequeue().unwrap();

    match dequeued.change_type {
        ChangeType::New(hash) => assert_eq!(hash, hash2),
        _ => panic!("Expected New change type after merging"),
    }
}

#[test]
fn test_concurrent_processing_prevention() {
    let mut queue = UpdateQueue::new();
    let path = PathBuf::from("/test/file.rs");
    let hash = FileHasher::hash_bytes(b"test");

    let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
    queue.enqueue(task);

    // Dequeue task (goes to processing)
    let task1 = queue.dequeue().unwrap();
    assert_eq!(queue.processing_size(), 1);

    // Try to enqueue another task for the same path
    let task2 = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::User);
    queue.enqueue(task2);

    // The new task should be queued (not merged with processing task)
    assert_eq!(queue.queue_size(), 1);
    assert_eq!(queue.processing_size(), 1);

    // Complete the first task
    queue.mark_completed(&task1.path);
    assert_eq!(queue.processing_size(), 0);

    // Now the queued task can be processed
    let task3 = queue.dequeue().unwrap();
    assert_eq!(task3.path, path);
}

#[test]
fn test_stats_consistency() {
    let mut queue = UpdateQueue::new();
    let hash = FileHasher::hash_bytes(b"test");

    // Initial stats
    let stats = queue.stats();
    assert_eq!(stats.pending, 0);
    assert_eq!(stats.processing, 0);
    assert_eq!(stats.dead_letter, 0);

    // Add a task
    let task = UpdateTask::new(
        PathBuf::from("/test/file.rs"),
        ChangeType::New(hash),
        Trigger::Save,
    );
    queue.enqueue(task);

    let stats = queue.stats();
    assert_eq!(stats.pending, 1);
    assert_eq!(stats.processing, 0);

    // Dequeue
    let task = queue.dequeue().unwrap();
    let stats = queue.stats();
    assert_eq!(stats.pending, 0);
    assert_eq!(stats.processing, 1);

    // Mark completed
    queue.mark_completed(&task.path);
    let stats = queue.stats();
    assert_eq!(stats.pending, 0);
    assert_eq!(stats.processing, 0);
}

#[test]
fn test_default_trait() {
    let queue = UpdateQueue::default();
    assert_eq!(queue.queue_size(), 0);
    assert!(queue.is_empty());
}
