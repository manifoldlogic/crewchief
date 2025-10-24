# Ticket: HYBRID_SEARCH-5002: A/B Testing Framework

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- search-quality-engineer
- integration-tester
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement a comprehensive A/B testing framework to validate hybrid search quality improvements in production. The framework enables shadow mode testing, user interaction logging, comparison dashboards, and statistical analysis to ensure new search algorithms meet quality thresholds before full rollout.

## Background
As hybrid search evolves from MVP to production-ready, we need rigorous validation that changes improve search quality without regressions. A/B testing allows us to:
1. Run new search algorithms in shadow mode (parallel to production) without impacting users
2. Collect real-world usage metrics (clicks, dwell time, result selection)
3. Compare quality metrics between old and new implementations
4. Make data-driven decisions about feature rollouts with statistical confidence
5. Gradually roll out changes with percentage-based experiments

This ticket implements the infrastructure from Phase 5, Week 5, Task 2 of the HYBRID_SEARCH plan.

## Acceptance Criteria
- [x] Shadow mode runs both old and new search in parallel, returns old results to users, logs both result sets
- [x] Comparison dashboard displays side-by-side quality metrics (recall, precision, NDCG)
- [x] User interaction logging captures: clicks, dwell time, result selection, query abandonment
- [x] A/B test configuration supports percentage rollout (e.g., 5%, 25%, 50% of traffic)
- [x] Statistical significance testing validates quality differences with confidence intervals
- [x] Experiment tracking system maintains history of tests and outcomes
- [x] Framework achieves target quality metrics: Recall >80%, Precision >70% at k=10, NDCG >0.75
- [x] Integration tests verify shadow mode correctness and logging accuracy

## Technical Requirements
- **Shadow Mode Infrastructure**: Execute both search implementations in parallel without blocking user response
- **Result Logging**: Persist both result sets with metadata (timestamps, query, user ID, experiment ID)
- **Async Execution**: Non-blocking parallel search execution using Tokio async runtime
- **User Interaction Events**: Capture click position, dwell time, result opened, query reformulation
- **Dashboard Backend**: Query aggregated metrics by experiment, time window, user segment
- **Statistical Analysis**: Implement chi-square tests, t-tests, confidence intervals for quality metrics
- **Configuration Management**: Experiment definitions with rollout percentages, start/end dates, quality gates
- **Performance**: Shadow mode adds <10ms latency to search requests
- **Data Retention**: Configurable retention policies for experiment logs (default 90 days)

## Implementation Notes

### Shadow Mode Architecture
```rust
// Pseudocode structure
async fn handle_search_request(query: Query, config: ExperimentConfig) -> SearchResults {
    let old_results_future = run_old_search(query.clone());
    let new_results_future = run_new_search(query.clone());

    // Return old results immediately, log new results async
    let old_results = old_results_future.await?;
    tokio::spawn(async move {
        if let Ok(new_results) = new_results_future.await {
            log_shadow_results(query, old_results, new_results, config).await;
        }
    });

    Ok(old_results)
}
```

### Key Components
1. **framework.rs**: Core A/B testing orchestration
   - Experiment configuration and routing
   - Traffic splitting logic (percentage-based)
   - Experiment lifecycle management (start, pause, stop, analyze)

2. **shadow_mode.rs**: Parallel execution engine
   - Dual search execution (old + new algorithms)
   - Timeout handling for slow experiments
   - Error isolation (new search failures don't affect users)

3. **logger.rs**: Event logging infrastructure
   - Result set logging with structured metadata
   - User interaction event capture
   - Batch writing for performance
   - Integration with PostgreSQL for persistence

4. **dashboard.rs**: Metrics aggregation and API
   - Real-time quality metric calculations
   - Time-series data for trend analysis
   - User segment breakdowns
   - Export functionality for detailed analysis

5. **analysis.rs**: Statistical validation
   - Chi-square tests for click-through rate differences
   - T-tests for continuous metrics (NDCG, latency)
   - Confidence intervals (95%, 99%)
   - Sample size calculations
   - Early stopping criteria for strong signals

### Database Schema Additions
```sql
-- Experiment definitions
CREATE TABLE experiments (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    rollout_percentage INTEGER NOT NULL,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ,
    status TEXT NOT NULL, -- 'running', 'paused', 'completed'
    config JSONB NOT NULL
);

-- Shadow mode result logs
CREATE TABLE shadow_results (
    id UUID PRIMARY KEY,
    experiment_id UUID REFERENCES experiments(id),
    query TEXT NOT NULL,
    old_results JSONB NOT NULL,
    new_results JSONB NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    user_id TEXT,
    latency_ms INTEGER
);

-- User interaction events
CREATE TABLE interaction_events (
    id UUID PRIMARY KEY,
    experiment_id UUID REFERENCES experiments(id),
    query TEXT NOT NULL,
    event_type TEXT NOT NULL, -- 'click', 'dwell', 'selection', 'abandon'
    result_position INTEGER,
    dwell_time_ms INTEGER,
    timestamp TIMESTAMPTZ NOT NULL,
    user_id TEXT
);
```

### Integration with Existing Search
- Hook into existing search endpoint in `crates/maproom/src/search/`
- Use feature flags to enable/disable A/B testing
- Minimal changes to existing search logic (decorator pattern)
- Backward compatible with non-experiment requests

### Quality Gates
Before rolling out to 100% of traffic:
- Statistical significance at p<0.05 for primary metrics
- Recall improvement >5% over baseline OR Recall >80%
- Precision at k=10 improvement >3% over baseline OR Precision >70%
- NDCG improvement >2% over baseline OR NDCG >0.75
- No increase in search latency (p95 < baseline + 10ms)
- No increase in error rate

## Dependencies
- **HYBRID_SEARCH-5001**: Golden test set for evaluation (provides ground truth for quality metrics)
- Existing hybrid search implementation (HYBRID_SEARCH-2003, HYBRID_SEARCH-3001, HYBRID_SEARCH-3002)
- PostgreSQL database schema from Phase 1

## Risk Assessment
- **Risk**: Shadow mode doubles search infrastructure load
  - **Mitigation**: Start with low rollout percentages (5%), monitor resource usage, implement circuit breakers, use sampling for high-traffic scenarios

- **Risk**: Logging infrastructure becomes bottleneck under high traffic
  - **Mitigation**: Async batch writes, implement sampling (log 10% of requests for quality analysis), use separate database/schema for logs, implement log rotation

- **Risk**: Statistical analysis may require large sample sizes for significance
  - **Mitigation**: Calculate required sample sizes upfront, extend test duration if needed, use Bayesian methods for early signals, prioritize high-impact changes

- **Risk**: Dashboard query performance degrades with large log volumes
  - **Mitigation**: Pre-aggregate metrics hourly/daily, use materialized views, implement time-based partitioning, add appropriate indexes

- **Risk**: Experiment configuration errors could impact user experience
  - **Mitigation**: Dry-run mode for configuration validation, gradual rollout with automated rollback, monitoring alerts for error rate spikes, shadow mode isolation prevents user impact

## Files/Packages Affected
- **New Files**:
  - `crates/maproom/src/ab_testing/framework.rs` - A/B testing orchestration
  - `crates/maproom/src/ab_testing/shadow_mode.rs` - Parallel search execution
  - `crates/maproom/src/ab_testing/logger.rs` - Event logging infrastructure
  - `crates/maproom/src/ab_testing/dashboard.rs` - Metrics API and aggregation
  - `crates/maproom/src/ab_testing/analysis.rs` - Statistical analysis tools
  - `crates/maproom/src/ab_testing/mod.rs` - Module definitions
  - `crates/maproom/migrations/0XX_ab_testing_schema.sql` - Database schema

- **Modified Files**:
  - `crates/maproom/src/search/mod.rs` - Integration hooks for A/B framework
  - `crates/maproom/src/search/hybrid.rs` - Experiment routing logic
  - `crates/maproom/src/lib.rs` - Module registration
  - `crates/maproom/Cargo.toml` - Dependencies (statistical libraries, async logging)

- **Test Files**:
  - `crates/maproom/tests/ab_testing_integration_test.rs` - End-to-end A/B testing validation
  - `crates/maproom/tests/shadow_mode_test.rs` - Shadow mode correctness tests
  - `crates/maproom/tests/statistical_analysis_test.rs` - Statistical method validation
