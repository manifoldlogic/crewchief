//! Incremental indexing system.
//!
//! This module provides the infrastructure for incremental indexing, including:
//! - File content hashing with blake3
//! - In-memory hash cache for fast change detection
//! - Change detection logic

pub mod cache;
pub mod detector;
pub mod hash;

pub use cache::HashCache;
pub use detector::{ChangeDetector, ChangeType};
pub use hash::{ContentHash, FileHasher};
