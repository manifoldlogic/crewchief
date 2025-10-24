//! Context assembly strategies for different code patterns.
//!
//! This module provides specialized assembly strategies that enhance
//! the basic context assembler with pattern-specific intelligence.

pub mod react;

pub use react::ReactAssemblyStrategy;
