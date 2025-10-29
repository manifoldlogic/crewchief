//! Database access layer for Maproom.
//!
//! This module provides database connectivity, connection pooling, and query utilities.

pub mod columns;
pub mod materialized_views;
pub mod pool;
pub mod queries;

// Re-export column selection types for convenience
pub use columns::{select_columns_for_dimension, ColumnSet};

// Re-export pool types for convenience
pub use pool::{create_pool, pool_stats, PgPool, PoolStats};

// Re-export query functions for convenience
pub use queries::*;
