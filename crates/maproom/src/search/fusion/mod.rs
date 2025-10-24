//! Score fusion for hybrid search results.
//!
//! This module implements score fusion strategies that combine results from
//! multiple search strategies (FTS, vector, graph, signals) into a single
//! ranked result set.
//!
//! # Fusion Strategies
//!
//! - **BasicWeightedFusion**: Simple weighted average (Phase 2 baseline)
//! - **RRFFusion**: Reciprocal Rank Fusion (Phase 3 sophisticated approach)
//!
//! # Score Normalization
//!
//! All scores are normalized to the 0.0-1.0 range before fusion to ensure
//! fair combination across different search types with different score ranges.

mod basic;
mod rrf;

pub use basic::{BasicWeightedFusion, FusionWeights};
pub use rrf::RRFFusion;

use crate::search::executor_types::{RankedResults, SearchSource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for score fusion strategies.
///
/// Implementations combine results from multiple search strategies into
/// a single ranked result set with fused scores.
pub trait ScoreFusion: Send + Sync {
    /// Fuse multiple result sets into a single ranked list.
    ///
    /// # Parameters
    /// - `results`: Vector of RankedResults from different search strategies
    /// - `weights`: Weights for each search type
    /// - `limit`: Maximum number of results to return
    ///
    /// # Returns
    /// Vector of FusedResult with combined scores, sorted by score descending
    fn fuse(
        &self,
        results: Vec<RankedResults>,
        weights: &FusionWeights,
        limit: usize,
    ) -> Vec<FusedResult>;
}

/// A single search result with fused score from multiple sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedResult {
    /// Chunk ID from maproom.chunks table
    pub chunk_id: i64,

    /// Combined score after fusion (0.0-1.0)
    pub score: f32,

    /// Individual scores from each search source that found this chunk
    pub source_scores: HashMap<SearchSource, f32>,
}

impl FusedResult {
    /// Create a new FusedResult.
    pub fn new(chunk_id: i64, score: f32, source_scores: HashMap<SearchSource, f32>) -> Self {
        Self {
            chunk_id,
            score,
            source_scores,
        }
    }
}
