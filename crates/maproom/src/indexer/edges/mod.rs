//! Edge extraction module for code relationships.
//!
//! This module provides language-specific extraction of code edges (relationships
//! between symbols) such as function calls, imports, and other dependencies.
//!
//! # Architecture
//!
//! - `extract_edges()` - Public API dispatcher by language
//! - `common` - Shared utilities for all extractors
//! - `typescript` - TypeScript/JavaScript call extraction
//!
//! # Usage
//!
//! ```no_run
//! use maproom::indexer::edges::{extract_edges, ChunkWithId};
//!
//! let source = "function foo() { bar(); }";
//! let chunks = vec![
//!     ChunkWithId {
//!         id: 1,
//!         symbol_name: Some("foo".to_string()),
//!         kind: "function".to_string(),
//!         start_line: 1,
//!         end_line: 1,
//!         file_id: 100,
//!     }
//! ];
//!
//! let edges = extract_edges(source, "typescript", &chunks)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use anyhow::Result;

// Re-export Edge and EdgeType from edge_updater (shared types)
pub use crate::incremental::edge_updater::{Edge, EdgeType};

pub mod common;
pub mod rust;
pub mod typescript;

/// Chunk with database ID (after insertion).
///
/// This struct represents a chunk that has been inserted into the database
/// and has a unique ID. It includes the file_id for Phase 2 cross-file resolution.
#[derive(Debug, Clone)]
pub struct ChunkWithId {
    /// Database chunk ID
    pub id: i64,
    /// Symbol name (e.g., function name, class name)
    pub symbol_name: Option<String>,
    /// Chunk kind (e.g., "function", "class", "method")
    pub kind: String,
    /// Starting line number (1-indexed)
    pub start_line: i32,
    /// Ending line number (1-indexed)
    pub end_line: i32,
    /// Database file ID (for Phase 2 cross-file resolution)
    pub file_id: i64,
}

/// Extract edges from source code.
///
/// Dispatches to language-specific extractors based on the language parameter.
/// Returns an empty vector for unsupported languages (graceful degradation).
///
/// # Arguments
///
/// * `source` - Source code text
/// * `language` - Language identifier ("typescript", "tsx", "javascript", "jsx", etc.)
/// * `chunks` - Chunks with database IDs from the same file
///
/// # Returns
///
/// * `Ok(Vec<Edge>)` - Extracted edges (may be empty for unsupported languages)
/// * `Err(_)` - Critical failure (parsing error, etc.)
///
/// # Reused Types
///
/// This function reuses `Edge` and `EdgeType` from the `crate::incremental::edge_updater`
/// module. These types are made public in edge_updater.rs for shared use.
///
/// # Example
///
/// ```no_run
/// use maproom::indexer::edges::{extract_edges, ChunkWithId};
///
/// let source = "function foo() { bar(); }";
/// let chunks = vec![
///     ChunkWithId {
///         id: 1,
///         symbol_name: Some("foo".to_string()),
///         kind: "function".to_string(),
///         start_line: 1,
///         end_line: 1,
///         file_id: 100,
///     }
/// ];
///
/// let edges = extract_edges(source, "typescript", &chunks)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract_edges(source: &str, language: &str, chunks: &[ChunkWithId]) -> Result<Vec<Edge>> {
    match language {
        "ts" | "tsx" | "js" | "jsx" => typescript::extract_calls(source, chunks),
        "rs" => rust::extract_calls(source, chunks),
        // Python will be added in Phase 2/3
        _ => {
            // No edge extraction for unsupported languages
            Ok(Vec::new())
        }
    }
}
