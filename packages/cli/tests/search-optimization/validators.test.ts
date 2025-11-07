/**
 * Tests for search task validators
 */

import { describe, it, expect } from 'vitest'
import type {
  SearchResult,
  SearchTarget,
  WorkResult,
  TaskValidator,
  AgentOutput,
} from '../../src/search-optimization/types.js'
import {
  validateSearchQuality,
  validateTaskCompletion,
  calculateEfficiency,
  createTaskValidator,
} from '../../src/search-optimization/validators.js'

describe('validators', () => {
  describe('validateSearchQuality', () => {
    it('should return 1.0 when target found in top 3', () => {
      const searchResults: SearchResult[] = [
        {
          query: 'worktree creation',
          results: [
            { relpath: 'packages/cli/src/git/worktree.ts', content: 'worktree code' },
            { relpath: 'other.ts', content: 'other' },
          ],
        },
      ]

      const target: SearchTarget = {
        type: 'file',
        path: 'packages/cli/src/git/worktree.ts',
      }

      const score = validateSearchQuality(searchResults, target)
      expect(score).toBe(1.0)
    })

    it('should return 0.7 when target found in top 10', () => {
      const searchResults: SearchResult[] = [
        {
          query: 'worktree',
          results: Array(8)
            .fill(null)
            .map((_, i) => ({ relpath: `other${i}.ts`, content: 'other' }))
            .concat([{ relpath: 'packages/cli/src/git/worktree.ts', content: 'worktree code' }]),
        },
      ]

      const target: SearchTarget = {
        type: 'file',
        path: 'packages/cli/src/git/worktree.ts',
      }

      const score = validateSearchQuality(searchResults, target)
      expect(score).toBe(0.7)
    })

    it('should return 0.0 when target not found', () => {
      const searchResults: SearchResult[] = [
        {
          query: 'worktree',
          results: [
            { relpath: 'other.ts', content: 'other' },
            { relpath: 'another.ts', content: 'another' },
          ],
        },
      ]

      const target: SearchTarget = {
        type: 'file',
        path: 'packages/cli/src/git/worktree.ts',
      }

      const score = validateSearchQuality(searchResults, target)
      expect(score).toBe(0.0)
    })

    it('should match alternatives', () => {
      const searchResults: SearchResult[] = [
        {
          query: 'worktree',
          results: [{ relpath: 'packages/cli/src/git/worktrees.ts', content: 'worktree code' }],
        },
      ]

      const target: SearchTarget = {
        type: 'file',
        path: 'packages/cli/src/git/worktree.ts',
        alternatives: ['packages/cli/src/git/worktrees.ts'],
      }

      const score = validateSearchQuality(searchResults, target)
      expect(score).toBe(1.0)
    })

    it('should match pattern targets', () => {
      const searchResults: SearchResult[] = [
        {
          query: 'competition',
          results: [{ relpath: 'competition.ts', content: 'CompetitionManager class' }],
        },
      ]

      const target: SearchTarget = {
        type: 'pattern',
        pattern: /CompetitionManager/,
      }

      const score = validateSearchQuality(searchResults, target)
      expect(score).toBe(1.0)
    })
  })

  describe('validateTaskCompletion', () => {
    it('should validate explanation tasks', () => {
      const workResult: WorkResult = {
        explanationText: 'Worktree creation uses git worktree add to create branches',
        success: true,
      }

      const validator: TaskValidator = {
        type: 'explanation',
        mentionsFiles: ['worktree'],
        mentionsPattern: /git worktree add/i,
      }

      const score = validateTaskCompletion(workResult, validator)
      expect(score).toBe(1.0)
    })

    it('should validate code change tasks', () => {
      const workResult: WorkResult = {
        filesChanged: ['packages/cli/src/test.ts'],
        success: true,
      }

      const validator: TaskValidator = {
        type: 'code_change',
        fileChanged: 'packages/cli/src/test.ts',
      }

      const score = validateTaskCompletion(workResult, validator)
      expect(score).toBe(1.0)
    })

    it('should return 0 when task failed', () => {
      const workResult: WorkResult = {
        success: false,
      }

      const validator: TaskValidator = {
        type: 'explanation',
        mentionsPattern: /test/,
      }

      const score = validateTaskCompletion(workResult, validator)
      expect(score).toBe(0.0)
    })

    it('should return partial score when file changed but pattern not found', () => {
      const workResult: WorkResult = {
        filesChanged: ['test.ts'],
        success: true,
      }

      const validator: TaskValidator = {
        type: 'code_change',
        fileChanged: 'test.ts',
        containsPattern: /some pattern that does not exist/,
      }

      const score = validateTaskCompletion(workResult, validator)
      expect(score).toBeLessThan(1.0)
    })
  })

  describe('calculateEfficiency', () => {
    it('should return high score for efficient execution', () => {
      const score = calculateEfficiency(2, 10, 60)
      expect(score).toBeGreaterThan(0.7)
    })

    it('should return low score for inefficient execution', () => {
      const score = calculateEfficiency(15, 50, 400)
      expect(score).toBeLessThan(0.3)
    })

    it('should penalize too many searches', () => {
      const score1 = calculateEfficiency(2, 10, 60)
      const score2 = calculateEfficiency(10, 10, 60)
      expect(score1).toBeGreaterThan(score2)
    })

    it('should penalize too many tool calls', () => {
      const score1 = calculateEfficiency(5, 10, 60)
      const score2 = calculateEfficiency(5, 40, 60)
      expect(score1).toBeGreaterThan(score2)
    })

    it('should penalize long duration', () => {
      const score1 = calculateEfficiency(5, 10, 60)
      const score2 = calculateEfficiency(5, 10, 400)
      expect(score1).toBeGreaterThan(score2)
    })
  })

  describe('createTaskValidator', () => {
    it('should create a validator function that scores all components', () => {
      const task = {
        searchTarget: {
          type: 'file' as const,
          path: 'test.ts',
        },
        followUpTask: {
          validator: {
            type: 'explanation' as const,
            mentionsPattern: /test/,
          },
        },
      }

      const validator = createTaskValidator(task)

      const agentOutput: AgentOutput = {
        searchResults: [
          {
            query: 'test',
            results: [{ relpath: 'test.ts', content: 'test code' }],
          },
        ],
        workResult: {
          explanationText: 'This is a test explanation',
          success: true,
        },
        searchCount: 2,
        toolCallCount: 10,
        durationSeconds: 60,
      }

      const score = validator(agentOutput)

      expect(score.searchQuality).toBeGreaterThan(0)
      expect(score.taskCompletion).toBeGreaterThan(0)
      expect(score.efficiency).toBeGreaterThan(0)
      expect(score.total).toBeGreaterThan(0)
      expect(score.details).toBeTruthy()
    })

    it('should weight components correctly', () => {
      const task = {
        searchTarget: {
          type: 'file' as const,
          path: 'test.ts',
        },
        followUpTask: {
          validator: {
            type: 'explanation' as const,
            mentionsPattern: /test/,
          },
        },
      }

      const validator = createTaskValidator(task)

      const agentOutput: AgentOutput = {
        searchResults: [
          {
            query: 'test',
            results: [{ relpath: 'test.ts', content: 'test code' }],
          },
        ],
        workResult: {
          explanationText: 'This is a test explanation',
          success: true,
        },
        searchCount: 2,
        toolCallCount: 10,
        durationSeconds: 60,
      }

      const score = validator(agentOutput)

      // Total should be weighted: 0.4 * searchQuality + 0.4 * taskCompletion + 0.2 * efficiency
      const expectedTotal = score.searchQuality * 0.4 + score.taskCompletion * 0.4 + score.efficiency * 0.2

      expect(score.total).toBeCloseTo(expectedTotal, 2)
    })
  })
})
