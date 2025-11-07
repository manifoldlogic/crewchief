/**
 * Tests for ambiguity resolution tasks (Tier 2)
 */

import { describe, it, expect } from 'vitest'
import type { AgentOutput } from '../../../types.js'
import {
  TASK_TRANSACTION_MANAGEMENT,
  TASK_AUTHENTICATION_CHECKS,
  TASK_RESOURCE_CLEANUP,
  TASK_CACHE_OPERATIONS,
} from '../index.js'

describe('Ambiguity Resolution Tasks', () => {
  describe('Task Structure Validation', () => {
    it('should have valid structure for transaction task', () => {
      expect(TASK_TRANSACTION_MANAGEMENT.id).toBe('tier2-ambiguity-transaction')
      expect(TASK_TRANSACTION_MANAGEMENT.category).toBe('ambiguity-resolution')
      expect(TASK_TRANSACTION_MANAGEMENT.difficulty).toBe('medium')
      expect(TASK_TRANSACTION_MANAGEMENT.expectedGrepSuccess).toBe(0.45)
      expect(TASK_TRANSACTION_MANAGEMENT.expectedSearchSuccess).toBe(0.8)
      expect(TASK_TRANSACTION_MANAGEMENT.searchTarget.type).toBe('pattern')
      expect(TASK_TRANSACTION_MANAGEMENT.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_TRANSACTION_MANAGEMENT.followUpTask.type).toBe('explanation')
      expect(TASK_TRANSACTION_MANAGEMENT.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for authentication task', () => {
      expect(TASK_AUTHENTICATION_CHECKS.id).toBe('tier2-ambiguity-auth')
      expect(TASK_AUTHENTICATION_CHECKS.category).toBe('ambiguity-resolution')
      expect(TASK_AUTHENTICATION_CHECKS.difficulty).toBe('medium')
      expect(TASK_AUTHENTICATION_CHECKS.expectedGrepSuccess).toBe(0.4)
      expect(TASK_AUTHENTICATION_CHECKS.expectedSearchSuccess).toBe(0.75)
      expect(TASK_AUTHENTICATION_CHECKS.searchTarget.type).toBe('pattern')
      expect(TASK_AUTHENTICATION_CHECKS.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_AUTHENTICATION_CHECKS.followUpTask.type).toBe('explanation')
      expect(TASK_AUTHENTICATION_CHECKS.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for cleanup task', () => {
      expect(TASK_RESOURCE_CLEANUP.id).toBe('tier2-ambiguity-cleanup')
      expect(TASK_RESOURCE_CLEANUP.category).toBe('ambiguity-resolution')
      expect(TASK_RESOURCE_CLEANUP.difficulty).toBe('medium')
      expect(TASK_RESOURCE_CLEANUP.expectedGrepSuccess).toBe(0.5)
      expect(TASK_RESOURCE_CLEANUP.expectedSearchSuccess).toBe(0.8)
      expect(TASK_RESOURCE_CLEANUP.searchTarget.type).toBe('pattern')
      expect(TASK_RESOURCE_CLEANUP.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_RESOURCE_CLEANUP.followUpTask.type).toBe('explanation')
      expect(TASK_RESOURCE_CLEANUP.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for cache operations task', () => {
      expect(TASK_CACHE_OPERATIONS.id).toBe('tier2-ambiguity-cache-ops')
      expect(TASK_CACHE_OPERATIONS.category).toBe('ambiguity-resolution')
      expect(TASK_CACHE_OPERATIONS.difficulty).toBe('medium')
      expect(TASK_CACHE_OPERATIONS.expectedGrepSuccess).toBe(0.35)
      expect(TASK_CACHE_OPERATIONS.expectedSearchSuccess).toBe(0.75)
      expect(TASK_CACHE_OPERATIONS.searchTarget.type).toBe('pattern')
      expect(TASK_CACHE_OPERATIONS.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_CACHE_OPERATIONS.followUpTask.type).toBe('explanation')
      expect(TASK_CACHE_OPERATIONS.successValidator).toBeTypeOf('function')
    })
  })

  describe('Grep Success Rates', () => {
    it('should have grep success in tier 2 range (30-60%)', () => {
      expect(TASK_TRANSACTION_MANAGEMENT.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_TRANSACTION_MANAGEMENT.expectedGrepSuccess).toBeLessThanOrEqual(0.6)

      expect(TASK_AUTHENTICATION_CHECKS.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_AUTHENTICATION_CHECKS.expectedGrepSuccess).toBeLessThanOrEqual(0.6)

      expect(TASK_RESOURCE_CLEANUP.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_RESOURCE_CLEANUP.expectedGrepSuccess).toBeLessThanOrEqual(0.6)

      expect(TASK_CACHE_OPERATIONS.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_CACHE_OPERATIONS.expectedGrepSuccess).toBeLessThanOrEqual(0.6)
    })
  })

  describe('Search Advantage', () => {
    it('should have >30% search advantage for all tasks', () => {
      const transactionAdvantage =
        TASK_TRANSACTION_MANAGEMENT.expectedSearchSuccess - TASK_TRANSACTION_MANAGEMENT.expectedGrepSuccess
      expect(transactionAdvantage).toBeGreaterThan(0.3)

      const authAdvantage =
        TASK_AUTHENTICATION_CHECKS.expectedSearchSuccess - TASK_AUTHENTICATION_CHECKS.expectedGrepSuccess
      expect(authAdvantage).toBeGreaterThan(0.3)

      const cleanupAdvantage = TASK_RESOURCE_CLEANUP.expectedSearchSuccess - TASK_RESOURCE_CLEANUP.expectedGrepSuccess
      expect(cleanupAdvantage).toBeGreaterThan(0.3)

      const cacheOpsAdvantage = TASK_CACHE_OPERATIONS.expectedSearchSuccess - TASK_CACHE_OPERATIONS.expectedGrepSuccess
      expect(cacheOpsAdvantage).toBeGreaterThan(0.3)
    })
  })

  describe('Transaction Task Validation', () => {
    it('should succeed when transaction patterns are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'transaction management',
            results: [{ relpath: 'src/git/merge.ts', content: 'transaction commit rollback atomic' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The merge.ts implements transaction-like patterns with commit and rollback. ' +
            'Atomic operations ensure data consistency.',
        },
        searchCount: 2,
        toolCallCount: 5,
        durationSeconds: 30,
      }

      const score = TASK_TRANSACTION_MANAGEMENT.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })

    it('should fail when only mentions found without implementation', () => {
      const mockOutput: AgentOutput = {
        searchResults: [{ query: 'transaction', results: [] }],
        workResult: {
          success: false,
          explanationText: 'Found some comments mentioning transactions but no actual code.',
        },
        searchCount: 1,
        toolCallCount: 3,
        durationSeconds: 20,
      }

      const score = TASK_TRANSACTION_MANAGEMENT.successValidator(mockOutput)
      expect(score.taskCompletion).toBe(0)
    })
  })

  describe('Authentication Task Validation', () => {
    it('should succeed when auth check patterns are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'authentication checks',
            results: [{ relpath: 'src/config/loader.ts', content: 'validation check auth verify' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The loader.ts performs authentication-related verification. ' +
            'It checks and guards access to protected resources.',
        },
        searchCount: 2,
        toolCallCount: 6,
        durationSeconds: 35,
      }

      const score = TASK_AUTHENTICATION_CHECKS.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })
  })

  describe('Resource Cleanup Task Validation', () => {
    it('should succeed when cleanup patterns are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'resource cleanup',
            results: [
              { relpath: 'src/bus/message.bus.ts', content: 'cleanup clearTimeout off' },
              { relpath: 'src/git/merge.ts', content: 'finally cleanup release' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The message.bus.ts implements resource cleanup with clearTimeout and event listener removal. ' +
            'The merge.ts uses finally blocks for cleanup.',
        },
        searchCount: 2,
        toolCallCount: 6,
        durationSeconds: 35,
      }

      const score = TASK_RESOURCE_CLEANUP.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.6)
      expect(score.taskCompletion).toBeGreaterThan(0.7)
    })
  })

  describe('Cache Operations Task Validation', () => {
    it('should succeed when cache operation types are distinguished', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'cache operations',
            results: [{ relpath: 'src/config/loader.ts', content: 'cache get set invalidate' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The loader.ts performs cache read operations with get. ' +
            'Cache write operations use set. Cache invalidation clears entries.',
        },
        searchCount: 2,
        toolCallCount: 5,
        durationSeconds: 30,
      }

      const score = TASK_CACHE_OPERATIONS.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })
  })

  describe('Integration Tests', () => {
    it('should have all tasks with required fields', () => {
      const tasks = [
        TASK_TRANSACTION_MANAGEMENT,
        TASK_AUTHENTICATION_CHECKS,
        TASK_RESOURCE_CLEANUP,
        TASK_CACHE_OPERATIONS,
      ]

      for (const task of tasks) {
        expect(task.id).toBeTruthy()
        expect(task.name).toBeTruthy()
        expect(task.description).toBeTruthy()
        expect(task.category).toBe('ambiguity-resolution')
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
      expect(TASK_TRANSACTION_MANAGEMENT.followUpTask.validator.mentionsFiles).toBeTruthy()
      expect(TASK_AUTHENTICATION_CHECKS.followUpTask.validator.mentionsFiles).toBeTruthy()
      expect(TASK_RESOURCE_CLEANUP.followUpTask.validator.mentionsFiles).toBeTruthy()
      expect(TASK_CACHE_OPERATIONS.followUpTask.validator.mentionsFiles).toBeTruthy()
    })

    it('should have pattern validators', () => {
      expect(TASK_TRANSACTION_MANAGEMENT.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
      expect(TASK_AUTHENTICATION_CHECKS.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
      expect(TASK_RESOURCE_CLEANUP.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
      expect(TASK_CACHE_OPERATIONS.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
    })

    it('should demonstrate ambiguity resolution capability', () => {
      // Each task should handle keywords with multiple meanings
      expect(TASK_TRANSACTION_MANAGEMENT.internalNotes).toContain('ambiguity')
      expect(TASK_AUTHENTICATION_CHECKS.internalNotes).toContain('ambiguity')
      expect(TASK_RESOURCE_CLEANUP.internalNotes).toContain('ambiguity')
      expect(TASK_CACHE_OPERATIONS.internalNotes).toContain('ambiguity')
    })
  })
})
