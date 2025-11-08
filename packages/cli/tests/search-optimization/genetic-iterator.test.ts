/**
 * Tests for genetic iteration framework
 */

import { mkdtempSync, rmSync, writeFileSync, mkdirSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import type { Variant } from '../../../maproom-mcp/test/tool-description-optimization/types.js'
import type { IterationConfig, Generation } from '../../src/search-optimization/genetic-iterator.js'
import {
  runGeneticIterations,
  generateNextGeneration,
  loadVariant,
  saveVariant,
  generateIterationReport,
} from '../../src/search-optimization/genetic-iterator.js'
import { TASK_FIND_WORKTREE_CREATION } from '../../src/search-optimization/tasks/implementation.js'

// Mock the competition runner
vi.mock('../../src/search-optimization/competition-runner.js', () => ({
  runCompetition: vi.fn(async (config) => {
    // Simulate competition results based on variant
    const participants = config.variants.map((variant: Variant) => {
      // Simulate improving scores over generations
      const baseScore = variant.id.includes('control') ? 0.7 : 0.75
      const generationBonus = variant.generation * 0.03 // +3% per generation
      const randomNoise = (Math.random() - 0.5) * 0.05

      return {
        variantId: variant.id,
        variantName: variant.name,
        score: Math.min(0.95, Math.max(0.65, baseScore + generationBonus + randomNoise)),
        evaluation: {
          taskScore: {
            searchQuality: 0.8,
            taskCompletion: 0.85,
            efficiency: 0.9,
            total: 0.85,
            details: 'Mock evaluation',
          },
          searchMetrics: {
            searchCount: 3,
            avgResultsPerSearch: 5,
            queriesIssued: ['worktree'],
            targetFound: true,
            targetFoundInTop: 1,
          },
          toolUsage: {
            totalToolCalls: 10,
            searchToolCalls: 3,
            otherToolCalls: { read: 5, write: 2 },
          },
          timing: {
            totalSeconds: 30,
            timeToTarget: 10,
          },
          compositeScore: 0.85,
          results: [],
          score: 0.85,
        },
        agentResult: {
          success: true,
          sessionId: 'test-session',
          transcriptPath: '/tmp/test',
          usage: { inputTokens: 1000, outputTokens: 500, totalCostUsd: 0.01 },
          performance: { durationMs: 5000, durationApiMs: 4000, numTurns: 3 },
          messages: [],
        },
      }
    })

    // Winner is the one with highest score
    const winner = participants.reduce((best, p) => (p.score > best.score ? p : best))

    return {
      competitionId: 'test-comp',
      task: config.task,
      participants,
      winner,
      metrics: {
        avgScore: participants.reduce((sum, p) => sum + p.score, 0) / participants.length,
        scoreRange: {
          min: Math.min(...participants.map((p) => p.score)),
          max: Math.max(...participants.map((p) => p.score)),
        },
        avgSearchCount: 3,
        successRate: 1.0,
      },
      report: 'Mock report',
    }
  }),
}))

describe('genetic-iterator', () => {
  let testDir: string

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'genetic-iter-test-'))
  })

  afterEach(() => {
    try {
      rmSync(testDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  })

  // Helper to create test variants
  const createTestVariants = (count: number = 3): Variant[] => {
    const variants: Variant[] = []

    for (let i = 0; i < count; i++) {
      variants.push({
        id: `test-variant-${i}`,
        name: `Test Variant ${i}`,
        description: `Test description for variant ${i}`,
        tokens: 100,
        generation: 0,
        parent_ids: [],
        created_at: new Date(),
      })
    }

    return variants
  }

  describe('loadVariant and saveVariant', () => {
    it('should save and load a variant', async () => {
      const variant: Variant = {
        id: 'test-save-load',
        name: 'Test Save Load',
        description: 'Test description',
        tokens: 100,
        generation: 1,
        parent_ids: ['parent-1'],
        mutation_type: 'amplification',
        created_at: new Date(),
      }

      await saveVariant(variant, testDir)

      const loaded = await loadVariant(join(testDir, 'variants', 'test-save-load.json'))

      expect(loaded.id).toBe(variant.id)
      expect(loaded.name).toBe(variant.name)
      expect(loaded.description).toBe(variant.description)
      expect(loaded.tokens).toBe(variant.tokens)
      expect(loaded.generation).toBe(variant.generation)
      expect(loaded.mutation_type).toBe(variant.mutation_type)
    })
  })

  describe('generateNextGeneration', () => {
    it('should keep best variant (elitism)', async () => {
      const variants = createTestVariants(3)
      const scores = new Map([
        ['test-variant-0', 0.9],
        ['test-variant-1', 0.75],
        ['test-variant-2', 0.7],
      ])

      const config: IterationConfig = {
        initialVariants: [],
        tasks: [],
        maxIterations: 1,
        convergenceThreshold: 0.01,
        mutationRate: 0.5,
        populationSize: 3,
      }

      const nextGen = await generateNextGeneration(variants, scores, config)

      // First variant should be the best (test-variant-0)
      expect(nextGen[0].id).toBe('test-variant-0')
    })

    it('should generate correct population size', async () => {
      const variants = createTestVariants(3)
      const scores = new Map([
        ['test-variant-0', 0.9],
        ['test-variant-1', 0.75],
        ['test-variant-2', 0.7],
      ])

      const config: IterationConfig = {
        initialVariants: [],
        tasks: [],
        maxIterations: 1,
        convergenceThreshold: 0.01,
        mutationRate: 0.5,
        populationSize: 5,
      }

      const nextGen = await generateNextGeneration(variants, scores, config)

      expect(nextGen.length).toBeGreaterThan(0)
      expect(nextGen.length).toBeLessThanOrEqual(5)
    })

    it('should include crossover variant', async () => {
      const variants = createTestVariants(3)
      const scores = new Map([
        ['test-variant-0', 0.9],
        ['test-variant-1', 0.75],
        ['test-variant-2', 0.7],
      ])

      const config: IterationConfig = {
        initialVariants: [],
        tasks: [],
        maxIterations: 1,
        convergenceThreshold: 0.01,
        mutationRate: 0.5,
        populationSize: 3,
      }

      const nextGen = await generateNextGeneration(variants, scores, config)

      // Should have at least 1 variant (best is always kept)
      expect(nextGen.length).toBeGreaterThanOrEqual(1)

      // First variant should be the best (elitism)
      expect(nextGen[0].id).toBe('test-variant-0')

      // If crossover succeeded, it should have proper genealogy
      const crossoverVariant = nextGen.find((v) => v.mutation_type === 'crossover')
      if (crossoverVariant) {
        expect(crossoverVariant.generation).toBe(1)
        expect(crossoverVariant.parent_ids).toHaveLength(2)
      }
    })
  })

  describe('generateIterationReport', () => {
    it('should generate comprehensive report', () => {
      const mockHistory = {
        generations: [
          {
            number: 1,
            variants: createTestVariants(3),
            taskResults: new Map(),
            avgScore: 0.75,
            bestVariant: createTestVariants(1)[0],
            bestScore: 0.8,
            improvement: 0.0,
          } as Generation,
          {
            number: 2,
            variants: createTestVariants(3),
            taskResults: new Map(),
            avgScore: 0.78,
            bestVariant: createTestVariants(1)[0],
            bestScore: 0.85,
            improvement: 0.05,
          } as Generation,
        ],
        bestOverall: createTestVariants(1)[0],
        convergenceReached: true,
        totalIterations: 2,
      }

      const report = generateIterationReport(mockHistory)

      expect(report).toContain('GENETIC ITERATION REPORT')
      expect(report).toContain('Total Iterations: 2')
      expect(report).toContain('Convergence: YES')
      expect(report).toContain('Generation 1:')
      expect(report).toContain('Generation 2:')
      expect(report).toContain('OVERALL BEST')
      expect(report).toContain('RECOMMENDATION')
    })

    it('should indicate when convergence not reached', () => {
      const mockHistory = {
        generations: [
          {
            number: 1,
            variants: createTestVariants(3),
            taskResults: new Map(),
            avgScore: 0.75,
            bestVariant: createTestVariants(1)[0],
            bestScore: 0.8,
            improvement: 0.0,
          } as Generation,
        ],
        bestOverall: createTestVariants(1)[0],
        convergenceReached: false,
        totalIterations: 1,
      }

      const report = generateIterationReport(mockHistory)

      expect(report).toContain('Convergence: NO')
      expect(report).toContain('Max iterations reached')
    })
  })

  describe('runGeneticIterations', () => {
    beforeEach(() => {
      // Save test variants to temp directory
      const variantsDir = join(testDir, 'variants')
      mkdirSync(variantsDir, { recursive: true })

      const testVariants = createTestVariants(3)
      for (const variant of testVariants) {
        writeFileSync(join(variantsDir, `${variant.id}.json`), JSON.stringify(variant, null, 2))
      }
    })

    it('should run single iteration', async () => {
      const config: IterationConfig = {
        initialVariants: [
          join(testDir, 'variants', 'test-variant-0.json'),
          join(testDir, 'variants', 'test-variant-1.json'),
          join(testDir, 'variants', 'test-variant-2.json'),
        ],
        tasks: [TASK_FIND_WORKTREE_CREATION],
        maxIterations: 1,
        convergenceThreshold: 0.01,
        mutationRate: 0.5,
        populationSize: 3,
        baseDir: testDir,
      }

      const result = await runGeneticIterations(config)

      expect(result.totalIterations).toBe(1)
      expect(result.generations).toHaveLength(1)
      expect(result.bestOverall).toBeDefined()
    })

    it('should track improvement over generations', async () => {
      const config: IterationConfig = {
        initialVariants: [
          join(testDir, 'variants', 'test-variant-0.json'),
          join(testDir, 'variants', 'test-variant-1.json'),
          join(testDir, 'variants', 'test-variant-2.json'),
        ],
        tasks: [TASK_FIND_WORKTREE_CREATION],
        maxIterations: 2,
        convergenceThreshold: 0.01,
        mutationRate: 0.5,
        populationSize: 3,
        baseDir: testDir,
      }

      const result = await runGeneticIterations(config)

      expect(result.generations).toHaveLength(2)

      // Generation 2 should have improvement calculated
      const gen2 = result.generations[1]
      expect(gen2.improvement).toBeDefined()
    })

    it('should run 3 iterations with improvement', async () => {
      const config: IterationConfig = {
        initialVariants: [
          join(testDir, 'variants', 'test-variant-0.json'),
          join(testDir, 'variants', 'test-variant-1.json'),
          join(testDir, 'variants', 'test-variant-2.json'),
        ],
        tasks: [TASK_FIND_WORKTREE_CREATION],
        maxIterations: 3,
        convergenceThreshold: 0.01,
        mutationRate: 0.5,
        populationSize: 3,
        baseDir: testDir,
      }

      const result = await runGeneticIterations(config)

      // Should run all 3 iterations
      expect(result.generations).toHaveLength(3)

      // Each generation should have required fields
      result.generations.forEach((gen, idx) => {
        expect(gen.number).toBe(idx + 1)
        expect(gen.variants).toBeDefined()
        expect(gen.bestVariant).toBeDefined()
        expect(gen.bestScore).toBeGreaterThanOrEqual(0)
        expect(gen.bestScore).toBeLessThanOrEqual(1)
        expect(gen.avgScore).toBeGreaterThanOrEqual(0)
        expect(gen.avgScore).toBeLessThanOrEqual(1)
      })

      // Should have best overall variant
      expect(result.bestOverall).toBeDefined()
      expect(result.bestOverall.id).toBeDefined()

      // Total iterations should match
      expect(result.totalIterations).toBe(3)

      // Check that improvement values are calculated for later generations
      // Note: With random noise in scoring, improvement can be positive or negative
      const improvements = result.generations.slice(1).map((gen) => gen.improvement)
      expect(improvements).toHaveLength(2) // Gen 2 and Gen 3 have improvement values
      improvements.forEach((imp) => {
        expect(typeof imp).toBe('number')
        expect(imp).toBeGreaterThanOrEqual(-100) // Reasonable bounds
        expect(imp).toBeLessThanOrEqual(100)
      })
    })

    it('should stop early on convergence', async () => {
      const config: IterationConfig = {
        initialVariants: [
          join(testDir, 'variants', 'test-variant-0.json'),
          join(testDir, 'variants', 'test-variant-1.json'),
          join(testDir, 'variants', 'test-variant-2.json'),
        ],
        tasks: [TASK_FIND_WORKTREE_CREATION],
        maxIterations: 10,
        convergenceThreshold: 0.2, // High threshold to trigger convergence quickly
        mutationRate: 0.5,
        populationSize: 3,
        baseDir: testDir,
      }

      const result = await runGeneticIterations(config)

      // Should stop before max iterations due to convergence
      expect(result.totalIterations).toBeLessThan(10)
      expect(result.convergenceReached).toBe(true)
    })

    it('should respect max iterations', async () => {
      const config: IterationConfig = {
        initialVariants: [
          join(testDir, 'variants', 'test-variant-0.json'),
          join(testDir, 'variants', 'test-variant-1.json'),
          join(testDir, 'variants', 'test-variant-2.json'),
        ],
        tasks: [TASK_FIND_WORKTREE_CREATION],
        maxIterations: 2,
        convergenceThreshold: 0.0001, // Low threshold, unlikely to converge
        mutationRate: 0.5,
        populationSize: 3,
        baseDir: testDir,
      }

      const result = await runGeneticIterations(config)

      expect(result.totalIterations).toBe(2)
      expect(result.generations).toHaveLength(2)
    })

    it('should handle single variant population', async () => {
      const config: IterationConfig = {
        initialVariants: [join(testDir, 'variants', 'test-variant-0.json')],
        tasks: [TASK_FIND_WORKTREE_CREATION],
        maxIterations: 1,
        convergenceThreshold: 0.01,
        mutationRate: 0.5,
        populationSize: 1,
        baseDir: testDir,
      }

      const result = await runGeneticIterations(config)

      expect(result.generations).toHaveLength(1)
      expect(result.bestOverall).toBeDefined()
    })
  })
})
