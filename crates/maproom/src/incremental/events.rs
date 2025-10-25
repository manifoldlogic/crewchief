//! File system event types for incremental indexing.
//!
//! This module defines the event types that are emitted by the file watcher
//! and consumed by the change detector.

use std::path::PathBuf;

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
}
