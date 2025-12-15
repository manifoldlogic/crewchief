# Ticket: SRCHREL-2003 - Pipeline Integration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- search-engineer
- verify-ticket
- commit-ticket

## Summary

Load configuration in search pipeline initialization and pass it to graph executor. Implement fusion weight override logic.

## Acceptance Criteria

- [x] Load `SearchConfig` in search pipeline initialization
- [x] Pass config to `GraphExecutor::execute()`
- [x] Implement fusion weight override from `graph_importance.fusion_weight_override`
- [x] Config changes reflected in search results (no restart needed if hot reload exists)
- [x] Integration test verifies config propagates correctly
- [x] Fusion weight override renormalizes other weights
- [x] Default fusion weight (0.10) used if no override

## Technical Requirements

**Pipeline Integration:**

```rust
// In src/search/pipeline.rs
pub struct SearchPipeline {
    config: SearchConfig,
    store: Arc<SqliteStore>,
}

impl SearchPipeline {
    pub async fn new() -> Result<Self, SearchError> {
        let config = SearchConfig::load_default().await?;
        let store = Arc::new(SqliteStore::connect().await?);

        Ok(Self { config, store })
    }

    pub async fn execute(&self, query: &Query) -> Result<SearchResults> {
        // Parallel executors
        let graph_results = GraphExecutor::execute(
            &self.store,
            query.repo_id,
            query.worktree_id,
            query.limit,
            Some(&self.config), // Pass config
        ).await?;

        // Fusion with override
        let fusion_weights = self.calculate_fusion_weights();
        let results = fuse_results(..., &fusion_weights);

        Ok(results)
    }

    fn calculate_fusion_weights(&self) -> FusionWeights {
        let mut weights = FusionWeights::default();

        if let Some(graph_override) = self.config.graph_importance.fusion_weight_override {
            weights.graph = graph_override;
            // Renormalize other weights
            let remaining = 1.0 - graph_override;
            let scale = remaining / (weights.fts + weights.vector + weights.recency + weights.churn);
            weights.fts *= scale;
            weights.vector *= scale;
            weights.recency *= scale;
            weights.churn *= scale;
        }

        weights
    }
}
```

## Dependencies

**Prerequisites:**
- SRCHREL-2002 (SQL parameterization complete)

**Blocks:**
- SRCHREL-2004 (performance benchmarking needs integrated pipeline)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 2.3, lines 300-303)
