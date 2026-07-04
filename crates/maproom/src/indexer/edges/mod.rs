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
//! let (edges, _unresolved) = extract_edges(source, "typescript", &chunks)?;
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

/// A call whose callee was NOT found among the same file's chunks (spec B1).
///
/// Instead of silently dropping it at the extractor's `trace!` miss site, the
/// extractor returns it so the worktree post-pass can resolve it cross-file under
/// the precision-first ambiguity policy (spec B3). The caller (`src_chunk_id`) is
/// always in the file being extracted, so its relpath/language come from the loop
/// context — this struct only needs the callee name.
#[derive(Debug, Clone)]
pub struct UnresolvedRef {
    /// Chunk id of the calling function/method (in the current file).
    pub src_chunk_id: i64,
    /// The unqualified callee name that did not resolve locally.
    pub callee_name: String,
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
/// let (edges, _unresolved) = extract_edges(source, "typescript", &chunks)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract_edges(
    source: &str,
    language: &str,
    chunks: &[ChunkWithId],
) -> Result<(Vec<Edge>, Vec<UnresolvedRef>)> {
    match language {
        "ts" | "tsx" | "js" | "jsx" => typescript::extract_calls(source, language, chunks),
        "rs" => rust::extract_calls(source, chunks),
        _ => {
            // No edge extraction for unsupported languages
            Ok((Vec::new(), Vec::new()))
        }
    }
}

/// Spec A1/A2: THE single language gate for call-edge extraction. Every
/// production site (scan, upsert/watch, incremental updater) MUST use this
/// predicate — three hand-rolled `matches!` gates previously drifted and
/// left the `rs` dispatcher arm dead in production.
pub fn supports_call_extraction(language: &str) -> bool {
    matches!(language, "ts" | "tsx" | "js" | "jsx" | "rs")
}

/// Spec A6/B3: chunk kinds that can be a call TARGET. `use`/import/module/
/// struct-name chunks share symbol names with real functions and are the
/// primary false-positive source when they enter a symbol table.
pub fn is_callable_kind(kind: &str) -> bool {
    matches!(kind, "func" | "function" | "method")
}
