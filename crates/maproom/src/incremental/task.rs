//! Update task types for incremental indexing queue.
//!
//! This module defines the core types for the update queue system:
//! - `UpdateTask` - A file update task with metadata
//! - `Trigger` - What triggered the update (User, Save, Auto)
//! - `Priority` - Task priority level (High, Medium, Low)

use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::path::PathBuf;

use super::detector::ChangeType;

/// What triggered this update task.
///
/// The trigger type determines the priority of the task:
/// - User: Manual user action (e.g., "reindex this file") → High priority
/// - Save: File save event from editor → Medium priority
/// - Auto: Automatic detection from file watcher → Low priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trigger {
    /// User-triggered action (highest priority)
    User,
    /// File save event (medium priority)
    Save,
    /// Automatic detection (lowest priority)
    Auto,
}

/// Priority level for update tasks.
///
/// Priority determines the order in which tasks are processed:
/// - High: Process immediately (user-triggered)
/// - Medium: Process soon (save events)
/// - Low: Process eventually (automatic detection)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Priority {
    /// High priority - process immediately
    High,
    /// Medium priority - process soon
    Medium,
    /// Low priority - process eventually
    Low,
}

impl Priority {
    /// Calculate priority from a trigger type.
    ///
    /// # Arguments
    /// * `trigger` - The trigger that created the task
    ///
    /// # Returns
    /// The appropriate priority level for the trigger
    pub fn from_trigger(trigger: Trigger) -> Self {
        match trigger {
            Trigger::User => Priority::High,
            Trigger::Save => Priority::Medium,
            Trigger::Auto => Priority::Low,
        }
    }

    /// Get a numeric value for priority comparison.
    ///
    /// Higher values = higher priority.
    /// Used internally by the priority queue.
    fn as_value(&self) -> u8 {
        match self {
            Priority::High => 3,
            Priority::Medium => 2,
            Priority::Low => 1,
        }
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_value().cmp(&other.as_value())
    }
}

/// An update task for the incremental indexing queue.
///
/// Each task represents a file that needs to be reindexed, along with
/// metadata about what changed and why.
#[derive(Debug, Clone)]
pub struct UpdateTask {
    /// Filesystem path to the file
    pub path: PathBuf,
    /// Type of change detected
    pub change_type: ChangeType,
    /// What triggered this update
    pub trigger: Trigger,
    /// Priority level (calculated from trigger)
    pub priority: Priority,
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// Number of retry attempts (for error handling)
    pub retry_count: u32,
}

impl UpdateTask {
    /// Create a new update task.
    ///
    /// # Arguments
    /// * `path` - Filesystem path to the file
    /// * `change_type` - Type of change detected
    /// * `trigger` - What triggered this update
    ///
    /// # Returns
    /// A new update task with priority calculated from the trigger
    pub fn new(path: PathBuf, change_type: ChangeType, trigger: Trigger) -> Self {
        Self {
            path,
            change_type,
            trigger,
            priority: Priority::from_trigger(trigger),
            created_at: Utc::now(),
            retry_count: 0,
        }
    }

    /// Merge another task into this one.
    ///
    /// When the same file has multiple pending updates, we merge them
    /// to avoid duplicate work.
    ///
    /// # Merge Rules
    /// - Keep the highest priority
    /// - Update change_type appropriately:
    ///   - New + Deleted → cancel out (no net change)
    ///   - Modified + Modified → keep Modified
    ///   - New + Modified → keep New (still new, just different content)
    ///   - Deleted + anything → keep Deleted
    /// - Update timestamp to latest
    /// - Reset retry count to 0
    ///
    /// # Arguments
    /// * `other` - The other task to merge into this one
    pub fn merge(&mut self, other: UpdateTask) {
        // Keep highest priority
        if other.priority > self.priority {
            self.priority = other.priority;
            self.trigger = other.trigger;
        }

        // Update change_type based on merge rules
        self.change_type = Self::merge_change_types(&self.change_type, &other.change_type);

        // Update timestamp to latest
        if other.created_at > self.created_at {
            self.created_at = other.created_at;
        }

        // Reset retry count when merging (fresh attempt)
        self.retry_count = 0;
    }

    /// Merge two change types according to semantic rules.
    ///
    /// # Arguments
    /// * `first` - The first change type (existing in queue)
    /// * `second` - The second change type (new task)
    ///
    /// # Returns
    /// The merged change type
    fn merge_change_types(first: &ChangeType, second: &ChangeType) -> ChangeType {
        match (first, second) {
            // New + Deleted → None (cancel out)
            (ChangeType::New(_), ChangeType::Deleted(_)) => ChangeType::None,

            // Deleted + New → Modified (file was deleted then recreated)
            // Use the new hash as both old and new for simplicity
            (ChangeType::Deleted(old), ChangeType::New(new)) => ChangeType::Modified {
                old: *old,
                new: *new,
            },

            // New + Modified → New (still new, just different content)
            (ChangeType::New(_), ChangeType::Modified { new, .. }) => ChangeType::New(*new),

            // Modified + Modified → Modified (chain the changes)
            (ChangeType::Modified { old, .. }, ChangeType::Modified { new, .. }) => {
                ChangeType::Modified {
                    old: *old,
                    new: *new,
                }
            }

            // Modified + Deleted → Deleted
            (ChangeType::Modified { old, .. }, ChangeType::Deleted(_)) => ChangeType::Deleted(*old),

            // Modified + New → Modified (treat New as a significant modification)
            (ChangeType::Modified { old, .. }, ChangeType::New(new)) => ChangeType::Modified {
                old: *old,
                new: *new,
            },

            // Deleted + Modified/Deleted → keep Deleted
            (ChangeType::Deleted(hash), ChangeType::Modified { .. })
            | (ChangeType::Deleted(hash), ChangeType::Deleted(_)) => ChangeType::Deleted(*hash),

            // None + anything → take the new change
            (ChangeType::None, change) => change.clone(),

            // anything + None → keep existing change
            (change, ChangeType::None) => change.clone(),

            // New + New → keep the latest New
            (ChangeType::New(_), ChangeType::New(new)) => ChangeType::New(*new),
        }
    }

    /// Increment the retry count.
    ///
    /// Called when a task fails and needs to be retried.
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Check if this task has exceeded the maximum retry limit.
    ///
    /// # Arguments
    /// * `max_retries` - Maximum number of retries allowed
    ///
    /// # Returns
    /// True if the task has been retried too many times
    pub fn has_exceeded_retries(&self, max_retries: u32) -> bool {
        self.retry_count >= max_retries
    }
}

impl PartialEq for UpdateTask {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for UpdateTask {}

impl std::hash::Hash for UpdateTask {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incremental::hash::FileHasher;

    #[test]
    fn test_priority_from_trigger() {
        assert_eq!(Priority::from_trigger(Trigger::User), Priority::High);
        assert_eq!(Priority::from_trigger(Trigger::Save), Priority::Medium);
        assert_eq!(Priority::from_trigger(Trigger::Auto), Priority::Low);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
        assert!(Priority::High > Priority::Low);
    }

    #[test]
    fn test_task_creation() {
        let path = PathBuf::from("/test/file.rs");
        let hash = FileHasher::hash_bytes(b"test content");
        let change = ChangeType::New(hash);

        let task = UpdateTask::new(path.clone(), change.clone(), Trigger::Save);

        assert_eq!(task.path, path);
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.trigger, Trigger::Save);
        assert_eq!(task.retry_count, 0);
    }

    #[test]
    fn test_task_merge_priority() {
        let path = PathBuf::from("/test/file.rs");
        let hash = FileHasher::hash_bytes(b"test");

        let mut task1 = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::Auto);
        let task2 = UpdateTask::new(path.clone(), ChangeType::New(hash), Trigger::User);

        assert_eq!(task1.priority, Priority::Low);

        task1.merge(task2);

        // Should take the higher priority
        assert_eq!(task1.priority, Priority::High);
        assert_eq!(task1.trigger, Trigger::User);
    }

    #[test]
    fn test_merge_change_types_new_deleted() {
        let hash1 = FileHasher::hash_bytes(b"content1");
        let hash2 = FileHasher::hash_bytes(b"content2");

        // New + Deleted → None (cancel out)
        let result =
            UpdateTask::merge_change_types(&ChangeType::New(hash1), &ChangeType::Deleted(hash2));
        assert_eq!(result, ChangeType::None);
    }

    #[test]
    fn test_merge_change_types_modified_modified() {
        let hash1 = FileHasher::hash_bytes(b"v1");
        let hash2 = FileHasher::hash_bytes(b"v2");
        let hash3 = FileHasher::hash_bytes(b"v3");

        // Modified + Modified → chain the changes
        let result = UpdateTask::merge_change_types(
            &ChangeType::Modified {
                old: hash1,
                new: hash2,
            },
            &ChangeType::Modified {
                old: hash2,
                new: hash3,
            },
        );

        match result {
            ChangeType::Modified { old, new } => {
                assert_eq!(old, hash1);
                assert_eq!(new, hash3);
            }
            _ => panic!("Expected Modified change type"),
        }
    }

    #[test]
    fn test_merge_change_types_new_modified() {
        let hash1 = FileHasher::hash_bytes(b"v1");
        let hash2 = FileHasher::hash_bytes(b"v2");

        // New + Modified → New (still new, just different content)
        let result = UpdateTask::merge_change_types(
            &ChangeType::New(hash1),
            &ChangeType::Modified {
                old: hash1,
                new: hash2,
            },
        );

        assert_eq!(result, ChangeType::New(hash2));
    }

    #[test]
    fn test_merge_change_types_deleted_new() {
        let hash1 = FileHasher::hash_bytes(b"v1");
        let hash2 = FileHasher::hash_bytes(b"v2");

        // Deleted + New → Modified (file recreated)
        let result =
            UpdateTask::merge_change_types(&ChangeType::Deleted(hash1), &ChangeType::New(hash2));

        match result {
            ChangeType::Modified { old, new } => {
                assert_eq!(old, hash1);
                assert_eq!(new, hash2);
            }
            _ => panic!("Expected Modified change type"),
        }
    }

    #[test]
    fn test_retry_count() {
        let path = PathBuf::from("/test/file.rs");
        let hash = FileHasher::hash_bytes(b"test");
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

    #[test]
    fn test_task_equality() {
        let path1 = PathBuf::from("/test/file1.rs");
        let path2 = PathBuf::from("/test/file2.rs");
        let hash = FileHasher::hash_bytes(b"test");

        let task1 = UpdateTask::new(path1.clone(), ChangeType::New(hash), Trigger::Auto);
        let task2 = UpdateTask::new(path1.clone(), ChangeType::New(hash), Trigger::User);
        let task3 = UpdateTask::new(path2, ChangeType::New(hash), Trigger::Auto);

        // Tasks are equal if they have the same path
        assert_eq!(task1, task2);
        assert_ne!(task1, task3);
    }
}
