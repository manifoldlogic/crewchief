//! Detectors for React-specific code patterns.
//!
//! This module provides specialized detectors for identifying React components,
//! hooks, and JSX relationships in TypeScript/JavaScript codebases.

pub mod component;
pub mod hooks;
pub mod jsx;

pub use component::ComponentDetector;
pub use hooks::HookDetector;
pub use jsx::JsxRelationshipDetector;
