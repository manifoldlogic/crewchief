# Ticket: SRCHREL-3002 - Monitoring Setup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (documentation only)
- [x] **Verified** - by the verify-ticket agent

## Agents
- infrastructure-engineer
- verify-ticket
- commit-ticket

## Summary

Set up metrics documentation and monitoring recommendations for latency and score distributions.

## Acceptance Criteria

- [x] Document existing Prometheus metrics for search performance - See monitoring-guide.md metrics section
- [x] Document metrics relevant to graph executor latency - See Graph Executor Latency section
- [x] Create metric definitions for quality-weighted mode monitoring - See Feature Flag Status and Score Distribution
- [x] Document how to monitor for regressions - See Monitoring Checklist and Regression Detection
- [x] Provide alert threshold recommendations - See Recommended Alerts section with YAML examples

## Implementation

**Documentation Created:**
- `planning/monitoring-guide.md` - Comprehensive monitoring guide (~190 lines)

**Sections:**
1. Key Metrics to Monitor (latency, feature flags, score distribution, errors)
2. Prometheus queries with p50/p95/p99 thresholds
3. Recommended Alerts (YAML examples)
4. Monitoring Checklist (before/after enabling)
5. Log Monitoring guidance
6. Dashboard Recommendations (4 panels)
7. Validation Commands

## Dependencies

**Prerequisites:**
- SRCHREL-2003 (pipeline integrated)

**Blocks:**
- SRCHREL-3003 (rollout plan needs monitoring)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Phase 3)
