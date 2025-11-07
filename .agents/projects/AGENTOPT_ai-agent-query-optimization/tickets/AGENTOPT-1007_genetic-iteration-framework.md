# Ticket: AGENTOPT-1007 - Build Genetic Iteration Framework

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Implement genetic algorithm framework for continuous tool description optimization: take competition winner, generate new mutated variants, run next competition, iterate.

## Background

The user wants a "genetic" approach - continuous iteration rather than one-shot optimization. After each competition:
1. Winner becomes new baseline
2. Generate mutations from winner (using AGENTOPT-0002 mutator)
3. Run next competition with new variants
4. Repeat until convergence or manual stop

This enables ongoing adaptation as:
- Claude models evolve
- Agent capabilities change
- Real-world usage patterns emerge
- New search patterns are discovered

## Acceptance Criteria

- [ ] Iteration orchestrator that runs multiple competition rounds
- [ ] Automatic variant generation from winners
- [ ] Convergence detection (stop when no improvement)
- [ ] Generation tracking and history
- [ ] Iteration report showing progress over time
- [ ] CLI command to run N iterations
- [ ] End-to-end test: 3 iterations with improvement

## Technical Requirements

**Iteration Orchestrator**:
```typescript
// packages/cli/src/search-optimization/genetic-iterator.ts

import { runCompetition } from './competition-runner'
import { mutate, applyCrossover } from '../test/tool-description-optimization/mutator'
import { loadVariant, saveVariant } from './variants'

export interface IterationConfig {
  initialVariants: string[]      // Starting variants
  tasks: SearchTask[]            // Tasks to run each iteration
  maxIterations: number          // Stop after N iterations
  convergenceThreshold: number   // Stop if improvement < threshold
  mutationRate: number           // Probability of mutation (0-1)
  populationSize: number         // Variants per generation
}

export interface IterationHistory {
  generations: Generation[]
  bestOverall: Variant
  convergenceReached: boolean
  totalIterations: number
}

export interface Generation {
  number: number
  variants: Variant[]
  taskResults: Map<string, CompetitionResult>  // task.id -> result
  avgScore: number
  bestVariant: Variant
  bestScore: number
  improvement: number            // vs previous generation
}

export async function runGeneticIterations(
  config: IterationConfig
): Promise<IterationHistory> {
  console.log('Starting genetic iterations...')
  console.log(`Population: ${config.populationSize}`)
  console.log(`Max Iterations: ${config.maxIterations}`)

  const history: Generation[] = []
  let currentVariants = await Promise.all(
    config.initialVariants.map(id => loadVariant(id))
  )

  for (let i = 0; i < config.maxIterations; i++) {
    console.log(`\n=== GENERATION ${i + 1} ===\n`)

    // Run competitions on all tasks
    const taskResults = new Map<string, CompetitionResult>()
    const variantScores = new Map<string, number[]>()

    for (const task of config.tasks) {
      console.log(`Running task: ${task.name}`)

      const result = await runCompetition({
        task,
        variants: currentVariants.map(v => v.id),
        parallelExecution: true,
        timeout: task.maxTimeSeconds || 300
      })

      taskResults.set(task.id, result)

      // Aggregate scores for each variant
      for (const participant of result.participants) {
        if (!variantScores.has(participant.variantId)) {
          variantScores.set(participant.variantId, [])
        }
        variantScores.get(participant.variantId)!.push(participant.score)
      }
    }

    // Calculate average score per variant across all tasks
    const variantAvgScores = new Map<string, number>()
    for (const [variantId, scores] of variantScores) {
      const avg = scores.reduce((sum, s) => sum + s, 0) / scores.length
      variantAvgScores.set(variantId, avg)
    }

    // Find best variant of this generation
    let bestVariant = currentVariants[0]
    let bestScore = variantAvgScores.get(bestVariant.id) || 0

    for (const variant of currentVariants) {
      const score = variantAvgScores.get(variant.id) || 0
      if (score > bestScore) {
        bestScore = score
        bestVariant = variant
      }
    }

    // Calculate improvement vs previous generation
    const prevBest = history[history.length - 1]?.bestScore || 0
    const improvement = bestScore - prevBest

    // Record generation
    const generation: Generation = {
      number: i + 1,
      variants: [...currentVariants],
      taskResults,
      avgScore: Array.from(variantAvgScores.values())
        .reduce((sum, s) => sum + s, 0) / variantAvgScores.size,
      bestVariant,
      bestScore,
      improvement
    }

    history.push(generation)

    console.log(`\nGeneration ${i + 1} Summary:`)
    console.log(`Best: ${bestVariant.name} (${(bestScore * 100).toFixed(1)}%)`)
    console.log(`Avg: ${(generation.avgScore * 100).toFixed(1)}%`)
    console.log(`Improvement: ${improvement > 0 ? '+' : ''}${(improvement * 100).toFixed(2)}%`)

    // Check convergence
    if (Math.abs(improvement) < config.convergenceThreshold) {
      console.log(`\nConvergence reached (improvement < ${config.convergenceThreshold})`)
      return {
        generations: history,
        bestOverall: bestVariant,
        convergenceReached: true,
        totalIterations: i + 1
      }
    }

    // Generate next generation
    console.log(`\nGenerating next generation...`)
    currentVariants = await generateNextGeneration(
      currentVariants,
      variantAvgScores,
      config
    )
  }

  // Max iterations reached
  const bestOverall = history.reduce((best, gen) =>
    gen.bestScore > best.bestScore ? gen : best
  ).bestVariant

  return {
    generations: history,
    bestOverall,
    convergenceReached: false,
    totalIterations: config.maxIterations
  }
}

async function generateNextGeneration(
  currentVariants: Variant[],
  scores: Map<string, number>,
  config: IterationConfig
): Promise<Variant[]> {
  // Sort by score (best first)
  const sorted = [...currentVariants].sort((a, b) =>
    (scores.get(b.id) || 0) - (scores.get(a.id) || 0)
  )

  const nextGen: Variant[] = []

  // 1. Keep best variant (elitism)
  nextGen.push(sorted[0])

  // 2. Crossover top 2 variants
  if (sorted.length >= 2) {
    const crossed = await applyCrossover(sorted[0], sorted[1])
    nextGen.push(crossed)
  }

  // 3. Mutate best variant
  const mutationTypes = ['amplification', 'reduction', 'reframing', 'specialization'] as const

  for (let i = 0; i < config.populationSize - nextGen.length; i++) {
    const mutationType = mutationTypes[i % mutationTypes.length]
    const mutated = await mutate(sorted[0], mutationType)
    nextGen.push(mutated)
  }

  // Save new variants
  for (const variant of nextGen) {
    await saveVariant(variant)
  }

  return nextGen
}
```

**Iteration Report**:
```typescript
export function generateIterationReport(
  history: IterationHistory
): string {
  return `
GENETIC ITERATION REPORT
========================

Total Iterations: ${history.totalIterations}
Convergence: ${history.convergenceReached ? 'YES' : 'NO (max iterations reached)'}

PROGRESS OVER TIME
------------------
${history.generations.map(gen => `
Generation ${gen.number}:
  Best: ${gen.bestVariant.name} - ${(gen.bestScore * 100).toFixed(1)}%
  Avg:  ${(gen.avgScore * 100).toFixed(1)}%
  Δ:    ${gen.improvement > 0 ? '+' : ''}${(gen.improvement * 100).toFixed(2)}%
`).join('')}

OVERALL BEST
------------
${history.bestOverall.name}
Score: ${(history.generations.find(g => g.bestVariant.id === history.bestOverall.id)?.bestScore || 0) * 100}%

${history.bestOverall.description.substring(0, 200)}...

RECOMMENDATION
--------------
${history.convergenceReached
  ? 'Deploy this variant to production'
  : 'Consider running more iterations or adjusting mutation strategy'
}
`
}
```

## Implementation Notes

**CLI Integration**:
```bash
# Run 10 iterations with 5 tasks
crewchief maproom:evolve \
  --iterations 10 \
  --population 5 \
  --tasks impl-worktree-001,arch-competition-001,error-db-001

# Continue from previous run
crewchief maproom:evolve --continue
```

**File Structure**:
```
packages/cli/src/search-optimization/
├── genetic-iterator.ts (new - main iteration logic)
├── convergence.ts (new - convergence detection)
└── generation-history.ts (new - history tracking)
```

**Convergence Criteria**:
- Improvement < threshold (e.g., 0.01 = 1%)
- OR No improvement for N consecutive generations
- OR Manual stop

**Population Management**:
- Keep best (elitism)
- Crossover top performers
- Mutate with variety
- Discard poor performers

**Future Enhancements** (out of scope):
- Multi-objective optimization (search quality + efficiency + simplicity)
- Adaptive mutation rates
- Tournament selection
- Island model (parallel populations)

## Dependencies

- AGENTOPT-1006 (competition runner) - runs competitions
- AGENTOPT-0002 (mutator) - generates new variants
- All previous tickets (1001-1005)

## Risk Assessment

**Risk**: Gets stuck in local optimum
**Mitigation**: Occasional random mutations, crossover diversity

**Risk**: Takes too long to converge
**Mitigation**: Set max iterations, allow early stopping

**Risk**: Overfitting to specific tasks
**Mitigation**: Use diverse task set, validate on held-out tasks

## Files/Packages Affected

- packages/cli/src/search-optimization/genetic-iterator.ts (new)
- packages/cli/src/search-optimization/convergence.ts (new)
- packages/cli/src/search-optimization/generation-history.ts (new)
- packages/cli/tests/search-optimization/genetic-iterator.test.ts (new)

## Planning References

- Replan Analysis: `.agents/work-in-progress/AGENTOPT-replan-analysis.md`
- Mutation System: AGENTOPT-0002
- Competition Runner: AGENTOPT-1006
