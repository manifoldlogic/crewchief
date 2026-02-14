//! Promoted types from sqlite submodules.
//!
//! These types are used in trait method signatures and need to live at the
//! backend-agnostic `db` level. They were originally defined in `db::sqlite::*`
//! submodules and are re-exported there for backward compatibility.

use std::collections::HashMap;

// ============================================================================
// Graph types (from db/sqlite/graph.rs)
// ============================================================================

/// Result from graph traversal
#[derive(Debug, Clone)]
pub struct GraphResult {
    /// Target chunk ID found in traversal
    pub chunk_id: i64,
    /// Depth from source (1 = direct relationship)
    pub depth: usize,
    /// Path from source to this chunk (list of chunk IDs)
    pub path: Vec<i64>,
    /// Type of relationship (calls, imports, extends)
    pub edge_type: String,
}

/// Direction for import relationship queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportDirection {
    /// Find chunks that import the target (who imports this?)
    Incoming,
    /// Find chunks that the source imports (what does this import?)
    Outgoing,
}

// ============================================================================
// Embedding types (from db/sqlite/embeddings.rs)
// ============================================================================

/// Record for batch embedding operations
#[derive(Clone)]
pub struct EmbeddingRecord {
    pub blob_sha: String,
    pub embedding: Vec<f32>,
    pub model_version: String,
}

// ============================================================================
// Encoding types (from db/sqlite/mod.rs)
// ============================================================================

/// Row data from the encoding_runs table.
#[derive(Debug, Clone)]
pub struct EncodingRunRow {
    pub id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub total_chunks: i64,
    pub chunks_completed: i64,
    pub chunks_per_second: Option<f64>,
    pub last_batch_at: Option<String>,
    pub provider: Option<String>,
    pub dimension: Option<i32>,
}

// ============================================================================
// Hybrid search types (from db/sqlite/hybrid.rs)
// ============================================================================

/// Weights for combining FTS and vector search contributions
#[derive(Debug, Clone)]
pub struct HybridWeights {
    /// Weight for FTS (keyword) contribution (default 0.3)
    pub fts_weight: f64,
    /// Weight for vector (semantic) contribution (default 0.7)
    pub vector_weight: f64,
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            fts_weight: 0.3,
            vector_weight: 0.7,
        }
    }
}

impl HybridWeights {
    /// Create weights with custom values
    pub fn new(fts_weight: f64, vector_weight: f64) -> Self {
        Self {
            fts_weight,
            vector_weight,
        }
    }

    /// Equal weights for FTS and vector (0.5 each)
    pub fn equal() -> Self {
        Self {
            fts_weight: 0.5,
            vector_weight: 0.5,
        }
    }

    /// FTS-heavy weights (0.7 FTS, 0.3 vector)
    pub fn fts_heavy() -> Self {
        Self {
            fts_weight: 0.7,
            vector_weight: 0.3,
        }
    }

    /// Vector-heavy weights (0.3 FTS, 0.7 vector) - default
    pub fn vector_heavy() -> Self {
        Self::default()
    }
}

/// Result from hybrid search combining FTS and vector scores
#[derive(Debug, Clone)]
pub struct HybridResult {
    /// Chunk ID in the chunks table
    pub chunk_id: i64,
    /// Combined RRF score (higher = better)
    pub score: f64,
    /// Position in FTS results (None if not found by FTS)
    pub fts_rank: Option<usize>,
    /// Position in vector results (None if not found by vector search)
    pub vector_rank: Option<usize>,
    /// Source indicator: "fts", "vector", or "both"
    pub source: String,
}

/// Semantic ranking configuration with domain-specific multipliers
///
/// Applies adjustments based on:
/// - Kind (function, class, variable, etc.)
/// - Exact match boost when symbol name matches query
/// - Recency weight for recently modified chunks
#[derive(Debug, Clone)]
pub struct SemanticRanking {
    /// Multipliers for different chunk kinds
    pub kind_multipliers: HashMap<String, f64>,
    /// Boost applied when symbol name matches query
    pub exact_match_boost: f64,
    /// Weight for recency score contribution (0-1)
    pub recency_weight: f64,
}

impl Default for SemanticRanking {
    fn default() -> Self {
        let mut kind_multipliers = HashMap::new();
        kind_multipliers.insert("function".to_string(), 1.2);
        kind_multipliers.insert("method".to_string(), 1.2);
        kind_multipliers.insert("class".to_string(), 1.1);
        kind_multipliers.insert("struct".to_string(), 1.1);
        kind_multipliers.insert("interface".to_string(), 1.1);
        kind_multipliers.insert("trait".to_string(), 1.1);
        kind_multipliers.insert("enum".to_string(), 1.0);
        kind_multipliers.insert("module".to_string(), 1.0);
        kind_multipliers.insert("constant".to_string(), 0.9);
        kind_multipliers.insert("variable".to_string(), 0.8);
        kind_multipliers.insert("import".to_string(), 0.7);

        Self {
            kind_multipliers,
            exact_match_boost: 1.5,
            recency_weight: 0.1, // Small boost for recent changes
        }
    }
}

impl SemanticRanking {
    /// Create semantic ranking with custom multipliers
    pub fn new(
        kind_multipliers: HashMap<String, f64>,
        exact_match_boost: f64,
        recency_weight: f64,
    ) -> Self {
        Self {
            kind_multipliers,
            exact_match_boost,
            recency_weight,
        }
    }

    /// Create semantic ranking that doesn't apply any adjustments
    pub fn identity() -> Self {
        Self {
            kind_multipliers: HashMap::new(),
            exact_match_boost: 1.0,
            recency_weight: 0.0,
        }
    }
}

/// Chunk metadata needed for semantic ranking
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    /// Chunk kind (function, class, variable, etc.)
    pub kind: String,
    /// Symbol name if available
    pub symbol_name: Option<String>,
    /// Recency score 0-1 (1 = most recent)
    pub recency_score: f64,
}

/// Extended search hit with chunk metadata for ranking
#[derive(Debug, Clone)]
pub struct RankedSearchHit {
    /// Chunk ID in the chunks table
    pub chunk_id: i64,
    /// Combined score after semantic ranking adjustments
    pub score: f64,
    /// Position in FTS results (None if not found by FTS)
    pub fts_rank: Option<usize>,
    /// Position in vector results (None if not found by vector search)
    pub vector_rank: Option<usize>,
    /// Chunk kind (function, class, variable, etc.)
    pub kind: String,
    /// Symbol name if available
    pub symbol_name: Option<String>,
    /// Recency score 0-1 (1 = most recent)
    pub recency_score: f64,
    /// Source indicator: "fts", "vector", or "both"
    pub source: String,
}
