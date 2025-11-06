# Ticket: AGENTOPT-0003: Implement Automated Testing Harness

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Build an automated testing harness that runs 100 test queries across multiple variants, simulates agent query transformation, and collects performance metrics. Supports parallel execution for fast iteration.

## Background
This ticket implements Phase 0, Step 3 from the AGENTOPT project plan (planning/plan.md lines 428-440). The testing harness is the execution engine for the data-driven optimization framework. It must simulate how Claude Code would transform queries given different tool descriptions, execute those searches, and measure results.

Three simulation strategies are supported:
1. Rule-based (free, 60-70% accuracy) - for rapid iteration
2. Haiku-based ($0.10/experiment, 85-90% accuracy) - for validation
3. API-based ($1/experiment, 95% accuracy) - for final decisions

## Acceptance Criteria
- [ ] Testing harness runs 100 queries per variant in <30 minutes
- [ ] Agent simulation implemented (start with rule-based, support LLM-based)
- [ ] Parallel execution for testing multiple variants concurrently
- [ ] Metrics collector records success_rate, avg_results, transformations
- [ ] Result formatter outputs both JSON and human-readable reports

## Technical Requirements
- Variant tester executes all queries from AGENTOPT-0001 test set
- Agent simulation strategies (architecture.md lines 899-948):
  - Rule-based: Extract patterns from variant, apply heuristics
  - LLM-based: Use Claude API to simulate transformation
  - API-based: Full Claude Sonnet simulation
- Metrics collection:
  - Success rate (% queries with ≥3 results)
  - Average result count
  - Query transformation consistency
  - Top-3 relevance (manual spot check support)
- Parallel execution using Promise.all or worker threads
- Structured output format for statistical analysis

## Implementation Notes
Create in `packages/maproom-mcp/test/tool-description-optimization/`:
- tester.ts (main testing orchestrator)
- simulator.ts (agent transformation simulation)
- metrics.ts (metrics collection and aggregation)
- reporter.ts (result formatting)

Start with rule-based simulation for MVP, add LLM-based later. Testing flow:
1. Load variant and test query set
2. For each query: simulate transformation → execute search → record metrics
3. Aggregate results per variant
4. Output summary report

## Dependencies
- AGENTOPT-0001 (test query set)
- AGENTOPT-0002 (variant system)
- Access to maproom MCP search tool

## Risk Assessment
- **Risk**: Testing too slow (>1 hour per experiment)
  - **Mitigation**: Parallel execution, start with 20-query subset for speed
- **Risk**: Agent simulation inaccurate
  - **Mitigation**: Validate rule-based against real API on subset

## Files/Packages Affected
- packages/maproom-mcp/test/tool-description-optimization/tester.ts
- packages/maproom-mcp/test/tool-description-optimization/simulator.ts
- packages/maproom-mcp/test/tool-description-optimization/metrics.ts
- packages/maproom-mcp/test/tool-description-optimization/reporter.ts

## Planning References
- Plan: planning/plan.md lines 428-440
- Architecture: planning/architecture.md lines 855-948, 984-1027
