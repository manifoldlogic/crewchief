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
pub use watcher::{FileWatcher, WatcherConfig};
pub use worktree_watcher::{WatcherStatus, WorktreeWatcher};
