# Ticket: TESTDES-1002: Implement Baseline Runner for Grep-Only Evaluation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the baseline runner that executes SearchTask with grep/glob/read tools only (no semantic search) to establish performance baselines. This is critical for objective comparison—we must prove grep fails/struggles before claiming search is better. The runner measures success rate, execution time, and tool usage patterns.

## Background
To prove semantic search provides value, we need objective comparison against a control condition. The baseline runner executes SearchTask with only grep/glob/read tools available, measuring success rate, time, and tool usage. This establishes what's possible without semantic search, enabling scientific comparison.

Research from information retrieval (TREC benchmarks) shows baseline comparison is essential for valid evaluation. Without baseline, we can't attribute performance improvements to the tool being tested—we have no counterfactual.

This implements the "Baseline Measurement" component from the TESTDES architecture, providing the control condition against which semantic search performance will be compared in Phase 2.

**Reference**: See architecture.md Section "Evaluation Infrastructure" for baseline runner design and metrics specification.

## Acceptance Criteria
- [x] Can execute SearchTask with configurable tool subset (grep/glob/read only)
- [x] Captures comprehensive metrics: success boolean, duration seconds, tool call counts by type, search queries issued
- [x] Returns structured BaselineResult with all metrics and task reference
- [x] Integration test demonstrates successful task execution and metric capture
- [x] Handles timeouts gracefully (returns partial results with timeout flag)
- [x] Handles errors gracefully (returns failure result with error details)

## Technical Requirements
- TypeScript implementation in `packages/cli/src/search-optimization/evaluation/baseline-runner.ts`
- Integrate with existing `spawnAgentWithVariant` from SDK for agent execution
- Define `BaselineConfig` interface: task, availableTools[], timeout, baseDir
- Define `BaselineResult` interface: task, success, metrics (time, toolCalls, etc), transcript
- Use existing tool-usage.log pattern from competition-runner.ts for metrics
- Follow ESM module conventions and existing code style
- Use Vitest for unit and integration tests

## Implementation Notes
The baseline runner is similar to competition-runner.ts but simpler:
- Only runs one variant (no competition between tools)
- Explicitly restricts available tools to create control condition
- Focus on measurement, not comparison (comparison logic comes in Phase 1.3)

**Key insight**: By removing `mcp__maproom__search` from available tools, we force agents to use grep/glob patterns. This creates the control condition for our experiment, showing what's achievable without semantic search.

**Agent spawning pattern**: Reference `packages/cli/src/search-optimization/competition-runner.ts` lines 113-178 for the agent spawning and metrics collection approach. The baseline runner should follow similar patterns but without the competition logic.

**Metrics to capture**:
- `success`: boolean (did agent complete the task successfully)
- `durationSeconds`: number (total execution time)
- `toolCalls`: Record<string, number> (counts per tool type)
- `searchQueries`: string[] (all grep patterns attempted)
- `filesExamined`: number (unique files read)
- `timedOut`: boolean (did execution hit timeout)

**Timeout handling**: Set reasonable timeout (e.g., 5 minutes). If timeout occurs, gracefully terminate agent and return partial results with `timedOut: true` flag.

**Tool restriction**: Available tools should be explicitly set to `['grep', 'glob', 'read']` in the agent variant configuration. This prevents agents from accessing semantic search tools.

## Dependencies
- TESTDES-1001 (task types from taxonomy)

## Risk Assessment
- **Risk**: Agents might give up quickly without search tool, skewing baseline results
  - **Mitigation**: Set appropriate timeouts (5+ minutes), use task descriptions that don't assume specific tools, validate with pilot runs

- **Risk**: Tool restriction might confuse agents or cause unexpected errors
  - **Mitigation**: Clear task descriptions, graceful error handling, log all agent responses for debugging

- **Risk**: Metrics collection might miss important signals
  - **Mitigation**: Start with comprehensive metrics, can refine based on Phase 2 analysis needs

## Files/Packages Affected
**Files to Create**:
- `packages/cli/src/search-optimization/evaluation/baseline-runner.ts`
- `packages/cli/src/search-optimization/evaluation/__tests__/baseline-runner.test.ts`
- `packages/cli/src/search-optimization/evaluation/index.ts` (if doesn't exist)

**Files to Reference**:
- `packages/cli/src/search-optimization/competition-runner.ts` (for agent spawning pattern)
- `packages/cli/src/search-optimization/taxonomy/index.ts` (for SearchTask types)
