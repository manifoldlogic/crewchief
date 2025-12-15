# Ticket: SRCHREL-0004 - Config Integration Design Validation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-expert
- verify-ticket
- commit-ticket

## Summary

Validate the configuration loading approach for the quality scoring feature flag before Phase 1 implementation. Design the simplest path to enable/disable quality scoring without full YAML configuration infrastructure.

## Background

Phase 1 aims to prove the quality-weighted algorithm works before building full configuration infrastructure. We need a simple feature flag mechanism that:
- Allows easy enable/disable of quality scoring
- Doesn't require complex YAML schema changes
- Provides clear upgrade path to Phase 2 full configuration

The existing codebase has `SearchConfig` in `crates/maproom/src/config/search_config.rs`. This ticket validates the integration approach.

## Acceptance Criteria

- [ ] Review existing `SearchConfig` structure in `src/config/search_config.rs`
- [ ] Decide on Phase 1 feature flag approach (environment variable vs simple config boolean)
- [ ] Write example configuration snippet for chosen approach
- [ ] Test configuration loading (verify it deserializes correctly)
- [ ] Verify backward compatibility (old configs without the flag still work)
- [ ] Document chosen approach in architecture.md
- [ ] Create code example showing how graph executor will access the flag
- [ ] Confirm flag can toggle without code changes (runtime configuration)

## Technical Requirements

**Option A: Environment Variable (Simplest - Recommended for Phase 1)**

```rust
// In graph executor
pub fn execute(...) -> Result<RankedResults> {
    let enable_quality = std::env::var("MAPROOM_ENABLE_QUALITY_SCORING")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false); // Default: disabled

    if enable_quality {
        // New quality-weighted path
    } else {
        // Existing legacy path
    }
}
```

**Pros:**
- No config file changes needed
- Easy to toggle in deployment (set env var)
- Zero impact on existing code
- Clear rollback path (unset env var)

**Cons:**
- Not discoverable (no config file documentation)
- Less structured than YAML

---

**Option B: Simple Config Boolean (Better for Phase 2 Transition)**

```yaml
# In existing config/maproom-search.yml
feature_flags:
  enable_quality_scoring: false  # Simple boolean toggle
```

```rust
// In src/config/search_config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    // ... existing fields ...

    #[serde(default)]
    pub feature_flags: FeatureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    #[serde(default)]
    pub enable_quality_scoring: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_quality_scoring: false, // Safe default
        }
    }
}
```

**Pros:**
- Discoverable in config file
- Structured approach
- Easy transition to Phase 2 full config
- Config validation on load

**Cons:**
- Requires config file changes
- Slightly more complex than env var

---

**Option C: Hybrid (Environment Variable Override)**

```rust
// Check env var first, fall back to config
let enable_quality = std::env::var("MAPROOM_ENABLE_QUALITY_SCORING")
    .map(|v| v.to_lowercase() == "true")
    .unwrap_or_else(|_| {
        config.feature_flags.enable_quality_scoring
    });
```

**Pros:**
- Best of both worlds
- Allows emergency override without config changes

**Cons:**
- More complex logic
- Two sources of truth

---

**Backward Compatibility Test:**

```rust
#[test]
fn test_old_config_without_feature_flags() {
    let old_config = r#"
    # Old config without feature_flags section
    some_existing_field: value
    "#;

    let config: SearchConfig = serde_yaml::from_str(old_config).unwrap();
    assert_eq!(config.feature_flags.enable_quality_scoring, false);
    // Should use default value
}
```

**Graph Executor Integration Pattern:**

```rust
// In src/search/graph.rs
impl GraphExecutor {
    pub async fn execute(
        store: &SqliteStore,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        config: Option<&SearchConfig>, // Backward compatible: None = legacy
    ) -> Result<RankedResults> {
        let enable_quality = config
            .map(|c| c.feature_flags.enable_quality_scoring)
            .unwrap_or(false);

        let scores = store.calculate_graph_importance(
            repo_id,
            worktree_id,
            limit,
            enable_quality, // Pass flag to database layer
        )?;

        Ok(RankedResults::from_scores(scores))
    }
}
```

## Implementation Notes

**Decision Criteria:**

Choose based on:
1. **Simplicity:** Fastest to implement
2. **Discoverability:** Can users find the flag?
3. **Phase 2 Path:** Easy upgrade to full config?
4. **Rollback:** Can we toggle quickly in emergency?

**Recommendation:**
- **Phase 1:** Option A (environment variable) for speed
- **Phase 2:** Migrate to Option B (config boolean) when adding weight configuration
- **Production:** Option C (hybrid) for operational flexibility

**Config File Location:**

Existing pattern in codebase:
- Config loaded from `~/.maproom/config.yml` or workspace-specific config
- Async loading: `SearchConfig::load_default().await`
- Already supports `#[serde(default)]` for backward compatibility

**Testing Requirements:**

1. Test: Config with flag=true loads successfully
2. Test: Config with flag=false loads successfully
3. Test: Old config without flag loads (default=false)
4. Test: Invalid config fails gracefully with clear error
5. Test: Environment variable override works (if hybrid approach chosen)

## Dependencies

**Prerequisites:**
- None (independent validation task)

**Blocks:**
- SRCHREL-1002 (feature flag implementation depends on chosen approach)

## Risk Assessment

**Risk:** Config changes break existing deployments
**Mitigation:** Use `#[serde(default)]` to ensure backward compatibility, test with old config files

**Risk:** Feature flag not accessible at graph executor layer
**Mitigation:** Validate config propagation path from pipeline to executor, may require small refactor

**Risk:** Hot reload not supported, requires restart
**Expected:** Phase 1 accepts restart requirement, Phase 3 can add hot reload
**Mitigation:** Document restart requirement in rollout plan

## Files/Packages Affected

**Files to Review:**
- `crates/maproom/src/config/search_config.rs` (existing config structure)
- `crates/maproom/src/search/pipeline.rs` (config loading and propagation)
- `crates/maproom/src/search/graph.rs` (graph executor signature)

**Files to Modify (based on chosen approach):**
- `crates/maproom/src/config/search_config.rs` (add FeatureFlags struct if Option B)
- Example config file (document the flag)

**Documentation to Create:**
- `.crewchief/projects/SRCHREL_relationship-aware-search/planning/config-design.md` (chosen approach and rationale)
- Update architecture.md with config integration section

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Prerequisite 4, lines 114-134)
- Architecture: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (Configuration approach, lines 66-177)
