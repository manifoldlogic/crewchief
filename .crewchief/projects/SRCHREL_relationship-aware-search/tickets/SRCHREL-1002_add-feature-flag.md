# Ticket: SRCHREL-1002 - Add Feature Flag Support

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-expert
- verify-ticket
- commit-ticket

## Summary

Add feature flag support to enable/disable quality-weighted graph scoring. Implement the approach chosen in SRCHREL-0004, defaulting to disabled for safe rollout.

## Background

Phase 1 needs a simple mechanism to toggle between legacy and quality-weighted graph scoring. The feature flag allows:
- Deployment with quality scoring disabled (safe)
- Easy enable for testing and validation
- Quick rollback if issues arise
- Clear upgrade path to Phase 2 configuration

Based on SRCHREL-0004 findings, implement the chosen approach (environment variable, config boolean, or hybrid).

## Acceptance Criteria

- [ ] Implement feature flag according to SRCHREL-0004 design decision
- [ ] Feature flag defaults to `false` (disabled, safe rollout)
- [ ] Flag is accessible at graph executor layer
- [ ] Configuration loads successfully with flag present
- [ ] Configuration loads successfully without flag (backward compatible)
- [ ] Flag can be changed without code recompilation
- [ ] Document flag usage in configuration file or README
- [ ] Add unit test: flag=false uses legacy behavior
- [ ] Add unit test: flag=true uses enhanced behavior
- [ ] Add unit test: missing flag defaults to false

## Technical Requirements

**Implementation will follow design from SRCHREL-0004. Example for Option B (Config Boolean):**

```rust
// In crates/maproom/src/config/search_config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    // ... existing fields ...

    #[serde(default)]
    pub feature_flags: FeatureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeatureFlags {
    #[serde(default)]
    pub enable_quality_scoring: bool, // Default: false
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_quality_scoring: false,
        }
    }
}
```

**Configuration File Example:**

```yaml
# config/maproom-search.yml
feature_flags:
  enable_quality_scoring: false  # Set to true to enable quality-weighted scoring
```

**Unit Tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flag_defaults_to_false() {
        let config = SearchConfig::default();
        assert_eq!(config.feature_flags.enable_quality_scoring, false);
    }

    #[test]
    fn test_feature_flag_loads_from_config() {
        let yaml = r#"
feature_flags:
  enable_quality_scoring: true
        "#;
        let config: SearchConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.feature_flags.enable_quality_scoring, true);
    }

    #[test]
    fn test_backward_compat_without_feature_flags() {
        let yaml = r#"
# Old config without feature_flags section
some_other_field: value
        "#;
        let config: Result<SearchConfig, _> = serde_yaml::from_str(yaml);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().feature_flags.enable_quality_scoring, false);
    }

    #[test]
    fn test_env_var_override() {
        // If implementing Option C (hybrid)
        std::env::set_var("MAPROOM_ENABLE_QUALITY_SCORING", "true");
        let enable = load_quality_flag(&SearchConfig::default());
        assert_eq!(enable, true);
        std::env::remove_var("MAPROOM_ENABLE_QUALITY_SCORING");
    }
}
```

## Implementation Notes

**Safe Default:**
The flag MUST default to `false` to ensure:
- Existing deployments are not affected
- New deployments start with legacy behavior
- Enable is explicit and intentional

**Configuration Validation:**
If using YAML config:
- Validate on deserialization (serde handles this automatically)
- Reject invalid boolean values
- Provide clear error message if config malformed

**Environment Variable (if applicable):**
- Name: `MAPROOM_ENABLE_QUALITY_SCORING`
- Values: `"true"` or `"false"` (case-insensitive)
- Default if unset: `false`

**Documentation:**
Update relevant config documentation:
- Describe what the flag does
- Explain when to enable it
- Note performance implications
- Provide rollback instructions

**Access Pattern:**

The flag will be accessed in the graph executor (SRCHREL-1003):
```rust
let enable_quality = config.feature_flags.enable_quality_scoring;
```

## Dependencies

**Prerequisites:**
- SRCHREL-0004 (config design decision made)

**Blocks:**
- SRCHREL-1003 (graph executor needs flag to toggle behavior)

**Related:**
- SRCHREL-1001 (database layer needs boolean parameter, but doesn't load config)

## Risk Assessment

**Risk:** Flag not accessible where needed
**Mitigation:** Test access pattern in graph executor layer, verify config propagation

**Risk:** Default value wrong (accidentally enabled)
**Probability:** Low (code review will catch)
**Mitigation:** Unit test validates default is false, PR review checklist

**Risk:** Config file changes break existing deployments
**Mitigation:** Use `#[serde(default)]` for backward compatibility, test with old config files

## Files/Packages Affected

**Modified Files:**
- `crates/maproom/src/config/search_config.rs` (add FeatureFlags struct)
- Config file example (document the flag)
- README or docs explaining the flag

**New Test Files:**
- Unit tests in `search_config.rs` test module

**Dependencies:**
- `serde` (already in Cargo.toml)
- `serde_yaml` (already in Cargo.toml)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 1.2, lines 183-192)
- Architecture: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (Configuration approach, lines 66-177)
- Config design: Results from SRCHREL-0004
