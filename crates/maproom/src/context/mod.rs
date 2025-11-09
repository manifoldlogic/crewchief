//! Context assembly engine for intelligent code context bundling.
//!
//! This module provides the foundational pipeline for assembling context bundles
//! that include code chunks, their relationships, tests, and supporting files
//! within a specified token budget.

pub mod assembler;
pub mod budget;
pub mod cache;
pub mod cache_stats;
pub mod detectors;
pub mod file_loader;
pub mod formatter;
pub mod graph;
pub mod heuristics;
pub mod importance;
pub mod language_detector;
pub mod priority_queue;
pub mod relationships;
pub mod strategies;
pub mod strategy;
pub mod token_counter;
pub mod truncation;
pub mod types;

// Re-export core types for convenience
pub use assembler::{BasicContextAssembler, ContextAssembler, ParallelContextAssembler};
pub use budget::{BudgetAllocation, SharedBudgetManager, TokenBudgetManager, UsageStats};
pub use cache::{hash_options, CacheConfig, CacheKey, CacheStats, ContextCache, DbCacheStats};
pub use cache_stats::{CacheMetrics, CacheStatistics, CacheStatsMonitor, MemoryStats};
pub use file_loader::FileLoader;
pub use formatter::ContentFormatter;
pub use graph::{
    find_related_chunks, find_related_chunks_directional, load_relationships_parallel, EdgeType,
    RelatedChunk,
};
pub use heuristics::{FileType, HeuristicScorer, HeuristicsConfig};
pub use importance::{ChunkMetadata, ImportanceScorer, Relationship, ScoringConfig};
pub use language_detector::{Language, LanguageDetector};
pub use priority_queue::{Category, PriorityItem, PriorityQueue};
pub use relationships::{
    find_all_relationships, find_callees, find_callers, find_exports, find_imports, find_routes,
    find_test_files,
};
pub use strategies::{
    DefaultAssemblyStrategy, PythonAssemblyStrategy, PythonConfig, ReactAssemblyStrategy,
    RustAssemblyStrategy, RustConfig,
};
pub use strategy::AssemblyStrategy;
pub use token_counter::TokenCounter;
pub use truncation::{CodeTruncator, TruncationResult, TruncationStrategy};
pub use types::{ContextBundle, ContextItem, ExpandOptions, LineRange};
