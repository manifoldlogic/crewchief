# Ticket: SRCHREL-3001 - Configuration Documentation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- technical-writer
- verify-ticket
- commit-ticket

## Summary

Create comprehensive documentation for quality-weighted graph scoring configuration, including YAML examples, weight tuning guidelines, and troubleshooting tips.

## Acceptance Criteria

- [ ] Document YAML configuration schema
- [ ] Provide example configurations for common scenarios
- [ ] Explain what each weight parameter does
- [ ] Provide tuning guidelines (when to adjust weights)
- [ ] Document fusion weight override usage
- [ ] Include troubleshooting section
- [ ] Add configuration validation error messages
- [ ] Document performance implications
- [ ] Provide migration guide from Phase 1 (environment variable → YAML config)

## Technical Requirements

**Documentation Sections:**

1. **Configuration Schema Reference**
2. **Quick Start Examples**
3. **Weight Tuning Guide**
4. **Fusion Weight Override**
5. **Troubleshooting**
6. **Migration Guide**

**Example Configuration:**

```yaml
# Default configuration (recommended starting point)
graph_importance:
  enable_quality_scoring: true
  edge_quality_weights:
    production_code: 1.0  # Baseline weight for production code edges
    test_code: 0.5        # Penalty for test code edges (50% weight)
    calls: 1.0            # Weight for call edges (only type in current version)
  # fusion_weight_override: 0.15  # Optional: override default 0.10

# Conservative configuration (subtle quality weighting)
graph_importance:
  enable_quality_scoring: true
  edge_quality_weights:
    production_code: 1.0
    test_code: 0.7        # Less aggressive penalty
    calls: 1.0

# Aggressive configuration (strong quality weighting)
graph_importance:
  enable_quality_scoring: true
  edge_quality_weights:
    production_code: 1.0
    test_code: 0.3        # Strong penalty for test code
    calls: 1.0
  fusion_weight_override: 0.20  # Increase graph signal influence
```

**Tuning Guidelines:**

```markdown
## When to Adjust Weights

### Test Code Weight

**Increase test_code weight (0.5 → 0.6-0.7):**
- Test utilities are ranking too low
- Integration tests contain important architectural insights
- False negative: Test code incorrectly penalized

**Decrease test_code weight (0.5 → 0.3-0.4):**
- Test code still ranks too high
- Production code not getting enough boost
- False positive: Test code still dominates results

### Fusion Weight Override

**Increase fusion weight (0.10 → 0.15-0.20):**
- Graph relationships are strong indicators of importance
- Keyword matching less reliable in your codebase
- Want to boost centrally-called code more

**Decrease fusion weight (0.10 → 0.05-0.08):**
- Graph signal dominating keyword matches
- Too much emphasis on call frequency
- Recent/modified code not ranking well enough
```

## Dependencies

**Prerequisites:**
- SRCHREL-2001 (configuration schema implemented)
- SRCHREL-2005 (have evaluation results to inform documentation)

**Blocks:**
- None (documentation can evolve)

## Files/Packages Affected

**New Files:**
- `docs/configuration/graph-quality-scoring.md`
- `docs/guides/tuning-search-quality.md`

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Phase 3, line 342)
