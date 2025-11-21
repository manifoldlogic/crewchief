//! Database access layer for Maproom.
//!
//! This module provides database connectivity, connection pooling, and query utilities.

pub mod cleanup;
pub mod columns;
pub mod connection;
pub mod index_state;
pub mod materialized_views;
pub mod pool;
pub mod queries;

// Re-export cleanup types for convenience
pub use cleanup::{
    CleanupError, CleanupReport, StaleWorktree, StaleWorktreeDetector, WorktreeCleaner,
};

// Re-export column selection types for convenience
pub use columns::{select_columns_for_dimension, ColumnSet};

// Re-export index state functions for convenience
pub use index_state::{get_last_indexed_tree, update_index_state, UpdateStats};

// Re-export pool types for convenience
pub use pool::{create_pool, pool_stats, PgPool, PoolStats};

// Re-export query functions for convenience
pub use queries::*;
