//! Maproom library - Code indexing and semantic search.
//!
//! This library provides code indexing, database access, and embedding services
//! for the Maproom semantic code search system.

pub mod ab_testing;
pub mod cache;
pub mod cli;
pub mod config;
pub mod content_hash;
pub mod context;
pub mod db;
pub mod embedding;
pub mod evaluation;
pub mod incremental;
pub mod indexer;
pub mod memory;
pub mod metrics;
pub mod migrate;
pub mod profiling;
pub mod progress;
pub mod search;
