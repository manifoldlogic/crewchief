//! Incremental indexing system.
//!
//! This module provides the infrastructure for incremental indexing, including:
//! - File content hashing with blake3
//! - In-memory hash cache for fast change detection
//! - Change detection logic
//! - File system watching with debouncing
//! - Ignore pattern filtering
//! - Multi-worktree support with event isolation

pub mod cache;
pub mod detector;
pub mod events;
pub mod hash;
pub mod ignore;
pub mod multi_watcher;
pub mod watcher;
pub mod worktree_watcher;

pub use cache::HashCache;
pub use detector::{ChangeDetector, ChangeType};
pub use events::{EventType, FileEvent, IndexingEvent, WorktreeId};
pub use hash::{ContentHash, FileHasher};
pub use ignore::IgnorePatternMatcher;
pub use multi_watcher::MultiWatcher;
pub use watcher::{FileWatcher, WatcherConfig};
pub use worktree_watcher::{WatcherStatus, WorktreeWatcher};
