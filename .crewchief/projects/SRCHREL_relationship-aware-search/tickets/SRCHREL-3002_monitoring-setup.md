# Ticket: SRCHREL-3002 - Monitoring and Metrics Setup

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- ops-engineer
- verify-ticket
- commit-ticket

## Summary

Set up monitoring dashboards and alerts for quality-weighted graph scoring. Track latency, score distributions, and feature flag usage to enable safe rollout and quick rollback if needed.

## Acceptance Criteria

- [ ] Add metrics for graph executor latency (p50, p95, p99)
- [ ] Add metrics for quality scoring mode (legacy vs enhanced)
- [ ] Track score distributions (production vs test code scores)
- [ ] Create Prometheus alert rules for performance degradation
- [ ] Create alert for feature flag unexpectedly disabled
- [ ] Build Grafana dashboard (or equivalent) for visualization
- [ ] Document metric meanings and alert thresholds
- [ ] Test alert firing conditions

## Technical Requirements

**Metrics to Collect:**

```rust
// Using prometheus crate
use prometheus::{histogram_vec, register_histogram_vec, IntCounterVec, register_int_counter_vec};

lazy_static! {
    static ref GRAPH_EXECUTOR_LATENCY: HistogramVec = register_histogram_vec!(
        "graph_executor_latency_seconds",
        "Graph executor latency in seconds",
        &["mode"], // "legacy" or "quality_weighted"
        vec![0.005, 0.01, 0.02, 0.03, 0.05, 0.1, 0.2]
    ).unwrap();

    static ref GRAPH_EXECUTOR_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "graph_executor_requests_total",
        "Total graph executor requests",
        &["mode"]
    ).unwrap();
}

// In graph executor
let timer = GRAPH_EXECUTOR_LATENCY
    .with_label_values(&[if enable_quality { "quality_weighted" } else { "legacy" }])
    .start_timer();

let results = execute_query(...);

timer.observe_duration();
GRAPH_EXECUTOR_REQUESTS
    .with_label_values(&[if enable_quality { "quality_weighted" } else { "legacy" }])
    .inc();
```

**Alert Rules:**

```yaml
# Prometheus alert rules
groups:
  - name: srchrel_quality_scoring
    rules:
      - alert: GraphExecutorSlowWarning
        expr: histogram_quantile(0.95, graph_executor_latency_seconds{mode="quality_weighted"}) > 0.035
        for: 5m
        severity: warning
        annotations:
          summary: "Graph executor p95 latency exceeds 35ms"
          description: "Current: {{ $value }}s, investigate query performance"

      - alert: GraphExecutorSlowCritical
        expr: histogram_quantile(0.95, graph_executor_latency_seconds{mode="quality_weighted"}) > 0.040
        for: 10m
        severity: critical
        annotations:
          summary: "Graph executor p95 latency exceeds 40ms - CONSIDER ROLLBACK"

      - alert: QualityScoringDisabled
        expr: |
          rate(graph_executor_requests_total{mode="quality_weighted"}[5m])
          / rate(graph_executor_requests_total[5m]) < 0.01
        for: 10m
        severity: warning
        annotations:
          summary: "Quality scoring not being used (<1% of requests)"
```

## Dependencies

**Prerequisites:**
- SRCHREL-2003 (pipeline integrated, can instrument)

**Blocks:**
- SRCHREL-3003 (rollout plan depends on monitoring)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Phase 3, line 344)
- Quality Strategy: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/quality-strategy.md` (Monitoring, lines 335-446)
