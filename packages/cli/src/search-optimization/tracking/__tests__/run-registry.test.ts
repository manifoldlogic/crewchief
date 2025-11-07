/**
 * Tests for run registry system
 */

import { mkdirSync, rmSync, existsSync } from 'fs'
import { join } from 'path'
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import type { Variant } from '../../../../../maproom-mcp/test/tool-description-optimization/types.js'
import type { IterationConfig, IterationHistory, Generation } from '../../genetic-iterator.js'
import type { MultiTierScore } from '../../multi-tier-scoring.js'
import {
  loadRunRegistry,
  registerRun,
  updateRunStatus,
  extractLearnings,
  compareRunResults,
  exportLearnings,
  getRun,
  listRuns,
  generateRunRegistryReport,
} from '../run-registry.js'

const TEST_BASE_DIR = join('/tmp', 'tracking-test-run-registry')

// Helper to create a mock config
function createMockConfig(): IterationConfig {
  return {
    initialVariants: ['v1'],
    tasks: [],
    maxIterations: 5,
    convergenceThreshold: 0.01,
    mutationRate: 0.2,
    populationSize: 4,
  }
}

// Helper to create a mock variant
function createMockVariant(id: string, name: string, generation: number): Variant {
  return {
    id,
    name,
    description: `Mock variant ${name}`,
    tokens: 500,
    generation,
    parent_ids: [],
    created_at: new Date(),
    mutation_type: generation > 0 ? 'amplification' : undefined,
  }
}

// Helper to create a mock multi-tier score
function createMockScore(composite: number): MultiTierScore {
  return {
    composite,
    tierMetrics: {
      tier1: { avgScore: composite * 0.9, searchUsageRate: 0.8, appropriateUsage: 0.8, taskCompletionRate: 0.7 },
      tier2: { avgScore: composite * 0.95, searchUsageRate: 0.85, appropriateUsage: 0.85, taskCompletionRate: 0.75 },
      tier3: { avgScore: composite * 1.05, searchUsageRate: 0.5, appropriateUsage: 0.9, taskCompletionRate: 0.8 },
    },
    toolSelection: {
      tier1SearchRate: 0.8,
      tier2SearchRate: 0.85,
      tier3SearchRate: 0.5,
      tier1Accuracy: 0.8,
      tier2Accuracy: 0.85,
      tier3Accuracy: 0.9,
      overallAccuracy: 0.85,
    },
    taskCoverage: {
      total: 30,
      passed: 22,
    },
  }
}

// Helper to create mock iteration history
function createMockHistory(generations: number, converged: boolean): IterationHistory {
  const gens: Generation[] = []
  const variants = [createMockVariant('v1', 'Variant 1', 0)]

  for (let i = 0; i < generations; i++) {
    const gen: Generation = {
      number: i + 1,
      variants,
      taskResults: new Map(),
      avgScore: 0.6 + i * 0.05,
      bestVariant: variants[0],
      bestScore: 0.6 + i * 0.05,
      improvement: i > 0 ? 0.05 : 0,
      multiTierScores: new Map([['v1', createMockScore(0.6 + i * 0.05)]]),
    }
    gens.push(gen)
  }

  return {
    generations: gens,
    bestOverall: variants[0],
    convergenceReached: converged,
    totalIterations: generations,
  }
}

describe('Run Registry System', () => {
  beforeEach(() => {
    // Clean up test directory
    if (existsSync(TEST_BASE_DIR)) {
      rmSync(TEST_BASE_DIR, { recursive: true, force: true })
    }
    mkdirSync(TEST_BASE_DIR, { recursive: true })
  })

  afterEach(() => {
    // Clean up after tests
    if (existsSync(TEST_BASE_DIR)) {
      rmSync(TEST_BASE_DIR, { recursive: true, force: true })
    }
  })

  it('should initialize empty run registry', () => {
    const registry = loadRunRegistry(TEST_BASE_DIR)

    expect(registry.runs).toHaveLength(0)
    expect(registry.schemaVersion).toBe(1)
  })

  it('should register new run', () => {
    const config = createMockConfig()

    const run = registerRun('run-123', config, TEST_BASE_DIR)

    expect(run.runId).toBe('run-123')
    expect(run.status).toBe('running')
    expect(run.convergenceReached).toBe(false)
    expect(run.config).toEqual(config)
  })

  it('should add run to registry', () => {
    const config = createMockConfig()

    registerRun('run-123', config, TEST_BASE_DIR)

    const registry = loadRunRegistry(TEST_BASE_DIR)
    expect(registry.runs).toHaveLength(1)
    expect(registry.runs[0].runId).toBe('run-123')
  })

  it('should update run status on completion', () => {
    const config = createMockConfig()
    const history = createMockHistory(3, true)

    registerRun('run-123', config, TEST_BASE_DIR)
    const updated = updateRunStatus('run-123', 'completed', history, TEST_BASE_DIR)

    expect(updated.status).toBe('completed')
    expect(updated.convergenceReached).toBe(true)
    expect(updated.generations).toBe(3)
    expect(updated.learnings).not.toBeNull()
  })

  it('should extract learnings from history', () => {
    const history = createMockHistory(5, true)

    const learnings = extractLearnings(history)

    expect(learnings.convergencePattern.generationsToConverge).toBe(5)
    expect(learnings.convergencePattern.plateauDetected).toBe(false)
    expect(learnings.scoreVelocity.avgImprovementPerGeneration).toBeGreaterThan(0)
    expect(learnings.insights).toBeInstanceOf(Array)
  })

  it('should detect plateau in learnings', () => {
    // Create history with minimal improvements (plateau)
    const gens: Generation[] = []
    const variant = createMockVariant('v1', 'Variant 1', 0)

    for (let i = 0; i < 5; i++) {
      const gen: Generation = {
        number: i + 1,
        variants: [variant],
        taskResults: new Map(),
        avgScore: 0.6,
        bestVariant: variant,
        bestScore: 0.6,
        improvement: 0.001, // Minimal improvement
        multiTierScores: new Map([['v1', createMockScore(0.6)]]),
      }
      gens.push(gen)
    }

    const history: IterationHistory = {
      generations: gens,
      bestOverall: variant,
      convergenceReached: false,
      totalIterations: 5,
    }

    const learnings = extractLearnings(history)

    expect(learnings.convergencePattern.plateauDetected).toBe(true)
  })

  it('should track task coverage trends', () => {
    const history = createMockHistory(3, true)

    const learnings = extractLearnings(history)

    expect(learnings.taskCoverageTrends.startingPassRate).toBeGreaterThan(0)
    expect(learnings.taskCoverageTrends.finalPassRate).toBeGreaterThan(0)
    expect(learnings.taskCoverageTrends.improvement).toBeGreaterThanOrEqual(0)
  })

  it('should compare two runs', () => {
    const config = createMockConfig()

    registerRun('run-1', config, TEST_BASE_DIR)
    registerRun('run-2', config, TEST_BASE_DIR)

    updateRunStatus('run-1', 'completed', createMockHistory(3, true), TEST_BASE_DIR)
    updateRunStatus('run-2', 'completed', createMockHistory(4, false), TEST_BASE_DIR)

    const comparison = compareRunResults('run-1', 'run-2', TEST_BASE_DIR)

    expect(comparison).toContain('RUN COMPARISON')
    expect(comparison).toContain('run-1')
    expect(comparison).toContain('run-2')
    expect(comparison).toContain('Converged: Yes')
    expect(comparison).toContain('Converged: No')
  })

  it('should export learnings from run', () => {
    const config = createMockConfig()

    registerRun('run-123', config, TEST_BASE_DIR)
    updateRunStatus('run-123', 'completed', createMockHistory(3, true), TEST_BASE_DIR)

    const learningsReport = exportLearnings('run-123', TEST_BASE_DIR)

    expect(learningsReport).toContain('LEARNINGS FROM RUN: run-123')
    expect(learningsReport).toContain('KEY INSIGHTS')
    expect(learningsReport).toContain('CONVERGENCE PATTERN')
  })

  it('should get run by ID', () => {
    const config = createMockConfig()

    registerRun('run-123', config, TEST_BASE_DIR)

    const run = getRun('run-123', TEST_BASE_DIR)

    expect(run).not.toBeNull()
    expect(run?.runId).toBe('run-123')
  })

  it('should return null for non-existent run', () => {
    const run = getRun('non-existent', TEST_BASE_DIR)
    expect(run).toBeNull()
  })

  it('should list all runs', () => {
    const config = createMockConfig()

    registerRun('run-1', config, TEST_BASE_DIR)
    registerRun('run-2', config, TEST_BASE_DIR)

    const runs = listRuns(TEST_BASE_DIR)

    expect(runs).toHaveLength(2)
    expect(runs[0].runId).toBe('run-1')
    expect(runs[1].runId).toBe('run-2')
  })

  it('should generate run registry report', () => {
    const config = createMockConfig()

    registerRun('run-123', config, TEST_BASE_DIR)
    updateRunStatus('run-123', 'completed', createMockHistory(3, true), TEST_BASE_DIR)

    const report = generateRunRegistryReport(TEST_BASE_DIR)

    expect(report).toContain('OPTIMIZATION RUN REGISTRY')
    expect(report).toContain('run-123')
    expect(report).toContain('completed')
  })

  it('should throw error when updating non-existent run', () => {
    const history = createMockHistory(3, true)

    expect(() => {
      updateRunStatus('non-existent', 'completed', history, TEST_BASE_DIR)
    }).toThrow('Run non-existent not found in registry')
  })

  it('should throw error when exporting learnings from non-existent run', () => {
    expect(() => {
      exportLearnings('non-existent', TEST_BASE_DIR)
    }).toThrow('Run non-existent not found in registry')
  })

  it('should throw error when exporting learnings from run without learnings', () => {
    const config = createMockConfig()

    registerRun('run-123', config, TEST_BASE_DIR)

    expect(() => {
      exportLearnings('run-123', TEST_BASE_DIR)
    }).toThrow('has no learnings')
  })

  it('should throw error when comparing non-existent runs', () => {
    expect(() => {
      compareRunResults('run-1', 'run-2', TEST_BASE_DIR)
    }).toThrow('Run(s) not found')
  })

  it('should preserve timestamps correctly', () => {
    const config = createMockConfig()
    const beforeTime = new Date()

    const run = registerRun('run-123', config, TEST_BASE_DIR)

    const afterTime = new Date()

    expect(run.startedAt).toBeInstanceOf(Date)
    expect(run.startedAt.getTime()).toBeGreaterThanOrEqual(beforeTime.getTime())
    expect(run.startedAt.getTime()).toBeLessThanOrEqual(afterTime.getTime())
  })

  it('should track multi-tier mode', () => {
    const config: IterationConfig = {
      ...createMockConfig(),
      multiTier: {
        enabled: true,
        tier1Suite: { name: 'Tier 1', tasks: [] },
        tier2Suite: { name: 'Tier 2', tasks: [] },
        tier3Suite: { name: 'Tier 3', tasks: [] },
      },
    }

    const run = registerRun('run-123', config, TEST_BASE_DIR)

    expect(run.multiTierEnabled).toBe(true)
  })

  it('should generate insights about mutation performance', () => {
    const history = createMockHistory(3, true)

    const learnings = extractLearnings(history)

    expect(learnings.bestMutationTypes).toBeInstanceOf(Array)
  })

  it('should track optimization velocity', () => {
    const history = createMockHistory(5, true)

    const learnings = extractLearnings(history)

    expect(learnings.scoreVelocity.avgImprovementPerGeneration).toBeGreaterThan(0)
    expect(learnings.scoreVelocity.bestGenerationImprovement).toBeGreaterThanOrEqual(
      learnings.scoreVelocity.avgImprovementPerGeneration,
    )
  })

  it('should include successful parameters in learnings', () => {
    const history = createMockHistory(3, true)

    const learnings = extractLearnings(history)

    expect(learnings.successfulParameters.populationSize).toBeGreaterThan(0)
    expect(learnings.successfulParameters.convergenceThreshold).toBeGreaterThan(0)
  })
})
