# Ticket: AGENTOPT-1006 - Create Competition Runner Orchestrator

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Create the main orchestrator that ties together all components: spawns agents with SDK, runs search tasks, collects metrics, evaluates results, and determines winners.

## Background

This ticket integrates all previous work into a cohesive competition runner:
- SDK spawner (AGENTOPT-1001)
- Variant injection (AGENTOPT-1002)
- Competition framework (AGENTOPT-1003)
- Task library (AGENTOPT-1004)
- Evaluation metrics (AGENTOPT-1005)

**Goal**: Run end-to-end competition from start to winner selection.

## Acceptance Criteria

- [x] Main orchestrator function that runs complete competition
- [x] Spawns N agents with different variants
- [x] Executes search task in parallel
- [x] Collects all metrics during execution
- [x] Evaluates results with scoring
- [x] Determines and reports winner
- [x] Generates competition report
- [x] End-to-end test with 3 variants on 1 task

## Technical Requirements

**Main Orchestrator**:
```typescript
// packages/cli/src/search-optimization/competition-runner.ts

import { SearchCompetitionManager } from '../orchestrator/search-competition'
import { loadVariants } from './variants'
import { TASK_LIBRARY } from './tasks'
import { runSearchTaskEvaluation } from '../evaluation/search-checks'
import { generateEvaluationReport } from '../evaluation/report'

export interface CompetitionConfig {
  task: SearchTask                // Which task to run
  variants: string[]             // Variant IDs to compete
  parallelExecution: boolean     // Run agents in parallel?
  timeout: number                // Max time per agent (seconds)
}

export interface CompetitionResult {
  competitionId: string
  task: SearchTask
  participants: ParticipantResult[]
  winner: ParticipantResult
  metrics: CompetitionMetrics
  report: string
}

export interface ParticipantResult {
  variantId: string
  variantName: string
  score: number
  evaluation: SearchEvaluationSummary
  worktreePath: string
}

export async function runCompetition(
  config: CompetitionConfig
): Promise<CompetitionResult> {
  console.log(`Starting competition: ${config.task.name}`)
  console.log(`Variants: ${config.variants.join(', ')}`)

  // 1. Load variants
  const variants = await Promise.all(
    config.variants.map(id => loadVariant(id))
  )

  // 2. Create competition
  const manager = new SearchCompetitionManager()
  const competition = await manager.startSearchCompetition(
    config.task,
    variants
  )

  // 3. Execute in parallel or serial
  if (config.parallelExecution) {
    await Promise.all(
      competition.participants.map(p =>
        executeParticipant(p, config.timeout)
      )
    )
  } else {
    for (const p of competition.participants) {
      await executeParticipant(p, config.timeout)
    }
  }

  // 4. Evaluate all participants
  const results = await Promise.all(
    competition.participants.map(async (p) => {
      const evaluation = await runSearchTaskEvaluation(
        config.task,
        p.worktreePath!,
        p.runDir!
      )

      return {
        variantId: p.variant!.id,
        variantName: p.variant!.name,
        score: evaluation.compositeScore,
        evaluation,
        worktreePath: p.worktreePath!
      }
    })
  )

  // 5. Determine winner
  const winner = results.reduce((best, current) =>
    current.score > best.score ? current : best
  )

  // 6. Generate report
  const report = generateCompetitionReport({
    task: config.task,
    results,
    winner
  })

  // 7. Save competition state
  await manager.saveCompetition(competition)

  console.log(`\nWinner: ${winner.variantName} (${(winner.score * 100).toFixed(1)}%)`)

  return {
    competitionId: competition.id,
    task: config.task,
    participants: results,
    winner,
    metrics: calculateCompetitionMetrics(results),
    report
  }
}

async function executeParticipant(
  participant: SearchCompetitionParticipant,
  timeout: number
): Promise<void> {
  console.log(`  Running: ${participant.variant!.name}...`)

  const agent = await spawnAgentWithVariant(
    participant.task!.description,
    participant.variant!,
    participant.worktreePath!,
    {
      timeout: timeout * 1000,
      hooks: {
        onToolUse: (event) => logToolUse(participant.runDir!, event),
        onComplete: (result) => saveResult(participant.runDir!, result)
      }
    }
  )

  // Wait for completion
  for await (const message of agent) {
    // Process messages, capture metrics
    if (message.type === 'tool_use') {
      await logToolUse(participant.runDir!, message)
    }
  }

  console.log(`  Completed: ${participant.variant!.name}`)
}

function calculateCompetitionMetrics(
  results: ParticipantResult[]
): CompetitionMetrics {
  return {
    avgScore: results.reduce((sum, r) => sum + r.score, 0) / results.length,
    scoreRange: {
      min: Math.min(...results.map(r => r.score)),
      max: Math.max(...results.map(r => r.score))
    },
    avgSearchCount: results.reduce((sum, r) =>
      sum + r.evaluation.searchMetrics.searchCount, 0
    ) / results.length,
    successRate: results.filter(r =>
      r.evaluation.taskScore.taskCompletion > 0.5
    ).length / results.length
  }
}
```

**Competition Report**:
```typescript
function generateCompetitionReport(data: {
  task: SearchTask
  results: ParticipantResult[]
  winner: ParticipantResult
}): string {
  return `
COMPETITION REPORT
==================

Task: ${data.task.name}
Difficulty: ${data.task.difficulty}
Category: ${data.task.category}

RESULTS
-------
${data.results.map((r, i) => `
${i + 1}. ${r.variantName}
   Score: ${(r.score * 100).toFixed(1)}%
   Search Quality: ${(r.evaluation.taskScore.searchQuality * 100).toFixed(1)}%
   Task Completion: ${(r.evaluation.taskScore.taskCompletion * 100).toFixed(1)}%
   Efficiency: ${(r.evaluation.taskScore.efficiency * 100).toFixed(1)}%
   Searches: ${r.evaluation.searchMetrics.searchCount}
`).join('')}

WINNER
------
${data.winner.variantName} (${(data.winner.score * 100).toFixed(1)}%)

${data.winner.evaluation.taskScore.details}

NEXT STEPS
----------
- Use winner as baseline for next generation
- Generate mutations from winner
- Run next competition
`
}
```

## Implementation Notes

**Phased Approach** (Recommended):
- **Phase A (MVP)**: Sequential execution only
  - Run one agent at a time, wait for completion
  - Simpler debugging, lower resource usage
  - Implement `executeParticipant()` as blocking
- **Phase B (Optimization)**: Add parallel execution
  - After Phase A working and tested
  - Requires resource monitoring
  - May need process pooling or rate limiting

**CLI Integration** (Optional - can be done later):
```bash
# Run single competition
crewchief maproom:compete \
  --task impl-worktree-001 \
  --variants baseline,variant-a,variant-b

# Run all tasks
crewchief maproom:compete-all \
  --variants baseline,variant-a,variant-b
```

**File Structure**:
```
packages/cli/src/search-optimization/
├── competition-runner.ts (new - main orchestrator)
├── report-generator.ts (new - report formatting)
└── cli.ts (optional - CLI commands)
```

**Testing Strategy**:
1. Unit test: competition logic
2. Integration test: 1 agent, 1 task, mock SDK
3. E2E test: 3 agents, 1 real task, real SDK

## Dependencies

- AGENTOPT-1001 (SDK) - spawns agents
- AGENTOPT-1002 (variants) - injects descriptions
- AGENTOPT-1003 (framework) - manages competition
- AGENTOPT-1004 (tasks) - defines tasks
- AGENTOPT-1005 (evaluation) - scores results

## Risk Assessment

**Risk**: Agents run too long, block completion
**Mitigation**: Timeout mechanism, parallel execution option

**Risk**: Parallel execution causes resource contention
**Mitigation**: Test with sequential first, add parallel later

**Risk**: Winner selection is unclear (tie scores)
**Mitigation**: Use consistent tiebreaker (efficiency, then search quality)

## Files/Packages Affected

- packages/cli/src/search-optimization/competition-runner.ts (new)
- packages/cli/src/search-optimization/report-generator.ts (new)
- packages/cli/tests/search-optimization/competition-runner.test.ts (new)

## Planning References

- Replan Analysis: `.agents/work-in-progress/AGENTOPT-replan-analysis.md`
- All previous tickets (1001-1005)
