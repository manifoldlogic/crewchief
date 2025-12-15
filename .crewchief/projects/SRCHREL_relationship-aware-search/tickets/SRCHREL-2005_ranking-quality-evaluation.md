# Ticket: SRCHREL-2005 - Ranking Quality Evaluation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- search-engineer
- test-engineer
- verify-ticket
- commit-ticket

## Summary

Curate 50 representative queries and manually evaluate ranking quality. Compare old vs enhanced rankings and validate ≥64% improved, ≤4% degraded.

## Acceptance Criteria

- [ ] Curate 50 diverse test queries covering different code types
- [ ] Run queries with both legacy and enhanced modes
- [ ] Manual evaluation: Is top result architecturally important?
- [ ] Count improved queries (central code moved to top 3)
- [ ] Count degraded queries (central code dropped from top 3)
- [ ] Validate ≥32/50 improved (64%), ≤2/50 degraded (4%)
- [ ] Document evaluation results
- [ ] Identify patterns in improvements/degradations

## Technical Requirements

**Evaluation Process:**

For each of 50 queries:
1. Run with legacy mode, record top 3
2. Run with enhanced mode, record top 3
3. Manually identify "most important" result
4. Compare: Did ranking improve, stay same, or degrade?

**Quantitative Metrics:**

```
Improved: Central code rank improved (e.g., 4→1, 5→2)
Same: Top 3 unchanged or already optimal
Degraded: Central code rank worsened (e.g., 2→4)

Target:
- ≥32/50 (64%) improved
- ≤2/50 (4%) degraded
```

## Dependencies

**Prerequisites:**
- SRCHREL-2003 (pipeline integrated)
- SRCHREL-2004 (performance validated)

**Blocks:**
- None (Phase 2 can complete after this)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 2.5, lines 312-317)
