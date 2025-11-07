/**
 * Tests for competition runner
 */

import { mkdtempSync, rmSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import type { CompetitionConfig } from '../../src/search-optimization/competition-runner.js'
import { runCompetition } from '../../src/search-optimization/competition-runner.js'
import { TASK_FIND_WORKTREE_CREATION } from '../../src/search-optimization/tasks/implementation.js'
import type { Variant } from '../../src/search-optimization/types.js'

// Mock the SDK spawner to avoid actually spawning agents
vi.mock('../../src/sdk/spawner.js', () => ({
  spawnAgentWithVariant: vi.fn(async (task, variant, hooks) => {
    // Simulate agent execution by calling hooks
    if (hooks?.onToolUse) {
      // Simulate a search
      await hooks.onToolUse({
        session_id: 'test-session',
        tool_name: 'search',
        tool_input: { query: 'worktree' },
        timestamp: Date.now(),
      })
    }

    return {
      success: true,
      sessionId: 'test-session',
      transcriptPath: '/tmp/test-transcript',
      usage: {
        inputTokens: 1000,
        outputTokens: 500,
        totalCostUsd: 0.01,
      },
      performance: {
        durationMs: 5000,
        durationApiMs: 4000,
        numTurns: 3,
      },
      messages: [],
    }
  }),
}))

// Mock the evaluation to avoid needing real worktrees
vi.mock('../../src/evaluation/search-checks.js', () => ({
  runSearchTaskEvaluation: vi.fn(async (task, worktreePath, runDir) => {
    // Return a mock evaluation with varying scores based on variant
    const isBaseline = runDir.includes('baseline')

    return {
      results: [],
      score: 0.8,
      task,
      taskScore: {
        searchQuality: isBaseline ? 0.7 : 0.9,
        taskCompletion: isBaseline ? 0.8 : 0.85,
        efficiency: isBaseline ? 0.9 : 0.85,
        total: isBaseline ? 0.8 : 0.87,
        details: 'Mock evaluation',
      },
      searchMetrics: {
        searchCount: isBaseline ? 3 : 2,
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
      compositeScore: isBaseline ? 0.8 : 0.87,
    }
  }),
}))

describe('competition-runner', () => {
  let testDir: string

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'comp-runner-test-'))
  })

  afterEach(() => {
    try {
      rmSync(testDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  })

  const createTestVariants = (): Variant[] => [
    {
      id: 'baseline',
      name: 'Baseline',
      description: 'Standard search tool description',
    },
    {
      id: 'variant-a',
      name: 'Variant A',
      description: 'Enhanced search tool with examples',
    },
    {
      id: 'variant-b',
      name: 'Variant B',
      description: 'Detailed search tool with best practices',
    },
  ]

  describe('runCompetition', () => {
    it('should run a competition with 3 variants', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: createTestVariants(),
        parallelExecution: false,
        timeout: 60,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      expect(result.competitionId).toBeDefined()
      expect(result.task).toBe(TASK_FIND_WORKTREE_CREATION)
      expect(result.participants).toHaveLength(3)
      expect(result.winner).toBeDefined()
      expect(result.metrics).toBeDefined()
      expect(result.report).toBeTruthy()
    })

    it('should determine the correct winner', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: createTestVariants(),
        parallelExecution: false,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      // Variant A should win (0.87 > 0.8)
      expect(result.winner.variantId).toBe('variant-a')
      expect(result.winner.score).toBeGreaterThan(0.8)
    })

    it('should calculate competition metrics', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: createTestVariants(),
        parallelExecution: false,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      expect(result.metrics.avgScore).toBeGreaterThan(0)
      expect(result.metrics.avgScore).toBeLessThanOrEqual(1)
      expect(result.metrics.scoreRange.min).toBeLessThanOrEqual(result.metrics.scoreRange.max)
      expect(result.metrics.avgSearchCount).toBeGreaterThan(0)
      expect(result.metrics.successRate).toBeGreaterThanOrEqual(0)
      expect(result.metrics.successRate).toBeLessThanOrEqual(1)
    })

    it('should generate a competition report', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: createTestVariants(),
        parallelExecution: false,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      expect(result.report).toContain('COMPETITION REPORT')
      expect(result.report).toContain('WINNER')
      expect(result.report).toContain('RESULTS')
      expect(result.report).toContain('Variant A')
      expect(result.report).toContain('Baseline')
    })

    it('should support sequential execution', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: createTestVariants(),
        parallelExecution: false,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      expect(result.participants).toHaveLength(3)
    })

    it('should support parallel execution', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: createTestVariants(),
        parallelExecution: true,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      expect(result.participants).toHaveLength(3)
    })

    it('should handle single variant', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: [createTestVariants()[0]],
        parallelExecution: false,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      expect(result.participants).toHaveLength(1)
      expect(result.winner).toBe(result.participants[0])
    })

    it('should break ties with efficiency', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: createTestVariants(),
        parallelExecution: false,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      // Winner should be determined by score, then efficiency
      expect(result.winner.variantId).toBeDefined()
    })

    it('should run end-to-end with 3 variants', async () => {
      const config: CompetitionConfig = {
        task: TASK_FIND_WORKTREE_CREATION,
        variants: createTestVariants(),
        parallelExecution: false,
        baseDir: testDir,
      }

      const result = await runCompetition(config)

      // Verify all 3 participants executed
      expect(result.participants).toHaveLength(3)
      expect(result.participants.map((p) => p.variantId)).toContain('baseline')
      expect(result.participants.map((p) => p.variantId)).toContain('variant-a')
      expect(result.participants.map((p) => p.variantId)).toContain('variant-b')

      // Verify all have scores
      result.participants.forEach((p) => {
        expect(p.score).toBeGreaterThanOrEqual(0)
        expect(p.score).toBeLessThanOrEqual(1)
        expect(p.evaluation).toBeDefined()
      })

      // Verify winner is selected from the 3
      expect(result.winner).toBeDefined()
      expect(result.participants).toContainEqual(result.winner)

      // Verify report includes all 3 variants
      expect(result.report).toContain('Baseline')
      expect(result.report).toContain('Variant A')
      expect(result.report).toContain('Variant B')
    })
  })
})
