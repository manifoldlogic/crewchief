//! Incremental indexing system.
//!
//! This module provides the infrastructure for incremental indexing, including:
//! - File content hashing with blake3
//! - In-memory hash cache for fast change detection
//! - Change detection logic
//! - File system watching with debouncing
//! - Ignore pattern filtering
//! - Multi-worktree support with event isolation
//! - Priority-based update queue with retry logic
//! - Incremental file processing with atomic updates
//! - Edge relationship maintenance
//!
//! # Path Handling Strategy
//!
//! The watch command deals with two path representations:
//! - **Absolute paths**: From file watcher (e.g., `/workspace/src/main.rs`)
//! - **Relative paths**: Stored in database (e.g., `src/main.rs`)
//!
//! **Critical Rule**: Always normalize to relative paths for database queries using
//! [`normalize_to_relpath()`](path_utils::normalize_to_relpath). Use absolute paths
//! only for filesystem operations (reading files, checking metadata).
//!
//! **Bug Fixed**: Previously, the watch command passed absolute paths to database
//! lookup functions that expected relative paths. This caused existing files to be
//! misclassified as NEW, resulting in "File not found" errors during re-indexing.
//! See `.agents/projects/WATCHFIX_watch-change-detection-fix/planning/analysis.md`
//! for detailed root cause analysis.
//!
//! # Security Considerations
//!
//! - **Path traversal protection**: [`normalize_to_relpath()`](path_utils::normalize_to_relpath)
//!   rejects paths containing `..` components
//! - **File size limits**: Files larger than 10MB are skipped to prevent DoS
//! - **Symlink awareness**: Symlinks are logged but allowed (user responsibility)

pub mod cache;
pub mod detector;
pub mod edge_updater;
pub mod events;
pub mod hash;
pub mod ignore;
pub mod multi_watcher;
pub mod path_utils;
pub mod processor;
pub mod queue;
pub mod task;
pub mod tree_sha_update;
pub mod watcher;
pub mod worktree_watcher;

pub use cache::HashCache;
pub use detector::{ChangeDetector, ChangeType};
pub use edge_updater::EdgeUpdater;
pub use events::{EventType, FileEvent, IndexingEvent, WorktreeId};
pub use hash::{ContentHash, FileHasher};
pub use ignore::IgnorePatternMatcher;
pub use multi_watcher::MultiWatcher;
pub use path_utils::normalize_to_relpath;
pub use processor::IncrementalProcessor;
pub use queue::{QueueStats, UpdateQueue};
pub use task::{Priority, Trigger, UpdateTask};
pub use tree_sha_update::incremental_update;
pub use watcher::{FileWatcher, WatcherConfig};
pub use worktree_watcher::{WatcherStatus, WorktreeWatcher};
