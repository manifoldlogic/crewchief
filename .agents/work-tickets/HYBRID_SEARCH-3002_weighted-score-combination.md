# Ticket: HYBRID_SEARCH-3002: Weighted Score Combination

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- search-quality-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement configurable weighted linear combination for fusing multiple search signal scores (FTS, vector, graph, recency, churn) into a single unified ranking score with support for runtime weight tuning and debug mode score breakdown.

## Background
After implementing the basic fusion infrastructure in HYBRID_SEARCH-2003, we need a sophisticated weighted combination system that allows fine-tuning of how different search signals contribute to the final ranking. Different use cases may benefit from different weight configurations (e.g., prioritizing semantic similarity vs. keyword matching vs. code importance), so we need a flexible, configurable system with debugging capabilities to understand score contributions.

The architecture defines a weighted linear combination approach where each signal (FTS, vector similarity, graph centrality, recency, churn) is multiplied by a configurable weight and summed to produce the final score. This ticket implements that system with hot-reload support and debugging tools.

## Acceptance Criteria
- [ ] Weighted fusion is configurable via YAML configuration file
- [ ] Weight tuning CLI interface created for experimentation
- [ ] Debug mode shows detailed score breakdown for each result
- [ ] Weight impacts are documented with examples and guidelines
- [ ] Default weights (FTS=0.4, vector=0.35, graph=0.1, recency=0.1, churn=0.05) are implemented
- [ ] Configuration hot-reload is supported without restart
- [ ] Weights are validated (must sum to 1.0 or normalized automatically)
- [ ] Score breakdown shows contribution of each signal to final score

## Technical Requirements
- Implement `WeightedFusion` struct with `FusionWeights` configuration as defined in architecture
- Create configuration schema for `fusion.weights` in YAML format
- Support hot-reload of weights from `maproom-search.yml` configuration file
- Implement score normalization to ensure weights sum to 1.0
- Add debug mode that outputs score breakdown showing each signal's contribution
- Create `tune_weights` CLI command for interactive weight adjustment
- Implement churn score inversion: `churn_weight * (1.0 / (1.0 + churn_score))`
- Document default weights and provide tuning guidelines
- Add validation to ensure all weights are non-negative
- Support weight override via CLI flags for quick experimentation

## Implementation Notes

### Core Weighted Fusion Implementation
```rust
// crates/maproom/src/search/fusion/weighted.rs
pub struct WeightedFusion {
    weights: FusionWeights,
}

impl WeightedFusion {
    pub fn fuse(&self, signals: SearchSignals) -> FusedScore {
        let fts_contrib = self.weights.fts * signals.fts_score;
        let vector_contrib = self.weights.vector * signals.vector_score;
        let graph_contrib = self.weights.graph * signals.graph_score;
        let recency_contrib = self.weights.recency * signals.recency_score;
        let churn_contrib = self.weights.churn * (1.0 / (1.0 + signals.churn_score));

        let final_score = fts_contrib + vector_contrib + graph_contrib
                        + recency_contrib + churn_contrib;

        FusedScore {
            score: final_score,
            breakdown: ScoreBreakdown {
                fts: fts_contrib,
                vector: vector_contrib,
                graph: graph_contrib,
                recency: recency_contrib,
                churn: churn_contrib,
            }
        }
    }
}
```

### Configuration Schema
```rust
// crates/maproom/src/search/fusion/config.rs
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FusionWeights {
    pub fts: f32,      // Default: 0.4
    pub vector: f32,   // Default: 0.35
    pub graph: f32,    // Default: 0.1
    pub recency: f32,  // Default: 0.1
    pub churn: f32,    // Default: 0.05
}

impl FusionWeights {
    pub fn validate(&self) -> Result<()> {
        // Ensure all weights are non-negative
        // Optionally normalize to sum to 1.0
    }

    pub fn normalize(&mut self) {
        // Normalize weights to sum to 1.0
    }
}
```

### Debug Mode Score Breakdown
The debug mode should output detailed information about how each signal contributed to the final score. This helps users understand why certain results ranked higher and tune weights accordingly.

### Weight Tuning Interface
Create a CLI command `crewchief-maproom tune-weights` that:
1. Runs a sample query with current weights
2. Shows score breakdown for top results
3. Allows interactive weight adjustment
4. Re-runs query to show impact
5. Saves tuned weights to configuration

### Configuration Hot-Reload
Use a file watcher or periodic reload mechanism to detect changes to `maproom-search.yml` and update weights without restarting the service. This enables rapid experimentation.

### Default Weights Rationale
- **FTS (0.4)**: Highest weight for keyword/exact matches
- **Vector (0.35)**: Strong semantic similarity signal
- **Graph (0.1)**: Moderate boost for important/central code
- **Recency (0.1)**: Moderate boost for recently changed code
- **Churn (0.05)**: Slight penalty for high-churn (unstable) code

## Dependencies
- **HYBRID_SEARCH-2003** (Initial Search Integration) - Must be completed first to provide the fusion infrastructure and `SearchSignals` type

## Risk Assessment
- **Risk**: Default weights may not be optimal for all codebases
  - **Mitigation**: Provide tuning interface and documentation with examples; support per-project weight overrides

- **Risk**: Weight normalization may cause confusion if users expect raw weights to be used
  - **Mitigation**: Clearly document normalization behavior; add flag to disable auto-normalization

- **Risk**: Hot-reload of configuration could cause inconsistent results during reload
  - **Mitigation**: Use atomic configuration updates; document reload behavior; add graceful degradation

- **Risk**: Score breakdown in debug mode may have performance overhead
  - **Mitigation**: Only compute breakdown when debug flag is enabled; optimize breakdown computation

## Files/Packages Affected
- `crates/maproom/src/search/fusion/weighted.rs` - New: Weighted fusion implementation
- `crates/maproom/src/search/fusion/config.rs` - New: FusionWeights struct and validation
- `crates/maproom/src/search/fusion/debug.rs` - New: Score breakdown utilities and formatting
- `crates/maproom/src/search/fusion/mod.rs` - Modified: Export weighted fusion types
- `crates/maproom/src/cli/commands/tune_weights.rs` - New: Weight tuning CLI command
- `crates/maproom/src/cli/commands/mod.rs` - Modified: Register tune_weights command
- `crates/maproom/src/config/mod.rs` - Modified: Add fusion weights to configuration schema
- `crates/maproom/config/maproom-search.yml` - Modified: Add default weights configuration
- `crates/maproom/docs/WEIGHT_TUNING.md` - New: Documentation for weight tuning and impacts
