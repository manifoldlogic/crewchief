# Ticket: SEMRANK-1005: Baseline Search Quality Metrics

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary
Document current search behavior (what ranks #1 for known queries), measure latency baselines (p50, p95, p99), create golden dataset of queries with expected results.

## Background
Phase 1 requires baseline metrics to measure improvement after implementing semantic ranking in Phase 3. The golden dataset created here will be used in SEMRANK-3005 performance benchmarks to validate that:
1. Search quality improves (implementations rank above tests/docs)
2. Latency remains acceptable (within 10% of baseline)

Baseline behavior was already documented in SEMRANK-0002, but now we need formal metrics with reproducible benchmarks. This ticket implements the measurement infrastructure portion of the Phase 1 plan.

## Acceptance Criteria
- [ ] Golden query set defined: 20 representative queries across languages (Rust, TypeScript, Python)
- [ ] Latency baselines measured: p50, p95, p99 over 100 runs per query
- [ ] Baseline format documented in CSV: query, latency_p50_ms, latency_p95_ms, top_3_kinds, implementation_rank
- [ ] Benchmark script created for reproducibility (automated execution, no manual steps)
- [ ] Current ranking behavior documented: Examples where tests rank above implementations
- [ ] Database query plans logged (EXPLAIN ANALYZE) for baseline queries

## Technical Requirements
- **Golden Query Set** (20 queries):
  - Mix of exact function names: "authenticate", "validate_token", "connect_db"
  - Concept searches: "HTTP handler", "database connection", "user authentication"
  - Acronym tests: "XMLParser", "HTTPSHandler"
  - React hooks: "useAuth", "useState"
- **Measurement Protocol**:
  - Run each query 100 times
  - Record p50, p95, p99 latencies
  - Measure on warm database (after 10 warm-up queries)
  - Use consistent hardware/environment
- **CSV Format**:
  ```csv
  query,latency_p50_ms,latency_p95_ms,top_3_kinds,implementation_rank
  authenticate,45,78,"func,heading_1,func",1
  ```
- **Acceptable Variance**: ±5ms between runs

## Implementation Notes
- Create benchmark script: `/packages/maproom-mcp/scripts/benchmark-search.ts`
- Use indexed test corpus from SEMRANK-1004
- Export baseline to `/packages/maproom-mcp/benchmarks/baseline-fts.csv`
- Document methodology in `/packages/maproom-mcp/docs/baseline-methodology.md`
- Include EXPLAIN ANALYZE output for 5 representative queries
- Use TypeScript with existing MCP infrastructure for consistency
- Calculate percentiles using standard statistical methods

## Dependencies
- SEMRANK-1004 (test corpus must be indexed)
- SEMRANK-0001 (search tool must exist)

## Risk Assessment
- **Risk**: High variance in latency measurements
  - **Mitigation**: Run on dedicated environment, disable background processes
- **Risk**: Baseline shows severe ranking issues
  - **Mitigation**: Document honestly, justifies project value and validates approach

## Files/Packages Affected
- `/packages/maproom-mcp/scripts/benchmark-search.ts` (new)
- `/packages/maproom-mcp/benchmarks/baseline-fts.csv` (new)
- `/packages/maproom-mcp/docs/baseline-methodology.md` (new)
