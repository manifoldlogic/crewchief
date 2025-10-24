//! Context assembly engine for intelligent code context bundling.
//!
//! This module provides the foundational pipeline for assembling context bundles
//! that include code chunks, their relationships, tests, and supporting files
//! within a specified token budget.

pub mod assembler;
pub mod budget;
pub mod file_loader;
pub mod graph;
pub mod priority_queue;
pub mod relationships;
pub mod token_counter;
pub mod truncation;
pub mod types;

// Re-export core types for convenience
pub use assembler::{BasicContextAssembler, ContextAssembler};
pub use budget::{BudgetAllocation, TokenBudgetManager, UsageStats};
pub use file_loader::FileLoader;
pub use graph::{EdgeType, RelatedChunk, find_related_chunks, find_related_chunks_directional};
pub use priority_queue::{Category, PriorityItem, PriorityQueue};
pub use relationships::{
    find_all_relationships, find_callees, find_callers, find_exports, find_imports,
    find_routes, find_test_files,
};
pub use token_counter::TokenCounter;
pub use truncation::{CodeTruncator, TruncationResult, TruncationStrategy};
pub use types::{ContextBundle, ContextItem, ExpandOptions, LineRange};
