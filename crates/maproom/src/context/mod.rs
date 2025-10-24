//! Context assembly engine for intelligent code context bundling.
//!
//! This module provides the foundational pipeline for assembling context bundles
//! that include code chunks, their relationships, tests, and supporting files
//! within a specified token budget.

pub mod assembler;
pub mod file_loader;
pub mod token_counter;
pub mod types;

// Re-export core types for convenience
pub use assembler::{BasicContextAssembler, ContextAssembler};
pub use file_loader::FileLoader;
pub use token_counter::TokenCounter;
pub use types::{ContextBundle, ContextItem, ExpandOptions, LineRange};
