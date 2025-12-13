# SRCHTRN-1000: Performance Baseline Measurement

## Title
Measure current search performance baseline for Phase 2 comparison

## Status
- [ ] **Implementation Complete**
- [ ] **Tests Passing**
- [ ] **Verified**
- [ ] **Committed**

## Agents
- **Primary**: general
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Measure and document current search latency metrics (p50, p95, p99) and query processing time breakdown before implementing any code changes. This baseline will be used in Phase 2 to validate that metadata assembly adds <10ms overhead.

## Background
The architecture claims <10ms overhead for query understanding metadata assembly. To validate this claim, we need baseline performance measurements before making any changes. These metrics will serve as the comparison point for Phase 2 regression testing.

**Key Requirement**: This is a measurement-only task with zero code changes. Measure, document, and commit the baseline for future comparison.

## Acceptance Criteria
- [ ] p50, p95, p99 search latency measured and documented
- [ ] Query processing time breakdown recorded (existing instrumentation)
- [ ] Baseline measurements saved in `/workspace/.crewchief/projects/SRCHTRN_search-transparency/planning/performance-baseline.md`
- [ ] Test workload documented (query count, types of searches)
- [ ] Measurement methodology documented (how metrics were collected)
- [ ] Prometheus metrics query commands documented for reproducibility
- [ ] No code changes made (measurement only)

## Technical Requirements

### Metrics to Collect
1. **Latency Distribution**:
   - p50 (median) latency
   - p95 latency (target: <100ms in final implementation)
   - p99 latency (outlier detection)

2. **Query Processing Breakdown**:
   - Query tokenization time
   - Embedding generation time (if applicable)
   - Search execution time
   - Result assembly time
   - Total end-to-end time

3. **Test Workload**:
   - 100 representative queries
   - Mix of search modes (auto, code, text)
   - Mix of query lengths (short, medium, long)
   - Real indexed repository

### Measurement Approach
- Use existing Prometheus metrics (`maproom_search_duration_seconds`)
- Run daemon with instrumentation enabled
- Execute standard search workload via MCP client
- Collect metrics from Prometheus dashboard or API

### Documentation Format
```markdown
# Performance Baseline - SRCHTRN Project

**Date**: YYYY-MM-DD
**Measured By**: [agent/user]
**Commit**: [git SHA]

## Test Setup
- Repository: crewchief (or specify test repo)
- Indexed chunks: [count]
- Embedding provider: [OpenAI/Ollama/etc]
- Query count: 100

## Latency Metrics
- p50: XX.Xms
- p95: XX.Xms
- p99: XXX.Xms

## Query Processing Breakdown
- Tokenization: X.Xms
- Embedding: XX.Xms
- Search execution: XX.Xms
- Result assembly: X.Xms
- Total: XX.Xms

## Prometheus Queries Used
```promql
histogram_quantile(0.50, maproom_search_duration_seconds)
histogram_quantile(0.95, maproom_search_duration_seconds)
histogram_quantile(0.99, maproom_search_duration_seconds)
```

## Test Workload
[List of representative queries or link to test script]

## Notes
[Any observations or anomalies]
```

## Implementation Notes
1. Check if Prometheus metrics are already instrumented
2. If not, use timing logs from daemon output
3. Run searches via `npx @crewchief/maproom-mcp`
4. Calculate percentiles from collected timings
5. Document everything for Phase 2 comparison

## Dependencies
None (first task in Phase 1)

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Metrics may not be instrumented yet
- Prometheus may not be running

**Mitigations**:
- Fallback to parsing daemon logs for timings
- Can manually calculate percentiles from log data
- Worst case: use representative sample of 10-20 queries

## Files/Packages Affected
- **New file**: `.crewchief/projects/SRCHTRN_search-transparency/planning/performance-baseline.md`
- **No code changes** (measurement only)

## Estimated Effort
2-4 hours

## Planning References
- [plan.md](../planning/plan.md) - Phase 1 baseline requirement
- [architecture.md](../planning/architecture.md) - Performance budget details
- [quality-strategy.md](../planning/quality-strategy.md) - Performance validation approach
