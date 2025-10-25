//! Priority-based update queue for incremental indexing.
//!
//! This module implements a thread-safe priority queue that manages update tasks
//! for the incremental indexing system. Features include:
//! - Priority-based task processing (High > Medium > Low)
//! - Task deduplication and merging
//! - Error handling with retry logic and exponential backoff
//! - Dead letter queue for tasks that fail repeatedly

use priority_queue::PriorityQueue;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, warn};

use super::task::{Priority, UpdateTask};

/// Maximum number of retry attempts before moving to dead letter queue.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (1 second).
const BASE_BACKOFF_SECS: u64 = 1;

/// Update queue for managing incremental indexing tasks.
///
/// The queue maintains three collections:
/// 1. Priority queue - pending tasks ordered by priority
/// 2. Processing set - tasks currently being processed (prevents duplicates)
/// 3. Dead letter queue - tasks that failed after max retries
///
/// # Thread Safety
///
/// This queue is NOT thread-safe on its own. Wrap it in `Arc<Mutex<UpdateQueue>>`
/// or `Arc<RwLock<UpdateQueue>>` for concurrent access from multiple threads.
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use crewchief_maproom::incremental::{UpdateQueue, UpdateTask, Trigger, ChangeType};
/// use crewchief_maproom::incremental::hash::FileHasher;
///
/// let mut queue = UpdateQueue::new();
///
/// let hash = FileHasher::hash_bytes(b"content");
/// let task = UpdateTask::new(
///     PathBuf::from("src/main.rs"),
///     ChangeType::New(hash),
///     Trigger::Save
/// );
///
/// queue.enqueue(task);
///
/// if let Some(task) = queue.dequeue() {
///     println!("Processing: {:?}", task.path);
///     // ... process the task ...
///     // On success:
///     queue.mark_completed(&task.path);
///     // On failure:
///     // queue.mark_failed(task, "error message");
/// }
/// ```
pub struct UpdateQueue {
    /// Priority queue of pending tasks.
    /// Tasks are ordered by priority (High > Medium > Low).
    queue: PriorityQueue<PathBuf, Priority>,

    /// Map of tasks by path for deduplication and merging.
    tasks: HashMap<PathBuf, UpdateTask>,

    /// Set of paths currently being processed.
    /// Prevents concurrent processing of the same file.
    processing: HashSet<PathBuf>,

    /// Dead letter queue - tasks that failed after max retries.
    /// These are stored for later inspection and potential retry.
    dead_letter: Vec<UpdateTask>,
}

impl UpdateQueue {
    /// Create a new empty update queue.
    ///
    /// # Returns
    /// A new queue with no pending tasks
    pub fn new() -> Self {
        Self {
            queue: PriorityQueue::new(),
            tasks: HashMap::new(),
            processing: HashSet::new(),
            dead_letter: Vec::new(),
        }
    }

    /// Create a queue with pre-allocated capacity.
    ///
    /// Use this when you know approximately how many tasks will be queued.
    ///
    /// # Arguments
    /// * `capacity` - Initial capacity for the queue
    ///
    /// # Returns
    /// A new queue with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: PriorityQueue::with_capacity(capacity),
            tasks: HashMap::with_capacity(capacity),
            processing: HashSet::with_capacity(capacity / 2), // Assume 50% concurrent processing
            dead_letter: Vec::new(),
        }
    }

    /// Enqueue a task for processing.
    ///
    /// If a task for the same path already exists in the queue, the tasks
    /// are merged according to the merge rules defined in `UpdateTask::merge`.
    ///
    /// # Arguments
    /// * `task` - The task to enqueue
    ///
    /// # Behavior
    /// - If path is currently being processed: task is queued for later
    /// - If path already in queue: tasks are merged (highest priority wins)
    /// - Otherwise: task is added to queue with its priority
    pub fn enqueue(&mut self, task: UpdateTask) {
        let path = task.path.clone();
        let priority = task.priority;

        // Check if we already have a task for this path
        if let Some(existing) = self.tasks.get_mut(&path) {
            debug!(
                "Merging task for path: {} (existing priority: {:?}, new priority: {:?})",
                path.display(),
                existing.priority,
                priority
            );

            // Merge the new task into the existing one
            existing.merge(task);

            // Update priority in the queue if it changed
            if existing.priority != priority {
                self.queue.change_priority(&path, existing.priority);
            }
        } else {
            debug!(
                "Enqueueing new task for path: {} (priority: {:?})",
                path.display(),
                priority
            );

            // New task - add to queue and tasks map
            self.queue.push(path.clone(), priority);
            self.tasks.insert(path, task);
        }
    }

    /// Dequeue the highest priority task.
    ///
    /// Returns the task with the highest priority that is not currently
    /// being processed. The task is moved to the processing set.
    ///
    /// # Returns
    /// * `Some(task)` - The highest priority task
    /// * `None` - Queue is empty or all tasks are being processed
    pub fn dequeue(&mut self) -> Option<UpdateTask> {
        // Get the highest priority task
        while let Some((path, _priority)) = self.queue.pop() {
            // Skip if already being processed (shouldn't happen, but safety check)
            if self.processing.contains(&path) {
                warn!(
                    "Task for {} already in processing set, skipping",
                    path.display()
                );
                continue;
            }

            // Remove from tasks map
            if let Some(task) = self.tasks.remove(&path) {
                // Add to processing set
                self.processing.insert(path.clone());

                debug!(
                    "Dequeued task for path: {} (priority: {:?})",
                    path.display(),
                    task.priority
                );

                return Some(task);
            }
        }

        None
    }

    /// Mark a task as completed successfully.
    ///
    /// Removes the task from the processing set.
    ///
    /// # Arguments
    /// * `path` - Path of the completed task
    pub fn mark_completed(&mut self, path: &Path) {
        if self.processing.remove(path) {
            debug!("Marked task as completed: {}", path.display());
        }
    }

    /// Mark a task as failed and handle retry logic.
    ///
    /// If the task has not exceeded the maximum retry count, it is
    /// re-enqueued with an incremented retry count. Otherwise, it is
    /// moved to the dead letter queue.
    ///
    /// # Arguments
    /// * `task` - The failed task
    /// * `error` - Error message describing the failure
    pub fn mark_failed(&mut self, mut task: UpdateTask, error: &str) {
        // Remove from processing set
        self.processing.remove(&task.path);

        task.increment_retry();

        if task.has_exceeded_retries(MAX_RETRIES) {
            warn!(
                "Task exceeded max retries ({}), moving to dead letter queue: {} - last error: {}",
                MAX_RETRIES,
                task.path.display(),
                error
            );
            self.dead_letter.push(task);
        } else {
            debug!(
                "Re-enqueueing failed task: {} (retry {}/{}) - error: {}",
                task.path.display(),
                task.retry_count,
                MAX_RETRIES,
                error
            );

            // Re-enqueue with exponential backoff
            // Note: The caller should handle the delay
            self.enqueue(task);
        }
    }

    /// Get a reference to the dead letter queue.
    ///
    /// The dead letter queue contains tasks that failed after the maximum
    /// number of retries. These can be inspected for debugging or manually
    /// retried.
    ///
    /// # Returns
    /// Slice of failed tasks
    pub fn get_dead_letter_queue(&self) -> &[UpdateTask] {
        &self.dead_letter
    }

    /// Get the number of tasks currently in the queue.
    ///
    /// This does not include tasks that are currently being processed
    /// or tasks in the dead letter queue.
    ///
    /// # Returns
    /// Number of pending tasks
    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }

    /// Get the number of tasks currently being processed.
    ///
    /// # Returns
    /// Number of tasks in the processing set
    pub fn processing_size(&self) -> usize {
        self.processing.len()
    }

    /// Get the number of tasks in the dead letter queue.
    ///
    /// # Returns
    /// Number of failed tasks
    pub fn dead_letter_size(&self) -> usize {
        self.dead_letter.len()
    }

    /// Check if the queue is empty.
    ///
    /// Returns true if there are no pending tasks. This does not include
    /// tasks currently being processed.
    ///
    /// # Returns
    /// True if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Clear all pending tasks from the queue.
    ///
    /// This does NOT affect tasks currently being processed or tasks
    /// in the dead letter queue.
    pub fn clear(&mut self) {
        debug!("Clearing {} tasks from queue", self.queue.len());
        self.queue.clear();
        self.tasks.clear();
    }

    /// Clear the dead letter queue.
    ///
    /// Use this to remove old failed tasks that have been manually
    /// resolved or are no longer relevant.
    pub fn clear_dead_letter(&mut self) {
        debug!("Clearing {} tasks from dead letter queue", self.dead_letter.len());
        self.dead_letter.clear();
    }

    /// Calculate the exponential backoff delay for a task.
    ///
    /// The delay increases exponentially with the retry count:
    /// - Retry 1: 1 second
    /// - Retry 2: 2 seconds
    /// - Retry 3: 4 seconds
    ///
    /// # Arguments
    /// * `retry_count` - Number of retry attempts
    ///
    /// # Returns
    /// Duration to wait before retrying
    pub fn calculate_backoff(retry_count: u32) -> Duration {
        let delay_secs = BASE_BACKOFF_SECS * 2_u64.pow(retry_count.saturating_sub(1));
        Duration::from_secs(delay_secs)
    }

    /// Retry all tasks in the dead letter queue.
    ///
    /// Moves all tasks from the dead letter queue back to the main queue
    /// and resets their retry counts. Use this for manual recovery after
    /// fixing the underlying issue.
    ///
    /// # Returns
    /// Number of tasks re-enqueued
    pub fn retry_dead_letter(&mut self) -> usize {
        let count = self.dead_letter.len();

        debug!("Retrying {} tasks from dead letter queue", count);

        // Collect tasks first to avoid borrowing issue
        let tasks: Vec<UpdateTask> = self.dead_letter.drain(..).collect();

        for mut task in tasks {
            // Reset retry count for fresh attempt
            task.retry_count = 0;
            self.enqueue(task);
        }

        count
    }

    /// Get statistics about the queue state.
    ///
    /// # Returns
    /// Tuple of (pending, processing, dead_letter) counts
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            pending: self.queue_size(),
            processing: self.processing_size(),
            dead_letter: self.dead_letter_size(),
        }
    }
}

impl Default for UpdateQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the queue state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueStats {
    /// Number of tasks pending in the queue
    pub pending: usize,
    /// Number of tasks currently being processed
    pub processing: usize,
    /// Number of tasks in the dead letter queue
    pub dead_letter: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incremental::detector::ChangeType;
    use crate::incremental::hash::FileHasher;
    use crate::incremental::task::Trigger;

    #[test]
    fn test_queue_new() {
        let queue = UpdateQueue::new();
        assert_eq!(queue.queue_size(), 0);
        assert_eq!(queue.processing_size(), 0);
        assert_eq!(queue.dead_letter_size(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_enqueue_dequeue() {
        let mut queue = UpdateQueue::new();
        let path = PathBuf::from("/test/file.rs");
        let hash = FileHasher::hash_bytes(b"test");

        let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);

        queue.enqueue(task);
        assert_eq!(queue.queue_size(), 1);

        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().path, path);
        assert_eq!(queue.queue_size(), 0);
        assert_eq!(queue.processing_size(), 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut queue = UpdateQueue::new();
        let hash = FileHasher::hash_bytes(b"test");

        // Enqueue tasks with different priorities
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

        // Enqueue in random order
        queue.enqueue(low_task);
        queue.enqueue(high_task);
        queue.enqueue(medium_task);

        assert_eq!(queue.queue_size(), 3);

        // Dequeue should return in priority order
        let task1 = queue.dequeue().unwrap();
        assert_eq!(task1.priority, Priority::High);

        let task2 = queue.dequeue().unwrap();
        assert_eq!(task2.priority, Priority::Medium);

        let task3 = queue.dequeue().unwrap();
        assert_eq!(task3.priority, Priority::Low);

        assert!(queue.is_empty());
    }

    #[test]
    fn test_task_deduplication() {
        let mut queue = UpdateQueue::new();
        let path = PathBuf::from("/test/file.rs");
        let hash1 = FileHasher::hash_bytes(b"v1");
        let hash2 = FileHasher::hash_bytes(b"v2");

        // Enqueue first task
        let task1 = UpdateTask::new(path.clone(), ChangeType::New(hash1), Trigger::Auto);
        queue.enqueue(task1);

        assert_eq!(queue.queue_size(), 1);

        // Enqueue second task for same path with higher priority
        let task2 = UpdateTask::new(
            path.clone(),
            ChangeType::Modified {
                old: hash1,
                new: hash2,
            },
            Trigger::User,
        );
        queue.enqueue(task2);

        // Should still have only 1 task
        assert_eq!(queue.queue_size(), 1);

        // Should have merged to higher priority
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
    fn test_mark_failed_with_retry() {
        let mut queue = UpdateQueue::new();
        let path = PathBuf::from("/test/file.rs");
        let hash = FileHasher::hash_bytes(b"test");

        let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
        queue.enqueue(task);

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(queue.queue_size(), 0);
        assert_eq!(queue.processing_size(), 1);

        // Mark as failed - should be re-enqueued
        queue.mark_failed(dequeued, "test error");
        assert_eq!(queue.queue_size(), 1);
        assert_eq!(queue.processing_size(), 0);
        assert_eq!(queue.dead_letter_size(), 0);

        // Dequeue again and check retry count
        let retry_task = queue.dequeue().unwrap();
        assert_eq!(retry_task.retry_count, 1);
    }

    #[test]
    fn test_dead_letter_queue() {
        let mut queue = UpdateQueue::new();
        let path = PathBuf::from("/test/file.rs");
        let hash = FileHasher::hash_bytes(b"test");

        let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
        queue.enqueue(task);

        // Fail the task MAX_RETRIES times
        for i in 0..MAX_RETRIES {
            let dequeued = queue.dequeue().unwrap();
            assert_eq!(dequeued.retry_count, i);
            queue.mark_failed(dequeued, "persistent error");
        }

        // After MAX_RETRIES, should be in dead letter queue
        assert_eq!(queue.queue_size(), 0);
        assert_eq!(queue.dead_letter_size(), 1);

        let dead_letter = queue.get_dead_letter_queue();
        assert_eq!(dead_letter[0].path, path);
        assert_eq!(dead_letter[0].retry_count, MAX_RETRIES);
    }

    #[test]
    fn test_calculate_backoff() {
        assert_eq!(UpdateQueue::calculate_backoff(0), Duration::from_secs(1));
        assert_eq!(UpdateQueue::calculate_backoff(1), Duration::from_secs(1));
        assert_eq!(UpdateQueue::calculate_backoff(2), Duration::from_secs(2));
        assert_eq!(UpdateQueue::calculate_backoff(3), Duration::from_secs(4));
        assert_eq!(UpdateQueue::calculate_backoff(4), Duration::from_secs(8));
    }

    #[test]
    fn test_retry_dead_letter() {
        let mut queue = UpdateQueue::new();
        let path = PathBuf::from("/test/file.rs");
        let hash = FileHasher::hash_bytes(b"test");

        let task = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Save);
        queue.enqueue(task);

        // Move to dead letter queue
        for _ in 0..MAX_RETRIES {
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
    fn test_clear() {
        let mut queue = UpdateQueue::new();
        let hash = FileHasher::hash_bytes(b"test");

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
    fn test_stats() {
        let mut queue = UpdateQueue::new();
        let hash = FileHasher::hash_bytes(b"test");

        // Add 3 tasks
        for i in 0..3 {
            let task = UpdateTask::new(
                PathBuf::from(format!("/test/file{}.rs", i)),
                ChangeType::New(hash),
                Trigger::Auto,
            );
            queue.enqueue(task);
        }

        // Dequeue 1 (goes to processing)
        let _processing_task = queue.dequeue();

        // Fail 1 task MAX_RETRIES times (goes to dead letter)
        // Use User trigger so it has high priority and gets dequeued first
        let task = UpdateTask::new(
            PathBuf::from("/test/failed.rs"),
            ChangeType::New(hash),
            Trigger::User, // High priority
        );
        queue.enqueue(task);

        for _ in 0..MAX_RETRIES {
            let dequeued = queue.dequeue().unwrap();
            queue.mark_failed(dequeued, "error");
        }

        let stats = queue.stats();
        assert_eq!(stats.pending, 2); // 3 - 1 dequeued
        assert_eq!(stats.processing, 1); // _processing_task
        assert_eq!(stats.dead_letter, 1);
    }
}
