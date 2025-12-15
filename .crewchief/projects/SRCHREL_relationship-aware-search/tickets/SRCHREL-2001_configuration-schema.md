# Ticket: SRCHREL-2001 - Configuration Schema

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-expert
- verify-ticket
- commit-ticket

## Summary

Add full YAML configuration schema for quality-weighted edge scoring. Define `GraphImportanceConfig` and `EdgeQualityWeights` structs with validated default values from Phase 1.

## Acceptance Criteria

- [ ] Add `GraphImportanceConfig` struct to `SearchConfig`
- [ ] Define `EdgeQualityWeights` struct with all weight fields
- [ ] Implement `Default` trait with validated Phase 1 weights
- [ ] Add config validation (reject negative/extreme weights)
- [ ] Validate weights sum or scale appropriately
- [ ] Add fusion_weight_override field (optional)
- [ ] Configuration loads from YAML successfully
- [ ] Invalid configs fail with clear error messages
- [ ] Backward compatible with existing configs (use defaults)
- [ ] Unit tests for config validation

## Technical Requirements

**Rust Structs:**

```rust
// In crates/maproom/src/config/search_config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    // ... existing fields ...

    #[serde(default)]
    pub graph_importance: GraphImportanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphImportanceConfig {
    #[serde(default)]
    pub enable_quality_scoring: bool,

    #[serde(default)]
    pub edge_quality_weights: EdgeQualityWeights,

    #[serde(default)]
    pub fusion_weight_override: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeQualityWeights {
    #[serde(default = "default_production_code_weight")]
    pub production_code: f32,  // 1.0

    #[serde(default = "default_test_code_weight")]
    pub test_code: f32,        // 0.5

    #[serde(default = "default_calls_weight")]
    pub calls: f32,            // 1.0 (only edge type in Phase 1)
}

fn default_production_code_weight() -> f32 { 1.0 }
fn default_test_code_weight() -> f32 { 0.5 }
fn default_calls_weight() -> f32 { 1.0 }

impl Default for GraphImportanceConfig {
    fn default() -> Self {
        Self {
            enable_quality_scoring: false,
            edge_quality_weights: EdgeQualityWeights::default(),
            fusion_weight_override: None,
        }
    }
}

impl Default for EdgeQualityWeights {
    fn default() -> Self {
        Self {
            production_code: 1.0,
            test_code: 0.5,
            calls: 1.0,
        }
    }
}

impl EdgeQualityWeights {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.production_code < 0.0 || self.production_code > 10.0 {
            return Err(ConfigError::InvalidWeight("production_code must be 0-10"));
        }
        if self.test_code < 0.0 || self.test_code > 10.0 {
            return Err(ConfigError::InvalidWeight("test_code must be 0-10"));
        }
        if self.calls < 0.0 || self.calls > 10.0 {
            return Err(ConfigError::InvalidWeight("calls must be 0-10"));
        }
        Ok(())
    }
}
```

**YAML Configuration:**

```yaml
graph_importance:
  enable_quality_scoring: true
  edge_quality_weights:
    production_code: 1.0
    test_code: 0.5
    calls: 1.0
  fusion_weight_override: 0.15  # Optional
```

## Dependencies

**Prerequisites:**
- SRCHREL-1008 (GO decision from Phase 1.5)

**Blocks:**
- SRCHREL-2002 (SQL parameterization needs config structs)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 2.1, lines 287-292)
- Architecture: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (Config schema, lines 84-162)
