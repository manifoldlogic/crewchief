//! Importance scoring for code chunks in context assembly.
//!
//! This module implements sophisticated importance scoring that combines:
//! - Base relevance scoring
//! - Relationship type weighting (test_of=1.5, calls=1.2, imports=1.1)
//! - Distance decay (exponential falloff with graph distance)
//! - Chunk importance from database (in-degree, recency, churn)
//! - Directory co-location bonus (same directory = 1.3x)
//!
//! The scoring system enables intelligent prioritization of related chunks
//! during context assembly, ensuring the most relevant code is included
//! within token budget constraints.

use std::collections::HashMap;
use std::path::Path;

use super::graph::EdgeType;
use super::heuristics::HeuristicScorer;

/// Configuration for importance scoring.
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    /// Base score to start with (default: 1.0)
    pub base_score: f64,
    /// Decay factor per hop distance (default: 0.7)
    pub decay_factor: f64,
    /// Weight multipliers for relationship types
    pub relationship_weights: HashMap<EdgeType, f64>,
    /// Bonus multiplier for same directory (default: 1.3)
    pub directory_bonus: f64,
    /// Whether to include recency in scoring
    pub include_recency: bool,
    /// Whether to include churn in scoring
    pub include_churn: bool,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        let mut weights = HashMap::new();
        weights.insert(EdgeType::TestOf, 1.5);
        weights.insert(EdgeType::Calls, 1.2);
        weights.insert(EdgeType::Imports, 1.1);
        weights.insert(EdgeType::Exports, 1.0);
        weights.insert(EdgeType::CalledBy, 1.2);
        weights.insert(EdgeType::RouteOf, 1.4);

        Self {
            base_score: 1.0,
            decay_factor: 0.7,
            relationship_weights: weights,
            directory_bonus: 1.3,
            include_recency: true,
            include_churn: true,
        }
    }
}

impl ScoringConfig {
    /// Create a new scoring configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base score.
    pub fn with_base_score(mut self, score: f64) -> Self {
        self.base_score = score;
        self
    }

    /// Set the decay factor.
    pub fn with_decay_factor(mut self, factor: f64) -> Self {
        self.decay_factor = factor;
        self
    }

    /// Set a relationship type weight.
    pub fn with_relationship_weight(mut self, edge_type: EdgeType, weight: f64) -> Self {
        self.relationship_weights.insert(edge_type, weight);
        self
    }

    /// Set the directory bonus.
    pub fn with_directory_bonus(mut self, bonus: f64) -> Self {
        self.directory_bonus = bonus;
        self
    }

    /// Enable or disable recency scoring.
    pub fn with_recency(mut self, enabled: bool) -> Self {
        self.include_recency = enabled;
        self
    }

    /// Enable or disable churn scoring.
    pub fn with_churn(mut self, enabled: bool) -> Self {
        self.include_churn = enabled;
        self
    }
}

/// Metadata about a chunk used for importance scoring.
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    /// Chunk ID
    pub id: i64,
    /// File path (for directory bonus calculation)
    pub relpath: String,
    /// Importance score from database (from chunk_importance materialized view)
    pub importance_score: Option<f64>,
    /// Recency score (0.0 = old, 1.0 = recent)
    pub recency_score: Option<f64>,
    /// Churn score (0.0 = stable, higher = frequently modified)
    pub churn_score: Option<f64>,
}

/// Relationship information for scoring.
#[derive(Debug, Clone)]
pub struct Relationship {
    /// Type of relationship edge
    pub edge_type: EdgeType,
    /// Distance in graph hops from target chunk
    pub distance: u32,
}

/// Importance scorer that calculates relevance scores for chunks.
pub struct ImportanceScorer {
    config: ScoringConfig,
    heuristic_scorer: Option<HeuristicScorer>,
}

impl ImportanceScorer {
    /// Create a new importance scorer with default configuration.
    pub fn new() -> Self {
        Self {
            config: ScoringConfig::default(),
            heuristic_scorer: Some(HeuristicScorer::new()),
        }
    }

    /// Create a new importance scorer with custom configuration.
    pub fn with_config(config: ScoringConfig) -> Self {
        Self {
            config,
            heuristic_scorer: Some(HeuristicScorer::new()),
        }
    }

    /// Create a new importance scorer without heuristics.
    pub fn without_heuristics(config: ScoringConfig) -> Self {
        Self {
            config,
            heuristic_scorer: None,
        }
    }

    /// Create a new importance scorer with custom heuristics.
    pub fn with_heuristics(config: ScoringConfig, heuristic_scorer: HeuristicScorer) -> Self {
        Self {
            config,
            heuristic_scorer: Some(heuristic_scorer),
        }
    }

    /// Calculate importance score for a chunk given its relationship to a target.
    ///
    /// # Arguments
    /// * `chunk` - Metadata about the chunk being scored
    /// * `relationship` - Relationship between chunk and target
    /// * `target` - Metadata about the target chunk
    ///
    /// # Returns
    /// Final importance score (typically 0.0-10.0 range, higher = more important)
    ///
    /// # Algorithm
    /// 1. Start with base score
    /// 2. Apply relationship type weight
    /// 3. Apply distance decay
    /// 4. Multiply by chunk importance (from DB)
    /// 5. Multiply by recency score (if enabled)
    /// 6. Multiply by inverse churn (if enabled)
    /// 7. Apply directory bonus (if same directory)
    /// 8. Apply heuristic weights (test/config file detection)
    /// 9. Clamp to reasonable range
    pub fn score(
        &self,
        chunk: &ChunkMetadata,
        relationship: &Relationship,
        target: &ChunkMetadata,
    ) -> f64 {
        let mut score = self.config.base_score;

        // Apply relationship type weight
        score = self.apply_relationship_weight(score, relationship.edge_type.clone());

        // Apply distance decay
        score = self.apply_distance_decay(score, relationship.distance);

        // Apply metadata scores
        score = self.apply_metadata_scores(score, chunk);

        // Apply directory bonus
        score = self.apply_directory_bonus(score, chunk, target);

        // Apply heuristic weights (test/config file detection)
        score = self.apply_heuristic_weights(score, chunk);

        // Clamp to reasonable range (prevent negative or infinite scores)
        score.max(0.0).min(100.0)
    }

    /// Apply relationship type weight multiplier.
    fn apply_relationship_weight(&self, score: f64, edge_type: EdgeType) -> f64 {
        let weight = self
            .config
            .relationship_weights
            .get(&edge_type)
            .copied()
            .unwrap_or(1.0);
        score * weight
    }

    /// Apply exponential distance decay.
    ///
    /// Formula: score *= decay_factor^distance
    /// Example: with decay_factor=0.7, distance=2: score *= 0.49
    fn apply_distance_decay(&self, score: f64, distance: u32) -> f64 {
        if distance == 0 {
            // Target chunk itself has no decay
            return score;
        }
        score * self.config.decay_factor.powf(distance as f64)
    }

    /// Apply metadata-based score multipliers from database.
    ///
    /// Multiplies by:
    /// - importance_score (graph centrality + recency + churn)
    /// - recency_score (if enabled and available)
    /// - inverse churn (if enabled and available)
    fn apply_metadata_scores(&self, score: f64, chunk: &ChunkMetadata) -> f64 {
        let mut result = score;

        // Apply precomputed importance score from materialized view
        if let Some(importance) = chunk.importance_score {
            result *= importance.max(0.1); // Avoid complete zeroing
        }

        // Apply additional recency boost if enabled
        if self.config.include_recency {
            if let Some(recency) = chunk.recency_score {
                result *= recency.max(0.1);
            }
        }

        // Apply inverse churn if enabled (stable code is more important)
        if self.config.include_churn {
            if let Some(churn) = chunk.churn_score {
                // Inverse relationship: high churn reduces score
                let churn_factor = 1.0 / (1.0 + churn);
                result *= churn_factor.max(0.1);
            }
        }

        result
    }

    /// Apply directory co-location bonus.
    ///
    /// If chunk and target are in the same directory, apply bonus multiplier.
    /// Optionally can implement graduated bonus for parent/child directories.
    fn apply_directory_bonus(
        &self,
        score: f64,
        chunk: &ChunkMetadata,
        target: &ChunkMetadata,
    ) -> f64 {
        if self.same_directory(&chunk.relpath, &target.relpath) {
            score * self.config.directory_bonus
        } else {
            score
        }
    }

    /// Check if two file paths are in the same directory.
    fn same_directory(&self, path1: &str, path2: &str) -> bool {
        let dir1 = Path::new(path1).parent();
        let dir2 = Path::new(path2).parent();

        match (dir1, dir2) {
            (Some(d1), Some(d2)) => d1 == d2,
            (None, None) => true, // Both in root
            _ => false,
        }
    }

    /// Apply heuristic-based weight multipliers (test/config file detection).
    ///
    /// Uses HeuristicScorer to detect file types and apply appropriate weights:
    /// - Test files: 1.5x multiplier (configurable)
    /// - Config files: 1.1x multiplier (configurable)
    /// - Regular files: no change
    fn apply_heuristic_weights(&self, score: f64, chunk: &ChunkMetadata) -> f64 {
        if let Some(ref heuristic_scorer) = self.heuristic_scorer {
            heuristic_scorer.apply_heuristic_weight(score, &chunk.relpath)
        } else {
            score
        }
    }

    /// Get the scoring configuration.
    pub fn config(&self) -> &ScoringConfig {
        &self.config
    }

    /// Get the heuristic scorer if enabled.
    pub fn heuristic_scorer(&self) -> Option<&HeuristicScorer> {
        self.heuristic_scorer.as_ref()
    }
}

impl Default for ImportanceScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chunk(id: i64, relpath: &str) -> ChunkMetadata {
        ChunkMetadata {
            id,
            relpath: relpath.to_string(),
            importance_score: Some(1.0),
            recency_score: Some(1.0),
            churn_score: Some(0.0),
        }
    }

    fn create_test_relationship(edge_type: EdgeType, distance: u32) -> Relationship {
        Relationship {
            edge_type,
            distance,
        }
    }

    #[test]
    fn test_scoring_config_defaults() {
        let config = ScoringConfig::default();
        assert_eq!(config.base_score, 1.0);
        assert_eq!(config.decay_factor, 0.7);
        assert_eq!(config.directory_bonus, 1.3);
        assert!(config.include_recency);
        assert!(config.include_churn);

        // Check relationship weights
        assert_eq!(config.relationship_weights.get(&EdgeType::TestOf), Some(&1.5));
        assert_eq!(config.relationship_weights.get(&EdgeType::Calls), Some(&1.2));
        assert_eq!(config.relationship_weights.get(&EdgeType::Imports), Some(&1.1));
    }

    #[test]
    fn test_scoring_config_builder() {
        let config = ScoringConfig::new()
            .with_base_score(2.0)
            .with_decay_factor(0.8)
            .with_directory_bonus(1.5)
            .with_recency(false)
            .with_churn(false);

        assert_eq!(config.base_score, 2.0);
        assert_eq!(config.decay_factor, 0.8);
        assert_eq!(config.directory_bonus, 1.5);
        assert!(!config.include_recency);
        assert!(!config.include_churn);
    }

    #[test]
    fn test_relationship_weight_application() {
        let scorer = ImportanceScorer::new();

        // Test relationship weight gets applied
        let chunk = create_test_chunk(1, "test.rs");
        let target = create_test_chunk(2, "test.rs");
        let rel_test = create_test_relationship(EdgeType::TestOf, 1);

        let score = scorer.score(&chunk, &rel_test, &target);

        // With TestOf (1.5x), distance 1 (0.7x), base 1.0, importance 1.0, recency 1.0, churn 0.0
        // Expected: 1.0 * 1.5 * 0.7 * 1.0 * 1.0 * 1.0 * 1.3 (same dir)
        // = 1.365
        assert!((score - 1.365).abs() < 0.01, "Expected ~1.365, got {}", score);
    }

    #[test]
    fn test_distance_decay() {
        let scorer = ImportanceScorer::new();

        let chunk = create_test_chunk(1, "src/a.rs");
        let target = create_test_chunk(2, "src/b.rs");

        // Distance 0 (target itself)
        let rel_0 = create_test_relationship(EdgeType::Calls, 0);
        let score_0 = scorer.score(&chunk, &rel_0, &target);
        // base=1.0, calls=1.2, dist=0 (no decay), importance=1.0, recency=1.0, churn=1.0, same_dir=1.3
        // = 1.0 * 1.2 * 1.0 * 1.0 * 1.0 * 1.0 * 1.3 = 1.56
        assert!((score_0 - 1.56).abs() < 0.01, "Expected ~1.56, got {}", score_0);

        // Distance 1
        let rel_1 = create_test_relationship(EdgeType::Calls, 1);
        let score_1 = scorer.score(&chunk, &rel_1, &target);
        // base=1.0, calls=1.2, dist=1 (0.7), importance=1.0, recency=1.0, churn=1.0, same_dir=1.3
        // = 1.0 * 1.2 * 0.7 * 1.0 * 1.0 * 1.0 * 1.3 = 1.092
        assert!((score_1 - 1.092).abs() < 0.01, "Expected ~1.092, got {}", score_1);

        // Distance 2
        let rel_2 = create_test_relationship(EdgeType::Calls, 2);
        let score_2 = scorer.score(&chunk, &rel_2, &target);
        // base=1.0, calls=1.2, dist=2 (0.49), importance=1.0, recency=1.0, churn=1.0, same_dir=1.3
        // = 1.0 * 1.2 * 0.49 * 1.0 * 1.0 * 1.0 * 1.3 = 0.7644
        assert!((score_2 - 0.7644).abs() < 0.01, "Expected ~0.7644, got {}", score_2);

        // Verify decay: score_1 < score_0 and score_2 < score_1
        assert!(score_1 < score_0);
        assert!(score_2 < score_1);
    }

    #[test]
    fn test_importance_score_multiplier() {
        let scorer = ImportanceScorer::new();

        let target = create_test_chunk(2, "target.rs");
        let rel = create_test_relationship(EdgeType::Calls, 1);

        // High importance chunk
        let mut high_importance = create_test_chunk(1, "high.rs");
        high_importance.importance_score = Some(5.0);
        let score_high = scorer.score(&high_importance, &rel, &target);

        // Low importance chunk
        let mut low_importance = create_test_chunk(3, "low.rs");
        low_importance.importance_score = Some(0.5);
        let score_low = scorer.score(&low_importance, &rel, &target);

        // High importance should score much higher
        assert!(score_high > score_low);
        assert!(score_high / score_low > 8.0); // Approximately 10x difference
    }

    #[test]
    fn test_recency_score_integration() {
        let scorer = ImportanceScorer::new();

        let target = create_test_chunk(2, "target.rs");
        let rel = create_test_relationship(EdgeType::Calls, 1);

        // Recent chunk
        let mut recent = create_test_chunk(1, "recent.rs");
        recent.recency_score = Some(1.0);
        let score_recent = scorer.score(&recent, &rel, &target);

        // Old chunk
        let mut old = create_test_chunk(3, "old.rs");
        old.recency_score = Some(0.2);
        let score_old = scorer.score(&old, &rel, &target);

        // Recent should score higher
        assert!(score_recent > score_old);
    }

    #[test]
    fn test_churn_score_inverse_relationship() {
        let scorer = ImportanceScorer::new();

        let target = create_test_chunk(2, "target.rs");
        let rel = create_test_relationship(EdgeType::Calls, 1);

        // Stable chunk (low churn)
        let mut stable = create_test_chunk(1, "stable.rs");
        stable.churn_score = Some(0.1);
        let score_stable = scorer.score(&stable, &rel, &target);

        // Churny chunk (high churn)
        let mut churny = create_test_chunk(3, "churny.rs");
        churny.churn_score = Some(5.0);
        let score_churny = scorer.score(&churny, &rel, &target);

        // Stable should score higher (inverse relationship)
        assert!(score_stable > score_churny);
    }

    #[test]
    fn test_directory_bonus() {
        let scorer = ImportanceScorer::new();

        let target = create_test_chunk(2, "src/modules/target.rs");
        let rel = create_test_relationship(EdgeType::Calls, 1);

        // Same directory
        let same_dir = create_test_chunk(1, "src/modules/helper.rs");
        let score_same = scorer.score(&same_dir, &rel, &target);

        // Different directory
        let diff_dir = create_test_chunk(3, "src/utils/helper.rs");
        let score_diff = scorer.score(&diff_dir, &rel, &target);

        // Same directory should get 1.3x bonus
        assert!(score_same > score_diff);
        let ratio = score_same / score_diff;
        assert!((ratio - 1.3).abs() < 0.01, "Expected ratio ~1.3, got {}", ratio);
    }

    #[test]
    fn test_same_directory_detection() {
        let scorer = ImportanceScorer::new();

        // Same directory
        assert!(scorer.same_directory("src/modules/a.rs", "src/modules/b.rs"));

        // Different directories
        assert!(!scorer.same_directory("src/modules/a.rs", "src/utils/b.rs"));

        // Root level files
        assert!(scorer.same_directory("a.rs", "b.rs"));

        // One in root, one in directory
        assert!(!scorer.same_directory("a.rs", "src/b.rs"));
    }

    #[test]
    fn test_missing_metadata_defaults() {
        let scorer = ImportanceScorer::new();

        let target = create_test_chunk(2, "target.rs");
        let rel = create_test_relationship(EdgeType::Calls, 1);

        // Chunk with all None metadata
        let mut no_metadata = create_test_chunk(1, "target.rs");
        no_metadata.importance_score = None;
        no_metadata.recency_score = None;
        no_metadata.churn_score = None;

        // Should not crash, should return reasonable score
        let score = scorer.score(&no_metadata, &rel, &target);
        assert!(score > 0.0);
        assert!(score < 100.0);
    }

    #[test]
    fn test_score_clamping() {
        // Create config that could produce very high scores
        let config = ScoringConfig::new().with_base_score(10.0);
        let scorer = ImportanceScorer::with_config(config);

        let mut target = create_test_chunk(2, "target.rs");
        target.importance_score = Some(50.0);

        let mut chunk = create_test_chunk(1, "target.rs");
        chunk.importance_score = Some(50.0);

        let rel = create_test_relationship(EdgeType::TestOf, 0);

        let score = scorer.score(&chunk, &rel, &target);

        // Should be clamped to maximum of 100.0
        assert!(score <= 100.0);
    }

    #[test]
    fn test_test_relationship_scores_highest() {
        let scorer = ImportanceScorer::new();

        let target = create_test_chunk(2, "target.rs");

        // All at same distance, same directory
        let chunk = create_test_chunk(1, "target.rs");

        let rel_test = create_test_relationship(EdgeType::TestOf, 1);
        let rel_calls = create_test_relationship(EdgeType::Calls, 1);
        let rel_imports = create_test_relationship(EdgeType::Imports, 1);

        let score_test = scorer.score(&chunk, &rel_test, &target);
        let score_calls = scorer.score(&chunk, &rel_calls, &target);
        let score_imports = scorer.score(&chunk, &rel_imports, &target);

        // Test relationship should score highest
        assert!(score_test > score_calls);
        assert!(score_calls > score_imports);
    }

    #[test]
    fn test_combined_scoring_scenario() {
        let scorer = ImportanceScorer::new();

        // Scenario: Finding best related chunk for a target
        let target = create_test_chunk(100, "src/api/handler.rs");

        // Option 1: Test file in same directory, high importance
        let mut test_chunk = create_test_chunk(1, "src/api/handler.test.rs");
        test_chunk.importance_score = Some(2.0);
        test_chunk.recency_score = Some(0.9);
        test_chunk.churn_score = Some(0.1);
        let test_rel = create_test_relationship(EdgeType::TestOf, 1);
        let test_score = scorer.score(&test_chunk, &test_rel, &target);

        // Option 2: Caller from different directory, moderate importance, distant
        let mut caller_chunk = create_test_chunk(2, "src/controllers/user.rs");
        caller_chunk.importance_score = Some(1.5);
        caller_chunk.recency_score = Some(0.5);
        caller_chunk.churn_score = Some(0.3);
        let caller_rel = create_test_relationship(EdgeType::Calls, 2);
        let caller_score = scorer.score(&caller_chunk, &caller_rel, &target);

        // Option 3: Import from same directory, low importance
        let mut import_chunk = create_test_chunk(3, "src/api/types.rs");
        import_chunk.importance_score = Some(0.5);
        import_chunk.recency_score = Some(0.3);
        import_chunk.churn_score = Some(0.5);
        let import_rel = create_test_relationship(EdgeType::Imports, 1);
        let import_score = scorer.score(&import_chunk, &import_rel, &target);

        // Test should score highest (test relationship + same dir + good metadata)
        assert!(test_score > caller_score);
        assert!(test_score > import_score);

        // Print for debugging
        println!("Test score: {:.4}", test_score);
        println!("Caller score: {:.4}", caller_score);
        println!("Import score: {:.4}", import_score);
    }
}
