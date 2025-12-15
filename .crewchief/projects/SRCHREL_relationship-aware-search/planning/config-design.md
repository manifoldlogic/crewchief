# Config Integration Design

## Design Decision

**Chosen Approach:** Option B - Config Boolean with Environment Variable Override

The existing infrastructure at `/crates/maproom/src/config/feature_flags.rs` already provides the exact pattern we need. The `FeatureFlags` struct has an existing `enable_graph_signals: bool` field (line 41) that can be repurposed for quality-weighted graph scoring.

## Rationale

### Why Option B (Config Boolean)?

1. **Infrastructure Already Exists**: The `FeatureFlags` struct is already integrated into `SearchConfig` with full YAML deserialization support
2. **Backward Compatibility Built-in**: Uses `#[serde(default)]` pattern, ensuring old configs without the flag work correctly
3. **Environment Variable Override**: Already supported via `SearchConfig::apply_env_overrides()` (line 312-319)
4. **Discoverability**: Flag is documented in config file, unlike pure environment variable approach
5. **Phase 2 Ready**: Same structure can be extended for full weight configuration

### Comparison to Other Options

**Option A (Environment Variable Only):**
- ❌ Not discoverable (hidden implementation detail)
- ❌ No config validation
- ✅ Simplest to implement
- ✅ Easy emergency override

**Option B (Config Boolean):** CHOSEN
- ✅ Discoverable in config file
- ✅ Structured, validated approach
- ✅ Environment variable override available
- ✅ Easy Phase 2 transition
- ✅ Already implemented!

**Option C (Hybrid):**
- ❌ Two sources of truth
- ❌ More complex logic
- ✅ Already available via Option B (env override built-in)

## Existing Infrastructure

### SearchConfig Structure

Located at: `/crates/maproom/src/config/search_config.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchConfig {
    pub embedding: EmbeddingConfig,
    pub fusion: FusionConfig,
    pub performance: PerformanceConfig,
    pub index: IndexConfig,
    pub feature_flags: FeatureFlags,  // Line 47
    // ... other fields
}
```

### FeatureFlags Structure

Located at: `/crates/maproom/src/config/feature_flags.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub enable_vector_search: bool,
    pub enable_hybrid_fusion: bool,
    pub enable_graph_signals: bool,      // Line 41 - CAN BE REPURPOSED
    pub enable_temporal_signals: bool,
    pub enable_query_cache: bool,
    pub enable_hot_reload: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_vector_search: true,
            enable_hybrid_fusion: true,
            enable_graph_signals: true,     // Line 67 - Enabled by default
            enable_temporal_signals: true,
            enable_query_cache: true,
            enable_hot_reload: true,
        }
    }
}
```

### Environment Variable Override

Already implemented in `SearchConfig::apply_env_overrides()` (lines 312-319):

```rust
if let Ok(graph) = std::env::var("MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_GRAPH_SIGNALS") {
    self.feature_flags.enable_graph_signals = graph
        .parse()
        .context("Failed to parse MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_GRAPH_SIGNALS")?;
    debug!(
        "Override: feature_flags.enable_graph_signals = {}",
        self.feature_flags.enable_graph_signals
    );
}
```

## Implementation Strategy

### Phase 1: Repurpose Existing Flag

**Recommendation:** Use the existing `enable_graph_signals` field for quality-weighted scoring.

**Why:**
- Zero config changes needed
- Flag name is semantically correct (it enables graph signals for ranking)
- Backward compatible by default (enabled=true)
- Environment override already works

**Alternative:** Add a new `enable_quality_weighted_graph: bool` field if we want to distinguish between:
- `enable_graph_signals` = basic graph importance (existing)
- `enable_quality_weighted_graph` = quality-weighted graph importance (new)

### Configuration Example

**YAML Config:**
```yaml
# In config/maproom-search.yml
feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true     # Controls quality-weighted graph scoring
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
```

**Environment Variable Override:**
```bash
# Disable quality-weighted graph scoring
export MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_GRAPH_SIGNALS=false

# Enable quality-weighted graph scoring
export MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_GRAPH_SIGNALS=true
```

### Code Integration Pattern

**Graph Executor Access:**

```rust
// In src/search/graph.rs or executor layer
impl GraphExecutor {
    pub async fn execute(
        store: &SqliteStore,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        config: Option<&SearchConfig>,  // Backward compatible: None = use defaults
    ) -> Result<RankedResults> {
        let enable_quality = config
            .map(|c| c.feature_flags.enable_graph_signals)
            .unwrap_or(true);  // Default: enabled

        let scores = store.calculate_graph_importance(
            repo_id,
            worktree_id,
            limit,
            enable_quality,  // Pass flag to database layer
        )?;

        Ok(RankedResults::from_scores(scores))
    }
}
```

**Database Layer Signature Change:**

```rust
// In src/db/sqlite/mod.rs
impl SqliteStore {
    pub fn calculate_graph_importance(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        enable_quality: bool,  // NEW PARAMETER
    ) -> Result<Vec<(i64, f32)>, DbError> {
        if !enable_quality {
            // Old implementation (simple edge counting)
            return self.calculate_graph_importance_legacy(repo_id, worktree_id, limit);
        }

        // New quality-weighted implementation
        // ... execute quality-weighted SQL ...
    }
}
```

## Backward Compatibility

### Guarantees

1. **Old configs without feature_flags section**: Use `Default::default()` (all enabled)
2. **Old configs with feature_flags but missing enable_graph_signals**: Use `true` (default)
3. **Config with enable_graph_signals=false**: Disable quality scoring (fallback to legacy)
4. **Config with enable_graph_signals=true**: Enable quality scoring (new behavior)

### Validation Test

The backward compatibility test (to be created in `crates/maproom/tests/config_backward_compatibility.rs`) will verify:

1. ✅ Old config without `feature_flags` deserializes correctly
2. ✅ Old config with `feature_flags` but missing `enable_graph_signals` uses default (true)
3. ✅ New config with `enable_graph_signals=false` loads correctly
4. ✅ New config with `enable_graph_signals=true` loads correctly
5. ✅ Environment variable override works
6. ✅ Invalid values fail gracefully

## Phase 2 Extension Path

When adding full configuration for edge quality weights:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    // ... existing fields ...

    // Phase 2: Optional detailed configuration
    #[serde(default)]
    pub graph_quality_config: Option<GraphQualityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQualityConfig {
    pub enable: bool,  // Replaces enable_graph_signals for quality-specific control
    pub edge_quality_weights: EdgeQualityWeights,
    pub fusion_weight_override: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeQualityWeights {
    pub production_code: f32,  // 1.0 default
    pub test_code: f32,        // 0.5 default
    pub calls: f32,            // 1.0 default
    // Future: imports, test_of, etc.
}
```

**Phase 2 YAML:**
```yaml
feature_flags:
  enable_graph_signals: true  # Simple toggle (Phase 1)
  graph_quality_config:       # Detailed config (Phase 2)
    enable: true
    edge_quality_weights:
      production_code: 1.0
      test_code: 0.5
      calls: 1.0
    fusion_weight_override: 0.15
```

**Backward Compatibility in Phase 2:**
- If `graph_quality_config` is None, use `enable_graph_signals` value
- If `graph_quality_config` is Some, use `graph_quality_config.enable`
- This allows gradual migration from simple toggle to detailed config

## Configuration Loading

### Async Pattern (Already Implemented)

```rust
// SearchConfig::load_default() is async
pub async fn load_default() -> Result<Self> {
    // Searches default paths, applies env overrides, validates
}
```

**Integration Points:**
- Search pipeline initialization: `SearchPipeline::new()` calls `SearchConfig::load_default().await`
- Daemon startup: Load config once, cache in memory
- CLI commands: Load config per-command (acceptable for CLI usage)

**Performance:**
- First load: ~5ms (file I/O + YAML parsing)
- Cached in pipeline: <0.1ms (memory access)
- No lazy_static needed (pipeline owns config)

## Runtime Configuration

### Phase 1: Restart Required

Changing `enable_graph_signals` requires service restart. This is acceptable for Phase 1.

**Rationale:**
- Hot reload adds complexity
- Phase 1 goal is to prove algorithm works, not operational flexibility
- Production rollout can use gradual deployment (blue/green, canary)

### Phase 2: Optional Hot Reload

If `enable_hot_reload: true`, watch config file for changes and reload.

**Implementation:**
- File watcher on config file
- Reload on change (debounced)
- Atomic swap of config in pipeline
- Log config changes

**Not in Phase 1 scope.**

## Testing Strategy

### Unit Tests

1. **Config Deserialization**
   - Test YAML with flag=true
   - Test YAML with flag=false
   - Test YAML without flag (uses default)
   - Test old YAML without feature_flags section

2. **Environment Variable Override**
   - Test env var sets flag=true
   - Test env var sets flag=false
   - Test env var with invalid value (should error)

3. **Validation**
   - Test config validation passes with valid values
   - Test config validation fails gracefully with invalid values

### Integration Tests

1. **Graph Executor Integration**
   - Test executor with config=None (uses default)
   - Test executor with enable_graph_signals=true
   - Test executor with enable_graph_signals=false
   - Verify flag toggles between legacy and quality-weighted paths

2. **End-to-End**
   - Test search with quality scoring enabled
   - Test search with quality scoring disabled
   - Verify results differ (quality scoring surfaces important code)

## Rollout Plan

### Phase 1: Feature Flag Launch

1. **Deploy with flag=true (enabled by default)**
   - New behavior is the default
   - Users get quality-weighted scoring automatically
   - Can disable via config or env var if issues

2. **Monitor metrics**
   - Graph executor latency (should be <30ms p95)
   - Search result quality (manual inspection)
   - Error rates

3. **Rollback if needed**
   - Set `enable_graph_signals=false` via env var (no code deploy)
   - Or deploy with default changed to `false`

### Phase 2: Remove Flag (Stabilization)

Once quality-weighted scoring is proven stable (e.g., 2-4 weeks):

1. **Remove flag from code**
   - Make quality-weighted scoring the only path
   - Delete legacy implementation
   - Update config to remove flag (or leave as no-op for backward compat)

2. **Update documentation**
   - Remove flag from example configs
   - Archive design docs

## Risk Mitigation

### Risk: Config changes break existing deployments

**Mitigation:** Use `#[serde(default)]` on all new fields, ensure backward compatibility tests pass

### Risk: Flag not accessible at executor layer

**Mitigation:** Validate config propagation path from pipeline to executor, refactor if needed (should be straightforward)

### Risk: Hot reload not supported

**Expected:** Phase 1 accepts restart requirement, document in release notes

### Risk: Users don't know about flag

**Mitigation:** Document in CHANGELOG, update example configs, default to enabled (users get benefit automatically)

## Files Created/Modified

### Created

1. `.crewchief/projects/SRCHREL_relationship-aware-search/planning/config-design.md` (this file)
2. `crates/maproom/tests/config_backward_compatibility.rs` (test file)

### Modified

1. `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (add config integration section)
2. `crates/maproom/src/db/sqlite/mod.rs` (add `enable_quality` parameter to `calculate_graph_importance()`)
3. `crates/maproom/src/search/graph.rs` (access flag from config, pass to database layer)

**Note:** `search_config.rs` and `feature_flags.rs` require NO changes for Phase 1 (infrastructure already exists).

## Conclusion

**Recommendation:** Use existing `enable_graph_signals` flag in `FeatureFlags` for Phase 1.

**Why:**
- Zero config infrastructure work needed
- Backward compatible by default
- Environment variable override already works
- Clear upgrade path to Phase 2
- Semantic naming is correct

**Decision:** Option B (Config Boolean) is the chosen approach, and it's already implemented in the codebase.
