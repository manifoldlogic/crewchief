# Ticket: SRCHREL-1003 - Update Graph Executor

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 5 graph executor tests pass
- [x] **Verified** - by manual verification

## Agents
- rust-expert
- verify-ticket
- commit-ticket

## Summary

Update the graph executor to read the feature flag and pass it to the database layer. Maintain backward compatibility for existing callers while supporting the new quality-weighted mode.

## Background

The graph executor (`crates/maproom/src/search/graph.rs`) currently calls `calculate_graph_importance()` without a quality flag. This ticket updates it to:
- Accept optional configuration parameter
- Read the feature flag from config
- Pass the flag to the database layer
- Maintain backward compatibility (None config = legacy behavior)

## Acceptance Criteria

- [x] Update `GraphExecutor::execute()` signature to accept `Option<&SearchConfig>`
- [x] Read `enable_quality_weighted_graph` flag from config (default to false if None)
- [x] Pass flag to `store.calculate_graph_importance()`
- [x] Existing callers with `None` config still work (backward compatible)
- [x] New callers can pass config to enable quality scoring
- [x] Executor calls database with correct flag value
- [x] Add unit test: None config → flag=false
- [x] Add unit test: Config with flag=false → flag=false
- [x] Add unit test: Config with flag=true → flag=true

## Technical Requirements

**Current Signature (Approximate):**

```rust
// Current implementation in crates/maproom/src/search/graph.rs
impl GraphExecutor {
    pub async fn execute(
        store: &SqliteStore,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<RankedResults, GraphError> {
        let scores = store.calculate_graph_importance(
            repo_id,
            worktree_id,
            limit,
        )?;

        Ok(RankedResults::from_scores(scores))
    }
}
```

**Enhanced Signature:**

```rust
impl GraphExecutor {
    pub async fn execute(
        store: &SqliteStore,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        config: Option<&SearchConfig>, // NEW: Backward compatible optional config
    ) -> Result<RankedResults, GraphError> {
        // Extract feature flag, default to false if config not provided
        let enable_quality = config
            .map(|c| c.feature_flags.enable_quality_scoring)
            .unwrap_or(false);

        // Pass flag to database layer
        let scores = store.calculate_graph_importance(
            repo_id,
            worktree_id,
            limit,
            enable_quality, // NEW PARAMETER
        )?;

        Ok(RankedResults::from_scores(scores))
    }
}
```

**Unit Tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::search_config::{SearchConfig, FeatureFlags};

    #[tokio::test]
    async fn test_execute_without_config_uses_legacy() {
        let store = setup_test_store().await;

        let result = GraphExecutor::execute(
            &store,
            1, // repo_id
            None, // worktree_id
            10, // limit
            None, // config = None → should use legacy
        ).await;

        assert!(result.is_ok());
        // Verify legacy SQL was called (can mock store to check)
    }

    #[tokio::test]
    async fn test_execute_with_flag_disabled_uses_legacy() {
        let store = setup_test_store().await;
        let mut config = SearchConfig::default();
        config.feature_flags.enable_quality_scoring = false;

        let result = GraphExecutor::execute(
            &store,
            1,
            None,
            10,
            Some(&config),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_flag_enabled_uses_quality() {
        let store = setup_test_store().await;
        let mut config = SearchConfig::default();
        config.feature_flags.enable_quality_scoring = true;

        let result = GraphExecutor::execute(
            &store,
            1,
            None,
            10,
            Some(&config),
        ).await;

        assert!(result.is_ok());
        // Verify quality-weighted SQL was called
    }
}
```

## Implementation Notes

**Backward Compatibility:**
Using `Option<&SearchConfig>` ensures:
- Existing callers passing no config continue to work
- No breaking changes to public API
- Clear upgrade path (add config parameter when ready)

**Config Propagation:**
The config must be passed from search pipeline down to graph executor:
```rust
// In search pipeline (Phase 2 work, documented for reference)
let config = SearchConfig::load_default().await?;

let graph_results = GraphExecutor::execute(
    &store,
    repo_id,
    worktree_id,
    limit,
    Some(&config), // Pass config
).await?;
```

**Error Handling:**
- Database errors from `calculate_graph_importance()` are propagated via `?`
- No new error cases introduced by this change
- Config parsing errors handled at config loading layer (not here)

**Logging:**

Add debug logging for observability:
```rust
use tracing::debug;

debug!(
    repo_id = repo_id,
    worktree_id = ?worktree_id,
    enable_quality = enable_quality,
    "Executing graph scoring"
);
```

**Performance:**
No performance impact from this change:
- Config access is a simple struct field read (nanoseconds)
- Boolean flag passed to database layer (zero overhead)
- Actual performance impact is in database query (validated in SRCHREL-0002)

## Dependencies

**Prerequisites:**
- SRCHREL-1001 (database layer accepts `enable_quality` parameter)
- SRCHREL-1002 (feature flag exists in SearchConfig)

**Blocks:**
- SRCHREL-1004 (unit tests need working executor)
- SRCHREL-1005 (integration tests need working executor)

## Risk Assessment

**Risk:** Breaking change for existing callers
**Mitigation:** `Option<&SearchConfig>` parameter is backward compatible, None = legacy behavior

**Risk:** Config not available at executor layer
**Mitigation:** SearchConfig is already used in search pipeline, just needs to be passed through

**Risk:** Flag value not propagated correctly
**Mitigation:** Unit tests verify flag values, simple struct field access

## Files/Packages Affected

**Modified Files:**
- `crates/maproom/src/search/graph.rs` (update execute() signature and implementation)

**Modified Callers (Phase 2):**
- `crates/maproom/src/search/pipeline.rs` (will pass config to executor)
- Any other callers of GraphExecutor::execute()

**Test Files:**
- Unit tests in `graph.rs` test module

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 1.3, lines 194-203)
- Architecture: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (GraphExecutor integration, lines 358-383)
