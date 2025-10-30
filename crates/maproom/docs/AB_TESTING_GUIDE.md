# A/B Testing Framework Guide

This guide explains how to use the A/B testing framework to validate hybrid search quality improvements before full production rollout.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Configuration](#configuration)
- [Running Experiments](#running-experiments)
- [Statistical Analysis](#statistical-analysis)
- [Quality Gates](#quality-gates)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The A/B testing framework enables data-driven decisions about search algorithm changes by:

1. **Shadow Mode**: Run new algorithms in parallel with production without impacting users
2. **Traffic Splitting**: Gradually roll out changes with percentage-based experiments
3. **Event Logging**: Capture shadow results and user interactions for analysis
4. **Statistical Validation**: Ensure improvements are statistically significant
5. **Quality Gates**: Automated validation before promotion to full rollout

## Quick Start

### 1. Setup Database

Run the migration to create A/B testing tables:

```bash
# Apply migration
psql -d maproom -f migrations/0007_ab_testing_schema.sql
```

### 2. Create an Experiment

```rust
use crewchief_maproom::ab_testing::*;

// Create experiment configuration
let config = ExperimentConfig::new(
    "hybrid-weights-v2".to_string(),
    25  // 25% traffic rollout
);

// Create experiment manager
let manager = ExperimentManager::new(db_pool.clone());
let experiment_id = manager.create_experiment(config).await?;
```

### 3. Run Shadow Mode

```rust
use crewchief_maproom::ab_testing::*;

async fn handle_search(query: String, user_id: Option<String>) -> Vec<SearchResult> {
    let shadow = ShadowMode::new();

    let results = shadow.execute(
        query.clone(),
        user_id.clone(),
        |q| old_search_implementation(q),
        |q| new_search_implementation(q),
    ).await?;

    // Log results for analysis
    let logger = ABTestLogger::new(db_pool);
    logger.log_shadow_results(experiment_id, &results).await?;

    // Return old results to user (no impact)
    results.old_results
}
```

### 4. Analyze Results

```rust
use crewchief_maproom::ab_testing::*;

let analyzer = StatisticalAnalyzer::new();

// Compare click-through rates
let test_result = analyzer.chi_square_test(
    old_clicks, old_total,
    new_clicks, new_total,
)?;

if test_result.is_significant {
    println!("Improvement is statistically significant (p = {})", test_result.p_value);
}

// Compare NDCG scores
let t_test = analyzer.t_test(&old_ndcg_scores, &new_ndcg_scores)?;
```

### 5. Validate Quality Gates

```rust
let passes = manager.validate_quality_gates(
    experiment_id,
    0.85,  // recall
    0.75,  // precision
    0.80,  // ndcg
    5,     // latency increase ms
    0.005, // error rate increase
    0.03,  // p-value
).await?;

if passes {
    // Promote to full rollout
    manager.update_rollout(experiment_id, 100).await?;
}
```

## Core Concepts

### Shadow Mode

Shadow mode executes both old and new search implementations in parallel:

- **Old implementation** results are returned to users (production path)
- **New implementation** runs asynchronously without blocking
- Both result sets are logged for comparison
- Timeout handling prevents slow experiments from impacting users
- Error isolation ensures new implementation failures don't affect users

**Performance Impact**: Shadow mode adds <10ms latency to search requests.

### Traffic Splitting

Traffic splitting routes a percentage of queries to the new implementation:

- **Deterministic**: Same user/query always gets the same implementation
- **Gradual Rollout**: Start at 5-25%, increase to 100% after validation
- **Instant Rollback**: Set rollout to 0% to disable experiment
- **Segmentation**: Use user_id for consistent user experience

### Experiment Lifecycle

Experiments follow a state machine:

```
Created (running) → Paused → Running → Completed
                              ↓
                           Failed
```

- **Running**: Experiment is active, collecting data
- **Paused**: Temporarily stopped, can be resumed
- **Completed**: Successfully finished, data retained
- **Failed**: Experiment failed quality gates or encountered errors

### Quality Gates

Quality gates ensure new implementations meet minimum standards before full rollout:

| Metric | Threshold | Description |
|--------|-----------|-------------|
| Recall@10 | >80% | Proportion of relevant results in top 10 |
| Precision@10 | >70% | Proportion of top 10 that are relevant |
| NDCG | >0.75 | Normalized discounted cumulative gain |
| Latency increase | <10ms | p95 latency delta vs baseline |
| Error rate increase | <1% | Error rate delta vs baseline |
| Statistical significance | p<0.05 | Results are not due to chance |

## Configuration

### Experiment Configuration

```rust
let mut config = ExperimentConfig::new("my-experiment".to_string(), 50);

// Optional: Set description
config.description = Some("Testing new fusion weights".to_string());

// Optional: Set end date
config.end_date = Some(Utc::now() + chrono::Duration::days(7));

// Optional: Customize quality gates
config.quality_gates = QualityGates {
    min_recall: 0.85,
    min_precision: 0.75,
    min_ndcg: 0.80,
    max_latency_increase_ms: 5,
    max_error_rate_increase: 0.005,
    significance_threshold: 0.01,
};

// Optional: Add metadata
config.metadata.insert("author".to_string(), json!("engineering-team"));
config.metadata.insert("ticket".to_string(), json!("HYBRID_SEARCH-5002"));
```

### Logger Configuration

```rust
// Default configuration (batch_size=100, flush every 10s)
let logger = ABTestLogger::new(db_pool);

// Custom configuration
let logger = ABTestLogger::with_config(
    db_pool,
    50,   // batch_size
    5,    // flush_interval_secs
);

// Start background flusher
let handle = Arc::new(logger).start_background_flusher();
```

### Shadow Mode Configuration

```rust
// Default timeout (5 seconds)
let shadow = ShadowMode::new();

// Custom timeout
let shadow = ShadowMode::with_timeout(3000); // 3 seconds
```

## Running Experiments

### Phase 1: Shadow Mode (0% rollout)

Run new implementation in shadow mode to collect data without user impact:

```rust
// Set rollout to 0 (shadow mode only)
manager.update_rollout(experiment_id, 0).await?;

// Run for 24-48 hours to collect sufficient data
// Monitor error rates, latency, and result quality
```

### Phase 2: Small Rollout (5-25%)

Route a small percentage of traffic to new implementation:

```rust
// Gradual rollout
manager.update_rollout(experiment_id, 5).await?;
// Monitor for 24 hours
manager.update_rollout(experiment_id, 25).await?;
// Monitor for 48 hours
```

### Phase 3: Medium Rollout (50%)

Expand to half of traffic:

```rust
manager.update_rollout(experiment_id, 50).await?;
// Monitor for 3-7 days to collect statistically significant data
```

### Phase 4: Full Rollout (100%)

After passing quality gates, roll out to all traffic:

```rust
if passes_quality_gates {
    manager.update_rollout(experiment_id, 100).await?;

    // Monitor for 1-2 weeks before completing experiment
    manager.complete_experiment(experiment_id).await?;
}
```

### Rollback

If issues are detected, immediately roll back:

```rust
// Emergency rollback
manager.update_rollout(experiment_id, 0).await?;

// Or pause experiment
manager.pause_experiment(experiment_id).await?;
```

## Statistical Analysis

### Chi-Square Test (Categorical Metrics)

Use for click-through rates, selection rates, abandon rates:

```rust
let analyzer = StatisticalAnalyzer::new();

let result = analyzer.chi_square_test(
    150, 1000,  // old: 150 clicks out of 1000 queries
    180, 1000,  // new: 180 clicks out of 1000 queries
)?;

println!("Chi-square: {}", result.statistic);
println!("P-value: {}", result.p_value);
println!("Significant: {}", result.is_significant);
```

### T-Test (Continuous Metrics)

Use for NDCG scores, latency, precision/recall:

```rust
let old_ndcg = vec![0.75, 0.76, 0.74, 0.77, 0.75];
let new_ndcg = vec![0.82, 0.83, 0.84, 0.82, 0.85];

let result = analyzer.t_test(&old_ndcg, &new_ndcg)?;

println!("T-statistic: {}", result.statistic);
println!("P-value: {}", result.p_value);
```

### Confidence Intervals

Calculate confidence intervals for proportions:

```rust
let ci = analyzer.proportion_confidence_interval(
    180, 1000,  // 180 successes out of 1000
    0.95,       // 95% confidence level
)?;

println!("Estimate: {} ({}% - {}%)",
    ci.estimate,
    ci.lower_bound * 100.0,
    ci.upper_bound * 100.0
);
```

Calculate confidence intervals for means:

```rust
let ci = analyzer.mean_confidence_interval(&ndcg_scores, 0.95)?;

println!("Mean NDCG: {} (95% CI: {} - {})",
    ci.estimate,
    ci.lower_bound,
    ci.upper_bound
);
```

### Sample Size Calculation

Determine required sample size before running experiment:

```rust
let sample_size = analyzer.calculate_sample_size(
    0.15,  // baseline rate (15% CTR)
    0.02,  // minimum detectable effect (2% absolute increase)
    0.80,  // power (80%)
    0.05,  // significance level (5%)
)?;

println!("Required sample size per group: {}", sample_size);
```

## Quality Gates

### Default Quality Gates

```rust
QualityGates {
    min_recall: 0.80,
    min_precision: 0.70,
    min_ndcg: 0.75,
    max_latency_increase_ms: 10,
    max_error_rate_increase: 0.01,
    significance_threshold: 0.05,
}
```

### Custom Quality Gates

```rust
let mut config = ExperimentConfig::new("strict-experiment".to_string(), 25);

config.quality_gates = QualityGates {
    min_recall: 0.85,           // Higher recall requirement
    min_precision: 0.80,        // Higher precision requirement
    min_ndcg: 0.80,            // Higher NDCG requirement
    max_latency_increase_ms: 5, // Stricter latency requirement
    max_error_rate_increase: 0.005, // Lower error tolerance
    significance_threshold: 0.01,   // More stringent significance
};
```

### Validation Example

```rust
let passes = manager.validate_quality_gates(
    experiment_id,
    recall_at_10,
    precision_at_10,
    ndcg_score,
    latency_increase_ms,
    error_rate_increase,
    p_value,
).await?;

if !passes {
    println!("Quality gates failed - rolling back");
    manager.update_rollout(experiment_id, 0).await?;
}
```

## Best Practices

### 1. Start Small

- Begin with shadow mode (0% rollout) for 24-48 hours
- Gradually increase rollout: 5% → 25% → 50% → 100%
- Monitor metrics at each stage before increasing

### 2. Calculate Sample Size

- Use `calculate_sample_size()` to determine required traffic
- Ensure experiment runs long enough to collect sufficient data
- Typical experiments need 1000-10000 queries per variant

### 3. Monitor Continuously

- Set up alerts for error rate spikes
- Track latency percentiles (p50, p95, p99)
- Review user interaction logs daily

### 4. Use Consistent User Assignment

- Same user/query should always get same implementation
- Prevents users from seeing inconsistent results
- Built into `TrafficSplitter` via stable hashing

### 5. Document Everything

- Record experiment rationale in `description` field
- Tag experiments with metadata (ticket ID, author)
- Keep notes on quality gate failures and rollbacks

### 6. Set Realistic Expectations

- Not all changes will improve metrics
- Small improvements (2-5%) are valuable
- Statistical significance requires patience

### 7. Clean Up Data

- Archive completed experiments after 90 days
- Implement retention policies for shadow_results and interaction_events
- Use database partitioning for high-volume production

## Troubleshooting

### Experiment Not Routing Traffic

**Symptom**: All queries use old implementation

**Solutions**:
- Check experiment status is "running"
- Verify rollout_percentage > 0
- Ensure start_date is in the past
- Check end_date hasn't passed

### High Error Rate in New Implementation

**Symptom**: new_error field populated frequently

**Solutions**:
- Review error logs in shadow_results table
- Check timeout configuration (increase if needed)
- Fix bugs in new implementation
- Roll back to 0% while debugging

### Insufficient Statistical Power

**Symptom**: P-values remain high despite apparent differences

**Solutions**:
- Calculate required sample size upfront
- Run experiment longer to collect more data
- Increase rollout percentage (more users = more data)
- Consider effect size may be too small to detect

### Database Performance Issues

**Symptom**: Slow queries on shadow_results or interaction_events

**Solutions**:
- Verify indexes are created (see migration file)
- Implement time-based partitioning
- Increase batch_size in logger configuration
- Archive old experiment data

### Shadow Mode Adding Too Much Latency

**Symptom**: Search requests slower than expected

**Solutions**:
- Reduce shadow mode timeout (default 5s)
- Optimize new implementation
- Use sampling (log only 10% of shadow results)
- Increase async worker threads

## Additional Resources

- [Statistical Analysis Module](/src/ab_testing/analysis.rs)
- [Shadow Mode Implementation](/src/ab_testing/shadow_mode.rs)
- [Database Schema](/migrations/0007_ab_testing_schema.sql)
- [Integration Tests](/tests/ab_testing_test.rs)

## Support

For questions or issues:
1. Review this guide and code documentation
2. Check integration tests for usage examples
3. Consult statistical analysis literature for methodology questions
4. File issues with detailed reproduction steps
