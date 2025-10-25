//! File system event types for incremental indexing.
//!
//! This module defines the event types that are emitted by the file watcher
//! and consumed by the change detector.

use std::path::PathBuf;
use std::time::SystemTime;

/// Represents a file system event that may trigger incremental indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    /// A file was modified or created.
    Modified(PathBuf),

    /// A file was deleted.
    Deleted(PathBuf),

    /// A file was renamed or moved.
    /// First path is the old location, second path is the new location.
    Renamed(PathBuf, PathBuf),
}

impl FileEvent {
    /// Returns the primary path associated with this event.
    /// For Renamed events, this returns the new path.
    pub fn path(&self) -> &PathBuf {
        match self {
            FileEvent::Modified(path) => path,
            FileEvent::Deleted(path) => path,
            FileEvent::Renamed(_, new_path) => new_path,
        }
    }

    /// Returns all paths associated with this event.
    /// For Renamed events, this returns both old and new paths.
    pub fn paths(&self) -> Vec<&PathBuf> {
        match self {
            FileEvent::Modified(path) => vec![path],
            FileEvent::Deleted(path) => vec![path],
            FileEvent::Renamed(old, new) => vec![old, new],
        }
    }
}

/// Unique identifier for a worktree.
pub type WorktreeId = String;

/// Event type categorization for indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    /// File was modified or created.
    Modified,
    /// File was deleted.
    Deleted,
    /// File was renamed or moved.
    Renamed,
}

/// A file system event tagged with worktree context for multi-worktree indexing.
///
/// This extends FileEvent with worktree identification, enabling simultaneous
/// monitoring of multiple worktree directories with proper event isolation.
#[derive(Debug, Clone)]
pub struct IndexingEvent {
    /// The worktree this event originated from.
    pub worktree_id: WorktreeId,
    /// The file path affected by this event.
    pub path: PathBuf,
    /// The type of event that occurred.
    pub event_type: EventType,
    /// When the event was detected.
    pub timestamp: SystemTime,
    /// For rename events, the old path (before the rename).
    pub old_path: Option<PathBuf>,
}

impl IndexingEvent {
    /// Create a new IndexingEvent from a FileEvent.
    pub fn from_file_event(
        worktree_id: WorktreeId,
        file_event: FileEvent,
        timestamp: SystemTime,
    ) -> Self {
        match file_event {
            FileEvent::Modified(path) => IndexingEvent {
                worktree_id,
                path,
                event_type: EventType::Modified,
                timestamp,
                old_path: None,
            },
            FileEvent::Deleted(path) => IndexingEvent {
                worktree_id,
                path,
                event_type: EventType::Deleted,
                timestamp,
                old_path: None,
            },
            FileEvent::Renamed(old_path, new_path) => IndexingEvent {
                worktree_id,
                path: new_path,
                event_type: EventType::Renamed,
                timestamp,
                old_path: Some(old_path),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_modified_event() {
        let path = PathBuf::from("/test/file.txt");
        let event = FileEvent::Modified(path.clone());
        assert_eq!(event.path(), &path);
        assert_eq!(event.paths(), vec![&path]);
    }

    #[test]
    fn test_deleted_event() {
        let path = PathBuf::from("/test/file.txt");
        let event = FileEvent::Deleted(path.clone());
        assert_eq!(event.path(), &path);
        assert_eq!(event.paths(), vec![&path]);
    }

    #[test]
    fn test_renamed_event() {
        let old_path = PathBuf::from("/test/old.txt");
        let new_path = PathBuf::from("/test/new.txt");
        let event = FileEvent::Renamed(old_path.clone(), new_path.clone());
        assert_eq!(event.path(), &new_path);
        assert_eq!(event.paths(), vec![&old_path, &new_path]);
    }

    #[test]
    fn test_indexing_event_from_modified() {
        let path = PathBuf::from("/test/file.txt");
        let file_event = FileEvent::Modified(path.clone());
        let timestamp = SystemTime::now();
        let worktree_id = "worktree-1".to_string();

        let indexing_event =
            IndexingEvent::from_file_event(worktree_id.clone(), file_event, timestamp);

        assert_eq!(indexing_event.worktree_id, worktree_id);
        assert_eq!(indexing_event.path, path);
        assert_eq!(indexing_event.event_type, EventType::Modified);
        assert_eq!(indexing_event.timestamp, timestamp);
        assert_eq!(indexing_event.old_path, None);
    }

    #[test]
    fn test_indexing_event_from_deleted() {
        let path = PathBuf::from("/test/file.txt");
        let file_event = FileEvent::Deleted(path.clone());
        let timestamp = SystemTime::now();
        let worktree_id = "worktree-2".to_string();

        let indexing_event =
            IndexingEvent::from_file_event(worktree_id.clone(), file_event, timestamp);

        assert_eq!(indexing_event.worktree_id, worktree_id);
        assert_eq!(indexing_event.path, path);
        assert_eq!(indexing_event.event_type, EventType::Deleted);
        assert_eq!(indexing_event.timestamp, timestamp);
        assert_eq!(indexing_event.old_path, None);
    }

    #[test]
    fn test_indexing_event_from_renamed() {
        let old_path = PathBuf::from("/test/old.txt");
        let new_path = PathBuf::from("/test/new.txt");
        let file_event = FileEvent::Renamed(old_path.clone(), new_path.clone());
        let timestamp = SystemTime::now();
        let worktree_id = "worktree-3".to_string();

        let indexing_event =
            IndexingEvent::from_file_event(worktree_id.clone(), file_event, timestamp);

        assert_eq!(indexing_event.worktree_id, worktree_id);
        assert_eq!(indexing_event.path, new_path);
        assert_eq!(indexing_event.event_type, EventType::Renamed);
        assert_eq!(indexing_event.timestamp, timestamp);
        assert_eq!(indexing_event.old_path, Some(old_path));
    }
}
