# Ticket: AGENTOPT-1005 - Extend Evaluation Framework with Search Metrics

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Extend the existing crewchief CLI evaluation framework to include search-specific metrics and scoring, integrating with the task validators from AGENTOPT-1004.

## Background

The crewchief CLI has an evaluation framework at `packages/cli/src/evaluation/checks.ts` that:
- Runs quality checks on agent work
- Returns scores (0-1 range)
- Supports config-driven custom checks

**Current Limitations**:
- Generic checks (env, events, build)
- No search task awareness
- No search metrics capture

**What We Need**:
- Search task success validation
- Tool usage metrics
- Composite scoring aligned with task validators

## Acceptance Criteria

- [ ] Extend `EvaluationSummary` for search tasks
- [ ] Implement search-specific quality checks
- [ ] Integrate with task validators (from AGENTOPT-1004)
- [ ] Capture and score search tool usage
- [ ] Generate detailed evaluation reports
- [ ] Verification test showing full evaluation flow

## Technical Requirements

**Extended Types** (add to `packages/cli/src/evaluation/checks.ts`):
```typescript
import { SearchTask, TaskScore } from '../search-optimization/types'

export interface SearchEvaluationSummary extends EvaluationSummary {
  task: SearchTask
  taskScore: TaskScore

  // Search-specific metrics
  searchMetrics: {
    searchCount: number
    avgResultsPerSearch: number
    queriesIssued: string[]
    targetFound: boolean
    targetFoundInTop: number | null  // null if not found, else rank (1-20)
  }

  // Tool usage
  toolUsage: {
    totalToolCalls: number
    searchToolCalls: number
    otherToolCalls: Record<string, number>
  }

  // Timing
  timing: {
    totalSeconds: number
    timeToTarget: number | null  // null if not found
  }

  // Overall
  compositeScore: number  // 0-1 (from task validator)
}
```

**Search Quality Check**:
```typescript
// packages/cli/src/evaluation/search-checks.ts

export async function runSearchTaskEvaluation(
  task: SearchTask,
  worktreePath: string,
  runDir: string
): Promise<SearchEvaluationSummary> {
  // 1. Load tool usage logs
  const toolLogs = await loadToolUsageLogs(runDir)

  // 2. Extract search metrics
  const searchMetrics = extractSearchMetrics(toolLogs, task.searchTarget)

  // 3. Validate task completion
  const workResult = await loadWorkResult(worktreePath)
  const taskScore = task.successValidator(workResult)

  // 4. Calculate tool usage stats
  const toolUsage = calculateToolUsage(toolLogs)

  // 5. Calculate timing
  const timing = calculateTiming(toolLogs)

  // 6. Run generic checks (from existing framework)
  const genericChecks = await runDefaultChecks(worktreePath, runDir)

  // 7. Combine into summary
  return {
    ...genericChecks,
    task,
    taskScore,
    searchMetrics,
    toolUsage,
    timing,
    compositeScore: taskScore.total
  }
}
```

**Tool Usage Log Parsing**:
```typescript
interface ToolUseLog {
  timestamp: number
  toolName: string
  arguments: Record<string, any>
  result: any
}

function extractSearchMetrics(
  logs: ToolUseLog[],
  target: SearchTarget
): SearchEvaluationSummary['searchMetrics'] {
  const searchLogs = logs.filter(log => log.toolName === 'search')

  return {
    searchCount: searchLogs.length,
    avgResultsPerSearch: calculateAvg(searchLogs.map(l => l.result.length)),
    queriesIssued: searchLogs.map(l => l.arguments.query),
    targetFound: checkIfTargetFound(searchLogs, target),
    targetFoundInTop: findTargetRank(searchLogs, target)
  }
}
```

**Report Generation**:
```typescript
export function generateEvaluationReport(
  summary: SearchEvaluationSummary
): string {
  return `
Search Task Evaluation Report
==============================

Task: ${summary.task.name}
Composite Score: ${(summary.compositeScore * 100).toFixed(1)}%

Breakdown:
- Search Quality: ${(summary.taskScore.searchQuality * 100).toFixed(1)}%
- Task Completion: ${(summary.taskScore.taskCompletion * 100).toFixed(1)}%
- Efficiency: ${(summary.taskScore.efficiency * 100).toFixed(1)}%

Search Metrics:
- Searches Performed: ${summary.searchMetrics.searchCount}
- Target Found: ${summary.searchMetrics.targetFound ? 'YES' : 'NO'}
${summary.searchMetrics.targetFoundInTop ? `- Target Rank: #${summary.searchMetrics.targetFoundInTop}` : ''}

Tool Usage:
- Total Tool Calls: ${summary.toolUsage.totalToolCalls}
- Search Tool: ${summary.toolUsage.searchToolCalls}

Timing:
- Total Duration: ${summary.timing.totalSeconds}s
${summary.timing.timeToTarget ? `- Time to Target: ${summary.timing.timeToTarget}s` : ''}

${summary.taskScore.details}
`
}
```

## Implementation Notes

**Integration Points**:
- Extends existing `packages/cli/src/evaluation/checks.ts`
- Uses task validators from AGENTOPT-1004
- Hooks into competition scoring (AGENTOPT-1003)
- Feeds genetic algorithm (AGENTOPT-1007)

**File Structure**:
```
packages/cli/src/evaluation/
├── checks.ts (existing - extend types)
├── search-checks.ts (new - search evaluation)
└── report.ts (new - report generation)
```

**Design Principles**:
- Reuse existing evaluation infrastructure
- Add search-specific logic as extension
- Keep generic checks working
- Maintain 0-1 score compatibility

## Dependencies

- AGENTOPT-1004 (task library) - provides task validators
- AGENTOPT-1003 (competition framework) - consumes evaluation results

## Risk Assessment

**Risk**: Tool logs don't capture enough detail
**Mitigation**: Test with SDK hooks (from AGENTOPT-1001), ensure all searches logged

**Risk**: Validators have edge cases
**Mitigation**: Extensive testing with known good/bad outputs

**Risk**: Score inflation/deflation
**Mitigation**: Test with multiple tasks, calibrate weights

## Files/Packages Affected

- packages/cli/src/evaluation/checks.ts (extend)
- packages/cli/src/evaluation/search-checks.ts (new)
- packages/cli/src/evaluation/report.ts (new)
- packages/cli/tests/evaluation/search-checks.test.ts (new)

## Planning References

- Existing Evaluation: `packages/cli/src/evaluation/checks.ts`
- Task Validators: AGENTOPT-1004
- Deep Thinking: `.agents/work-in-progress/search-tasks-deep-thinking.md`
