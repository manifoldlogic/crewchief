# Quality-Weighted Graph Scoring Monitoring Guide

**Ticket:** SRCHREL-3002
**Feature:** Relationship-Aware Search Ranking (SRCHREL)

---

## Overview

This guide documents monitoring recommendations for quality-weighted graph scoring. Proper monitoring ensures the feature performs as expected and helps identify regressions early.

---

## Key Metrics to Monitor

### 1. Graph Executor Latency

**Existing Prometheus metrics in maproom:**

```promql
# Histogram: Search duration by component
maproom_search_duration_seconds{component="graph_executor"}

# P95 latency
histogram_quantile(0.95, sum(rate(maproom_search_duration_seconds_bucket{component="graph_executor"}[5m])) by (le))
```

**Alert Thresholds:**

| Metric | Warning | Critical |
|--------|---------|----------|
| Graph executor p50 | >25ms | >30ms |
| Graph executor p95 | >35ms | >40ms |
| Graph executor p99 | >60ms | >80ms |

### 2. Feature Flag Status

Track whether quality-weighted scoring is enabled:

```promql
# Gauge: Feature flag status (0 or 1)
maproom_feature_flags{flag="enable_quality_weighted_graph"}
```

### 3. Score Distribution

Monitor score distribution shifts after enabling quality scoring:

```promql
# Histogram: Graph importance scores
maproom_graph_scores_distribution_bucket

# Mean graph score over time
sum(rate(maproom_graph_scores_sum[5m])) / sum(rate(maproom_graph_scores_count[5m]))
```

### 4. Error Rates

```promql
# Counter: Search errors by component
maproom_search_errors_total{component="graph_executor"}

# Error rate percentage
rate(maproom_search_errors_total{component="graph_executor"}[5m])
  / rate(maproom_search_requests_total[5m]) * 100
```

---

## Recommended Alerts

### Latency Degradation

```yaml
alert: GraphExecutorLatencyHigh
expr: histogram_quantile(0.95, sum(rate(maproom_search_duration_seconds_bucket{component="graph_executor"}[5m])) by (le)) > 0.035
for: 5m
labels:
  severity: warning
annotations:
  summary: Graph executor p95 latency exceeds 35ms
  description: Graph executor latency is {{ $value | humanizeDuration }}
```

### Error Rate Spike

```yaml
alert: GraphExecutorErrorRateHigh
expr: rate(maproom_search_errors_total{component="graph_executor"}[5m]) / rate(maproom_search_requests_total[5m]) > 0.01
for: 5m
labels:
  severity: warning
annotations:
  summary: Graph executor error rate above 1%
  description: Error rate is {{ $value | humanizePercentage }}
```

---

## Monitoring Checklist

### Before Enabling Quality Scoring

- [ ] Baseline graph executor p95 latency recorded
- [ ] Baseline error rate recorded
- [ ] Baseline score distribution captured
- [ ] Alerts configured for latency and errors

### After Enabling Quality Scoring

- [ ] Compare p95 latency to baseline (expect <10% increase)
- [ ] Verify error rate unchanged
- [ ] Observe score distribution shift (expected)
- [ ] Monitor user feedback on search relevance

### Regression Detection

If latency increases >10% after enabling:
1. Check `fusion_weight_override` value
2. Verify database indexes exist
3. Consider disabling via feature flag
4. Review edge count growth

---

## Log Monitoring

Enable debug logging to see quality scoring in action:

```bash
RUST_LOG=crewchief_maproom::search::graph=debug cargo run ...
```

Look for log messages:
- "Executing graph importance query" with `enable_quality=true/false`
- Production/test code weights being applied
- Query execution times

---

## Dashboard Recommendations

### Panel 1: Latency Overview
- Graph: p50, p95, p99 latency over time
- Compare to baseline (dashed line)
- Annotation: when quality scoring enabled

### Panel 2: Feature Flag Status
- Single stat: enabled/disabled
- Color: green=disabled (safe), yellow=enabled

### Panel 3: Score Distribution
- Heatmap: score distribution over time
- Compare before/after enabling

### Panel 4: Request Rate & Errors
- Stacked area: total requests vs errors
- Error rate percentage

---

## Validation Commands

### Check Current Configuration

```bash
# View active feature flags
cargo run --bin crewchief-maproom -- status --repo crewchief

# Test search with debug output
cargo run --bin crewchief-maproom -- search \
  --repo crewchief \
  --query "handler" \
  --debug
```

### Benchmark Performance

```bash
# Run graph quality benchmarks
cargo bench --bench graph_quality_benchmark

# Compare with baseline
cargo bench --bench graph_quality_benchmark -- --save-baseline current
```

---

**Document Version:** 1.0
**Author:** infrastructure-engineer agent
**Last Updated:** 2025-12-15
