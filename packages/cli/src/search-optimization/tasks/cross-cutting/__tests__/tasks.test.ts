/**
 * Tests for cross-cutting concerns tasks (Tier 2)
 */

import { describe, it, expect } from 'vitest'
import type { AgentOutput } from '../../../types.js'
import { TASK_ASYNC_ERROR_HANDLING, TASK_SECURITY_LOGGING, TASK_INPUT_VALIDATION } from '../index.js'

describe('Cross-Cutting Concerns Tasks', () => {
  describe('Task Structure Validation', () => {
    it('should have valid structure for async error handling task', () => {
      expect(TASK_ASYNC_ERROR_HANDLING.id).toBe('tier2-cross-cutting-async-errors')
      expect(TASK_ASYNC_ERROR_HANDLING.category).toBe('cross-cutting-concerns')
      expect(TASK_ASYNC_ERROR_HANDLING.difficulty).toBe('medium')
      expect(TASK_ASYNC_ERROR_HANDLING.expectedGrepSuccess).toBe(0.5)
      expect(TASK_ASYNC_ERROR_HANDLING.expectedSearchSuccess).toBe(0.85)
      expect(TASK_ASYNC_ERROR_HANDLING.searchTarget.type).toBe('pattern')
      expect(TASK_ASYNC_ERROR_HANDLING.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_ASYNC_ERROR_HANDLING.followUpTask.type).toBe('explanation')
      expect(TASK_ASYNC_ERROR_HANDLING.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for security logging task', () => {
      expect(TASK_SECURITY_LOGGING.id).toBe('tier2-cross-cutting-security-logging')
      expect(TASK_SECURITY_LOGGING.category).toBe('cross-cutting-concerns')
      expect(TASK_SECURITY_LOGGING.difficulty).toBe('medium')
      expect(TASK_SECURITY_LOGGING.expectedGrepSuccess).toBe(0.4)
      expect(TASK_SECURITY_LOGGING.expectedSearchSuccess).toBe(0.75)
      expect(TASK_SECURITY_LOGGING.searchTarget.type).toBe('pattern')
      expect(TASK_SECURITY_LOGGING.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_SECURITY_LOGGING.followUpTask.type).toBe('explanation')
      expect(TASK_SECURITY_LOGGING.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for input validation task', () => {
      expect(TASK_INPUT_VALIDATION.id).toBe('tier2-cross-cutting-input-validation')
      expect(TASK_INPUT_VALIDATION.category).toBe('cross-cutting-concerns')
      expect(TASK_INPUT_VALIDATION.difficulty).toBe('medium')
      expect(TASK_INPUT_VALIDATION.expectedGrepSuccess).toBe(0.45)
      expect(TASK_INPUT_VALIDATION.expectedSearchSuccess).toBe(0.8)
      expect(TASK_INPUT_VALIDATION.searchTarget.type).toBe('pattern')
      expect(TASK_INPUT_VALIDATION.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_INPUT_VALIDATION.followUpTask.type).toBe('explanation')
      expect(TASK_INPUT_VALIDATION.successValidator).toBeTypeOf('function')
    })
  })

  describe('Grep Success Rates', () => {
    it('should have grep success in tier 2 range (30-60%)', () => {
      expect(TASK_ASYNC_ERROR_HANDLING.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_ASYNC_ERROR_HANDLING.expectedGrepSuccess).toBeLessThanOrEqual(0.6)

      expect(TASK_SECURITY_LOGGING.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_SECURITY_LOGGING.expectedGrepSuccess).toBeLessThanOrEqual(0.6)

      expect(TASK_INPUT_VALIDATION.expectedGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(TASK_INPUT_VALIDATION.expectedGrepSuccess).toBeLessThanOrEqual(0.6)
    })
  })

  describe('Search Advantage', () => {
    it('should have >30% search advantage for all tasks', () => {
      const asyncErrorAdvantage =
        TASK_ASYNC_ERROR_HANDLING.expectedSearchSuccess - TASK_ASYNC_ERROR_HANDLING.expectedGrepSuccess
      expect(asyncErrorAdvantage).toBeGreaterThan(0.3)

      const securityAdvantage = TASK_SECURITY_LOGGING.expectedSearchSuccess - TASK_SECURITY_LOGGING.expectedGrepSuccess
      expect(securityAdvantage).toBeGreaterThan(0.3)

      const validationAdvantage =
        TASK_INPUT_VALIDATION.expectedSearchSuccess - TASK_INPUT_VALIDATION.expectedGrepSuccess
      expect(validationAdvantage).toBeGreaterThan(0.3)
    })
  })

  describe('Async Error Handling Task Validation', () => {
    it('should succeed when async error patterns are found across files', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'async error handling',
            results: [
              { relpath: 'src/git/merge.ts', content: 'async try catch await error' },
              { relpath: 'src/config/loader.ts', content: 'async catch promise rejection' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'Async error handling in merge.ts uses try/catch around await. ' +
            'The loader.ts handles promise rejections. ' +
            'Multiple files implement async error propagation.',
        },
        searchCount: 2,
        toolCallCount: 6,
        durationSeconds: 35,
      }

      const score = TASK_ASYNC_ERROR_HANDLING.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.6)
      expect(score.taskCompletion).toBeGreaterThan(0.7)
    })

    it('should fail when only synchronous error handling is found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [{ query: 'error', results: [] }],
        workResult: {
          success: false,
          explanationText: 'Found some synchronous try/catch blocks but no async error handling.',
        },
        searchCount: 1,
        toolCallCount: 3,
        durationSeconds: 20,
      }

      const score = TASK_ASYNC_ERROR_HANDLING.successValidator(mockOutput)
      expect(score.taskCompletion).toBe(0)
    })
  })

  describe('Security Logging Task Validation', () => {
    it('should succeed when security logging patterns are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'security logging',
            results: [{ relpath: 'src/config/loader.ts', content: 'logger warn error auth security' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The loader.ts logs security-related events. ' +
            'Authentication and access control events are logged for audit trail.',
        },
        searchCount: 2,
        toolCallCount: 5,
        durationSeconds: 30,
      }

      const score = TASK_SECURITY_LOGGING.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })

    it('should fail when only general logging is found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [{ query: 'log', results: [] }],
        workResult: {
          success: false,
          explanationText: 'Found logging but nothing security-related.',
        },
        searchCount: 1,
        toolCallCount: 3,
        durationSeconds: 20,
      }

      const score = TASK_SECURITY_LOGGING.successValidator(mockOutput)
      expect(score.taskCompletion).toBe(0)
    })
  })

  describe('Input Validation Task Validation', () => {
    it('should succeed when validation patterns are found across files', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'input validation',
            results: [
              { relpath: 'src/config/loader.ts', content: 'validation schema parse safeParse' },
              { relpath: 'src/config/schema.ts', content: 'Zod schema validation' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The loader.ts validates input using schema parsing. ' +
            'The schema.ts defines validation schemas. ' +
            'Multiple validation approaches are used across the codebase.',
        },
        searchCount: 2,
        toolCallCount: 6,
        durationSeconds: 35,
      }

      const score = TASK_INPUT_VALIDATION.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.7)
      expect(score.taskCompletion).toBeGreaterThan(0.8)
    })
  })

  describe('Integration Tests', () => {
    it('should have all tasks with required fields', () => {
      const tasks = [TASK_ASYNC_ERROR_HANDLING, TASK_SECURITY_LOGGING, TASK_INPUT_VALIDATION]

      for (const task of tasks) {
        expect(task.id).toBeTruthy()
        expect(task.name).toBeTruthy()
        expect(task.description).toBeTruthy()
        expect(task.category).toBe('cross-cutting-concerns')
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
      expect(TASK_ASYNC_ERROR_HANDLING.followUpTask.validator.mentionsFiles).toBeTruthy()
      expect(TASK_SECURITY_LOGGING.followUpTask.validator.mentionsFiles).toBeTruthy()
      expect(TASK_INPUT_VALIDATION.followUpTask.validator.mentionsFiles).toBeTruthy()
    })

    it('should have pattern validators', () => {
      expect(TASK_ASYNC_ERROR_HANDLING.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
      expect(TASK_SECURITY_LOGGING.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
      expect(TASK_INPUT_VALIDATION.followUpTask.validator.mentionsPattern).toBeInstanceOf(RegExp)
    })

    it('should demonstrate cross-cutting concern aggregation', () => {
      // Each task should handle scattered patterns
      expect(TASK_ASYNC_ERROR_HANDLING.internalNotes).toContain('scattered')
      expect(TASK_SECURITY_LOGGING.internalNotes).toContain('scattered')
      expect(TASK_INPUT_VALIDATION.internalNotes).toContain('scattered')
    })

    it('should require multiple files in validation', () => {
      // Cross-cutting concerns should mention multiple files
      expect(TASK_ASYNC_ERROR_HANDLING.followUpTask.validator.mentionsFiles?.length).toBeGreaterThanOrEqual(2)
      expect(TASK_INPUT_VALIDATION.followUpTask.validator.mentionsFiles?.length).toBeGreaterThanOrEqual(2)
    })
  })
})
