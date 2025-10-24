# Maproom Hybrid Search Monitoring Guide

This guide provides comprehensive instructions for setting up, operating, and interpreting monitoring for the Maproom hybrid search system.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Setup Instructions](#setup-instructions)
4. [Metrics Reference](#metrics-reference)
5. [Dashboard Usage](#dashboard-usage)
6. [Alert Configuration](#alert-configuration)
7. [Operational Procedures](#operational-procedures)
8. [Troubleshooting](#troubleshooting)

## Overview

The Maproom hybrid search monitoring system provides comprehensive observability through:

- **Prometheus metrics** for time-series data collection
- **Grafana dashboards** for visualization
- **Alertmanager** for automated alerting
- **Structured logging** with tracing for debugging

### Key Metrics Tracked

- **Latency**: Query latency percentiles (p50, p95, p99)
- **Throughput**: Query rate and volume
- **Errors**: Error rates by type
- **Cache**: Cache hit rate and effectiveness
- **Quality**: Result counts and distribution
- **Fusion**: Score fusion performance

### Performance Targets

- **p95 latency**: < 50ms (warning at >50ms, critical at >100ms)
- **Error rate**: < 1% (warning at >1%, critical at >5%)
- **Cache hit rate**: > 50% (warning at <50%)
- **Fusion time (p95)**: < 10ms

## Architecture

```
┌─────────────────┐
│  Search System  │
│                 │
│  ┌───────────┐  │
│  │ Pipeline  │  │──┐
│  └───────────┘  │  │
│                 │  │
│  ┌───────────┐  │  │  Metrics
│  │  Metrics  │◄─┘  │  Recording
│  │ Collector │     │
│  └─────┬─────┘     │
└────────┼───────────┘
         │
         │ HTTP /metrics
         ▼
┌─────────────────┐
│   Prometheus    │
│   (Scraper)     │
└────────┬────────┘
         │
         │ PromQL
         ▼
┌─────────────────┐      ┌──────────────┐
│    Grafana      │      │ Alertmanager │
│  (Dashboards)   │      │ (Alerts)     │
└─────────────────┘      └──────────────┘
```

## Setup Instructions

### 1. Start Metrics Server

The metrics server exposes Prometheus metrics on port 9090 (configurable).

```rust
use crewchief_maproom::metrics::init_metrics_server;

#[tokio::main]
async fn main() {
    // Start metrics server in background
    tokio::spawn(async {
        if let Err(e) = init_metrics_server("0.0.0.0:9090").await {
            eprintln!("Metrics server error: {}", e);
        }
    });

    // ... rest of application ...
}
```

Verify metrics endpoint:
```bash
curl http://localhost:9090/metrics
```

### 2. Configure Prometheus

Create or update `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

# Alert rule files
rule_files:
  - "search_alerts.yml"

# Scrape configurations
scrape_configs:
  - job_name: 'maproom-search'
    static_configs:
      - targets: ['localhost:9090']
        labels:
          service: 'maproom'
          component: 'search'
```

Start Prometheus:
```bash
prometheus --config.file=prometheus.yml
```

Access Prometheus UI: `http://localhost:9090`

### 3. Import Grafana Dashboard

1. Open Grafana (default: `http://localhost:3000`)
2. Navigate to **Dashboards** → **Import**
3. Upload `config/grafana/search_dashboard.json`
4. Select Prometheus data source
5. Click **Import**

The dashboard will be available at: `http://localhost:3000/d/maproom-search`

### 4. Configure Alertmanager

Create `alertmanager.yml`:

```yaml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'severity']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'team-notifications'

receivers:
  - name: 'team-notifications'
    email_configs:
      - to: 'team@example.com'
        from: 'alertmanager@example.com'
        smarthost: 'smtp.example.com:587'
        auth_username: 'alerts'
        auth_password: 'password'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
        channel: '#alerts'
        title: '{{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

inhibit_rules:
  - source_match:
      severity: 'critical'
    target_match:
      severity: 'warning'
    equal: ['alertname']
```

Start Alertmanager:
```bash
alertmanager --config.file=alertmanager.yml
```

### 5. Enable Structured Logging

Configure tracing subscriber in your application:

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,crewchief_maproom=debug"))
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
}
```

Set log level via environment:
```bash
# Production (INFO level)
RUST_LOG=info,crewchief_maproom::search=debug

# Development (DEBUG level)
RUST_LOG=debug

# Verbose debugging (TRACE level)
RUST_LOG=trace
```

## Metrics Reference

### Query Latency Metrics

**Metric**: `maproom_search_query_latency_seconds`
**Type**: Histogram
**Labels**: `mode` (code/text/auto), `status` (success/error)

**Description**: Tracks end-to-end search query latency from request to response.

**Buckets**: 1ms, 5ms, 10ms, 25ms, 50ms, 75ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s

**PromQL Queries**:
```promql
# p95 latency by mode
histogram_quantile(0.95, rate(maproom_search_query_latency_seconds_bucket{status="success"}[5m]))

# Average latency
rate(maproom_search_query_latency_seconds_sum[5m]) / rate(maproom_search_query_latency_seconds_count[5m])

# Query count
rate(maproom_search_query_latency_seconds_count[5m])
```

### Fusion Time Metrics

**Metric**: `maproom_search_fusion_time_seconds`
**Type**: Histogram
**Labels**: `strategy` (basic_weighted/rrf/custom)

**Description**: Tracks score fusion computation time.

**Buckets**: 0.5ms, 1ms, 2ms, 5ms, 10ms, 25ms, 50ms

**PromQL Queries**:
```promql
# p95 fusion time
histogram_quantile(0.95, rate(maproom_search_fusion_time_seconds_bucket[5m]))
```

### Cache Hit Rate Metrics

**Metric**: `maproom_search_cache_hit_rate`
**Type**: Gauge
**Range**: 0.0 - 1.0

**Description**: Cache effectiveness as ratio of hits to total requests.

**PromQL Queries**:
```promql
# Current hit rate
maproom_search_cache_hit_rate

# Average hit rate over 1 hour
avg_over_time(maproom_search_cache_hit_rate[1h])
```

### Result Count Metrics

**Metric**: `maproom_search_result_count`
**Type**: Histogram
**Labels**: `mode` (code/text/auto)

**Description**: Number of results returned per query.

**Buckets**: 0, 1, 5, 10, 20, 50, 100

**PromQL Queries**:
```promql
# Median result count
histogram_quantile(0.50, rate(maproom_search_result_count_bucket[5m]))
```

### Error Metrics

**Metric**: `maproom_search_errors_total`
**Type**: Counter
**Labels**: `error_type` (query_processing/search_execution/fusion/database)

**Description**: Total number of errors by type.

**PromQL Queries**:
```promql
# Error rate
rate(maproom_search_errors_total[5m])

# Error ratio
rate(maproom_search_errors_total[5m]) / rate(maproom_search_queries_total[5m])

# Errors by type
sum by (error_type) (rate(maproom_search_errors_total[5m]))
```

### Query Counter

**Metric**: `maproom_search_queries_total`
**Type**: Counter
**Labels**: `mode` (code/text/auto), `status` (success/error)

**Description**: Total number of queries executed.

**PromQL Queries**:
```promql
# Query rate
rate(maproom_search_queries_total[5m])

# Success rate
rate(maproom_search_queries_total{status="success"}[5m])
```

## Dashboard Usage

### Panel Overview

The Grafana dashboard includes the following panels:

#### 1. Search Query Latency (p50, p95, p99)
- **Purpose**: Monitor query performance
- **Threshold**: p95 should be < 50ms
- **Action**: Investigate if p95 > 50ms consistently

#### 2. Cache Hit Rate
- **Purpose**: Track cache effectiveness
- **Threshold**: Should be > 50%
- **Action**: Review cache configuration if < 50%

#### 3. Query Rate
- **Purpose**: Monitor search traffic
- **Action**: Investigate sudden drops or spikes

#### 4. Error Rate by Type
- **Purpose**: Identify reliability issues
- **Threshold**: Should be < 1%
- **Action**: Check logs for error details

#### 5. Result Count Distribution
- **Purpose**: Understand result quality
- **Action**: Investigate if median < 1 (indexing issues)

#### 6. Fusion Time
- **Purpose**: Monitor fusion performance
- **Threshold**: p95 should be < 10ms
- **Action**: Optimize fusion if > 10ms

#### 7. Errors (Last Hour)
- **Purpose**: Quick error overview
- **Action**: Drill down into specific error types

### Common Workflows

#### Investigating High Latency

1. Check **Query Latency** panel for affected mode (code/text/auto)
2. Compare with **Fusion Time** to isolate bottleneck
3. Review **Cache Hit Rate** - low hit rate increases latency
4. Check logs for specific slow queries:
   ```bash
   grep "Search exceeded 50ms" logs/search.log
   ```

#### Investigating Errors

1. Check **Error Rate by Type** panel
2. Identify spike in specific error type
3. Check **Errors (Last Hour)** for counts
4. Review logs for error details:
   ```bash
   grep "ERROR" logs/search.log | grep "query_processing"
   ```

#### Performance Optimization

1. Monitor **Cache Hit Rate** - optimize for > 70%
2. Check **Fusion Time** - should be < 5ms at p95
3. Review **Result Count** - ensure indexing is working
4. Monitor **Query Latency** trends over time

## Alert Configuration

### Alert Severity Levels

- **Critical**: Immediate attention required, service degraded
- **Warning**: Attention needed, potential issue developing
- **Info**: Informational, no immediate action needed

### Alert Response Times

- **Critical**: < 15 minutes
- **Warning**: < 1 hour
- **Info**: Review during business hours

### Alert Tuning

If experiencing alert fatigue:

1. **Increase thresholds** gradually
2. **Extend for duration** (e.g., `for: 10m` instead of `5m`)
3. **Add inhibit rules** in Alertmanager
4. **Review in staging** before production

Example threshold adjustment in `search_alerts.yml`:
```yaml
# Before: too sensitive
expr: histogram_quantile(0.95, ...) > 0.050

# After: more lenient
expr: histogram_quantile(0.95, ...) > 0.075
```

## Operational Procedures

### Daily Operations

1. **Morning Check** (5 minutes):
   - Review Grafana dashboard
   - Check for active alerts
   - Verify metrics are being collected

2. **Weekly Review** (30 minutes):
   - Analyze latency trends
   - Review error patterns
   - Check cache effectiveness
   - Plan optimizations

3. **Monthly Review** (2 hours):
   - Capacity planning based on growth
   - Alert threshold tuning
   - Performance optimization planning
   - Documentation updates

### Incident Response

1. **Acknowledge Alert**: Acknowledge in Alertmanager
2. **Assess Impact**: Check dashboard for scope
3. **Review Logs**: Check structured logs for details
4. **Mitigate**: Apply fixes based on runbook
5. **Document**: Record incident details
6. **Post-mortem**: Analyze root cause

### Capacity Planning

Monitor these metrics for growth trends:

- **Query rate**: Plan for 2x growth buffer
- **Latency p95**: Should stay < 50ms under load
- **Cache size**: Adjust based on hit rate
- **Database connections**: Scale with query volume

### Maintenance Windows

Before maintenance:
1. Silence alerts in Alertmanager
2. Notify stakeholders
3. Monitor dashboards during maintenance
4. Un-silence alerts after completion

## Troubleshooting

### Metrics Not Appearing

**Symptom**: Grafana shows "No data"

**Diagnosis**:
1. Check metrics endpoint: `curl http://localhost:9090/metrics`
2. Verify Prometheus is scraping: Check Prometheus UI → Targets
3. Check Grafana data source configuration

**Solution**:
- Restart metrics server
- Verify Prometheus configuration
- Check firewall/network connectivity

### High Memory Usage

**Symptom**: Metrics server using excessive memory

**Diagnosis**:
- Check metric cardinality (number of unique label combinations)
- Monitor histogram bucket count

**Solution**:
- Reduce label cardinality
- Adjust histogram buckets
- Consider metric sampling for high-volume scenarios

### Alert Fatigue

**Symptom**: Too many alerts firing

**Diagnosis**:
- Review alert frequency
- Check threshold appropriateness
- Analyze false positive rate

**Solution**:
- Tune alert thresholds
- Increase `for:` duration
- Add alert inhibition rules
- Consolidate related alerts

### Cache Hit Rate Low

**Symptom**: Cache hit rate < 30%

**Diagnosis**:
- High query diversity
- Cache size too small
- TTL too short

**Solution**:
- Increase cache size
- Extend TTL
- Analyze query patterns
- Consider query normalization

## Best Practices

1. **Monitor the Monitors**: Set up uptime checks for Prometheus and Grafana
2. **Regular Reviews**: Schedule weekly metric reviews
3. **Document Changes**: Track threshold and configuration changes
4. **Test Alerts**: Regularly test alert routing and notifications
5. **Continuous Improvement**: Use metrics to drive optimization
6. **Baseline Establishment**: Establish performance baselines in staging
7. **Gradual Rollouts**: Monitor metrics during feature rollouts
8. **Correlation**: Correlate metrics with application logs

## Additional Resources

- **Runbook**: See `runbook.md` for troubleshooting procedures
- **Architecture**: See `HYBRID_SEARCH_ARCHITECTURE.md` for system design
- **Prometheus Docs**: https://prometheus.io/docs/
- **Grafana Docs**: https://grafana.com/docs/
- **PromQL Guide**: https://prometheus.io/docs/prometheus/latest/querying/basics/
