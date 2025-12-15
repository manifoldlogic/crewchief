# Ticket: SRCHREL-2002 - SQL Parameterization

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary

Replace hardcoded quality weights in SQL query with parameters from configuration. Update `calculate_graph_importance()` to accept `EdgeQualityWeights` struct.

## Acceptance Criteria

- [ ] Replace hardcoded 1.0 and 0.5 weights with parameters
- [ ] Update `calculate_graph_importance_quality()` to accept `EdgeQualityWeights`
- [ ] Pass weights from config through executor to database layer
- [ ] SQL query uses weight parameters correctly
- [ ] Results change when weights are modified
- [ ] Unit tests validate parameterization works
- [ ] Backward compatibility maintained (can still use defaults)

## Technical Requirements

**Updated Function Signature:**

```rust
fn calculate_graph_importance_quality(
    &self,
    repo_id: i64,
    worktree_id: Option<i64>,
    limit: usize,
    weights: &EdgeQualityWeights, // NEW PARAMETER
) -> Result<Vec<(i64, f32)>, DbError>
```

**Parameterized SQL:**

```sql
-- Use variables for weights
WITH quality_edges AS (
  SELECT
    ce.dst_chunk_id as chunk_id,
    CASE ce.type
      WHEN 'calls' THEN ?4  -- weights.calls parameter
      ELSE 1.0
    END *
    CASE
      WHEN src_file.relpath LIKE '%/test/%' ... THEN ?5  -- weights.test_code
      ELSE ?6  -- weights.production_code
    END as edge_quality
  FROM chunk_edges ce
  -- ... rest of query
)
```

## Dependencies

**Prerequisites:**
- SRCHREL-2001 (config schema defined)

**Blocks:**
- SRCHREL-2003 (pipeline integration needs parameterized query)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 2.2, lines 294-298)
