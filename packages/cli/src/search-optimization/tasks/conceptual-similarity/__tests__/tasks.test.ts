/**
 * Tests for conceptual similarity tasks (Tier 2)
 */

import { describe, it, expect } from 'vitest'
import type { AgentOutput } from '../../../types.js'
import {
  TASK_RETRY_IMPLEMENTATIONS,
  TASK_ERROR_HANDLING_PATTERNS,
  TASK_RATE_LIMITING,
  TASK_CACHING_STRATEGIES,
} from '../index.js'

describe('Conceptual Similarity Tasks', () => {
  describe('Task Structure Validation', () => {
    it('should have valid structure for retry task', () => {
      expect(TASK_RETRY_IMPLEMENTATIONS.id).toBe('tier2-conceptual-retry')
      expect(TASK_RETRY_IMPLEMENTATIONS.category).toBe('conceptual-similarity')
      expect(TASK_RETRY_IMPLEMENTATIONS.difficulty).toBe('medium')
      expect(TASK_RETRY_IMPLEMENTATIONS.expectedGrepSuccess).toBe(0.45)
      expect(TASK_RETRY_IMPLEMENTATIONS.expectedSearchSuccess).toBe(0.8)
      expect(TASK_RETRY_IMPLEMENTATIONS.searchTarget.type).toBe('pattern')
      expect(TASK_RETRY_IMPLEMENTATIONS.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_RETRY_IMPLEMENTATIONS.followUpTask.type).toBe('explanation')
      expect(TASK_RETRY_IMPLEMENTATIONS.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for error handling task', () => {
      expect(TASK_ERROR_HANDLING_PATTERNS.id).toBe('tier2-conceptual-error-handling')
      expect(TASK_ERROR_HANDLING_PATTERNS.category).toBe('conceptual-similarity')
      expect(TASK_ERROR_HANDLING_PATTERNS.difficulty).toBe('medium')
      expect(TASK_ERROR_HANDLING_PATTERNS.expectedGrepSuccess).toBe(0.5)
      expect(TASK_ERROR_HANDLING_PATTERNS.expectedSearchSuccess).toBe(0.85)
      expect(TASK_ERROR_HANDLING_PATTERNS.searchTarget.type).toBe('pattern')
      expect(TASK_ERROR_HANDLING_PATTERNS.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_ERROR_HANDLING_PATTERNS.followUpTask.type).toBe('explanation')
      expect(TASK_ERROR_HANDLING_PATTERNS.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for rate limiting task', () => {
      expect(TASK_RATE_LIMITING.id).toBe('tier2-conceptual-rate-limiting')
      expect(TASK_RATE_LIMITING.category).toBe('conceptual-similarity')
      expect(TASK_RATE_LIMITING.difficulty).toBe('medium')
      expect(TASK_RATE_LIMITING.expectedGrepSuccess).toBe(0.4)
      expect(TASK_RATE_LIMITING.expectedSearchSuccess).toBe(0.75)
      expect(TASK_RATE_LIMITING.searchTarget.type).toBe('pattern')
      expect(TASK_RATE_LIMITING.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_RATE_LIMITING.followUpTask.type).toBe('explanation')
      expect(TASK_RATE_LIMITING.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for caching task', () => {
      expect(TASK_CACHING_STRATEGIES.id).toBe('tier2-conceptual-caching')
      expect(TASK_CACHING_STRATEGIES.category).toBe('conceptual-similarity')
      expect(TASK_CACHING_STRATEGIES.difficulty).toBe('medium')
      expect(TASK_CACHING_STRATEGIES.expectedGrepSuccess).toBe(0.35)
      expect(TASK_CACHING_STRATEGIES.expectedSearchSuccess).toBe(0.8)
      expect(TASK_CACHING_STRATEGIES.searchTarget.type).toBe('pattern')
      expect(TASK_CACHING_STRATEGIES.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_CACHING_STRATEGIES.followUpTask.type).toBe('explanation')
      expect(TASK_CACHING_STRATEGIES.successValidator).toBeTypeOf('function')
    })
  })

  describe('Grep Success Rates', () => {
    it('should have grep success in tier 2 range (30-60%)', () => {
      expect(TASK_RETRY_IMPLEMENTATIONS.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_RETRY_IMPLEMENTATIONS.expectedGrepSuccess).toBeLessThanOrEqual(0.6)

      expect(TASK_ERROR_HANDLING_PATTERNS.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_ERROR_HANDLING_PATTERNS.expectedGrepSuccess).toBeLessThanOrEqual(0.6)

      expect(TASK_RATE_LIMITING.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_RATE_LIMITING.expectedGrepSuccess).toBeLessThanOrEqual(0.6)

      expect(TASK_CACHING_STRATEGIES.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_CACHING_STRATEGIES.expectedGrepSuccess).toBeLessThanOrEqual(0.6)
    })
  })

  describe('Search Advantage', () => {
    it('should have >30% search advantage for all tasks', () => {
      const retryAdvantage =
        TASK_RETRY_IMPLEMENTATIONS.expectedSearchSuccess - TASK_RETRY_IMPLEMENTATIONS.expectedGrepSuccess
      expect(retryAdvantage).toBeGreaterThan(0.3)

      const errorAdvantage =
        TASK_ERROR_HANDLING_PATTERNS.expectedSearchSuccess - TASK_ERROR_HANDLING_PATTERNS.expectedGrepSuccess
      expect(errorAdvantage).toBeGreaterThan(0.3)

      const rateLimitAdvantage = TASK_RATE_LIMITING.expectedSearchSuccess - TASK_RATE_LIMITING.expectedGrepSuccess
      expect(rateLimitAdvantage).toBeGreaterThan(0.3)

      const cacheAdvantage = TASK_CACHING_STRATEGIES.expectedSearchSuccess - TASK_CACHING_STRATEGIES.expectedGrepSuccess
      expect(cacheAdvantage).toBeGreaterThan(0.3)
    })
  })

  describe('Retry Task Validation', () => {
    it('should succeed when retry patterns are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'retry logic',
            results: [{ relpath: 'src/bus/message.bus.ts', content: 'waitForResponse timeout retry' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The message.bus.ts implements retry logic with timeout. ' +
            'It uses a timeout mechanism to retry operations that fail.',
        },
        searchCount: 2,
        toolCallCount: 5,
        durationSeconds: 30,
      }

      const score = TASK_RETRY_IMPLEMENTATIONS.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })

    it('should fail when no retry patterns are mentioned', () => {
      const mockOutput: AgentOutput = {
        searchResults: [{ query: 'code', results: [] }],
        workResult: {
          success: false,
          explanationText: 'Some unrelated code without retry logic.',
        },
        searchCount: 1,
        toolCallCount: 3,
        durationSeconds: 20,
      }

      const score = TASK_RETRY_IMPLEMENTATIONS.successValidator(mockOutput)
      expect(score.taskCompletion).toBe(0)
    })
  })

  describe('Error Handling Task Validation', () => {
    it('should succeed when error patterns are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'error handling',
            results: [
              { relpath: 'src/git/merge.ts', content: 'try catch error handling' },
              { relpath: 'src/config/loader.ts', content: 'throw Error validation' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'Error handling in merge.ts uses try/catch blocks. ' +
            'The loader.ts file throws errors for validation failures. ' +
            'Different error propagation strategies are used.',
        },
        searchCount: 2,
        toolCallCount: 6,
        durationSeconds: 35,
      }

      const score = TASK_ERROR_HANDLING_PATTERNS.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })
  })

  describe('Rate Limiting Task Validation', () => {
    it('should succeed when flow control patterns are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'rate limiting flow control',
            results: [{ relpath: 'src/bus/message.bus.ts', content: 'timeout delay waitFor' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The message bus implements flow control with timeout mechanisms. ' +
            'This provides backpressure by limiting concurrent operations.',
        },
        searchCount: 3,
        toolCallCount: 7,
        durationSeconds: 40,
      }

      const score = TASK_RATE_LIMITING.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })
  })

  describe('Caching Task Validation', () => {
    it('should succeed when caching patterns are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'caching memoization',
            results: [{ relpath: 'src/config/loader.ts', content: 'config caching load once' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The loader.ts implements caching by loading config once. ' +
            'This is a lazy initialization pattern that caches the result.',
        },
        searchCount: 2,
        toolCallCount: 5,
        durationSeconds: 30,
      }

      const score = TASK_CACHING_STRATEGIES.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })
  })

  describe('Integration Tests', () => {
    it('should have all tasks with required fields', () => {
      const tasks = [
        TASK_RETRY_IMPLEMENTATIONS,
        TASK_ERROR_HANDLING_PATTERNS,
        TASK_RATE_LIMITING,
        TASK_CACHING_STRATEGIES,
      ]

      for (const task of tasks) {
        expect(task.id).toBeTruthy()
        expect(task.name).toBeTruthy()
        expect(task.description).toBeTruthy()
        expect(task.category).toBe('conceptual-similarity')
        expect(task.difficulty).toBe('medium')
        expect(task.internalNotes).toBeTruthy()
        expect(task.expectedGrepSuccess).toBeGreaterThan(0)
        expect(task.expectedSearchSuccess).toBeGreaterThan(0)
        expect(task.searchTarget).toBeTruthy()
        expect(task.followUpTask).toBeTruthy()
        expect(task.successValidator).toBeTypeOf('function')
      }
    })

    it('should have expected files defined', () => {
      expect(TASK_RETRY_IMPLEMENTATIONS.followUpTask.validator.mentionsFiles).toBeTruthy()
      expect(TASK_ERROR_HANDLING_PATTERNS.followUpTask.validator.mentionsFiles).toBeTruthy()
      expect(TASK_RATE_LIMITING.followUpTask.validator.mentionsFiles).toBeTruthy()
      expect(TASK_CACHING_STRATEGIES.followUpTask.validator.mentionsFiles).toBeTruthy()
    })

    it('should have pattern validators', () => {
      expect(TASK_RETRY_IMPLEMENTATIONS.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
      expect(TASK_ERROR_HANDLING_PATTERNS.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
      expect(TASK_RATE_LIMITING.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
      expect(TASK_CACHING_STRATEGIES.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
    })
  })
})
