//! Context assembly strategies for different code patterns.
//!
//! This module provides specialized assembly strategies that enhance
//! the basic context assembler with pattern-specific intelligence.

pub mod default;
pub mod python;
pub mod react;
pub mod rust;

pub use default::DefaultAssemblyStrategy;
pub use python::{PythonAssemblyStrategy, PythonConfig};
pub use react::ReactAssemblyStrategy;
pub use rust::{RustAssemblyStrategy, RustConfig};
