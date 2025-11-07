/**
 * Tests for relationship discovery tasks
 */

import { describe, it, expect } from 'vitest'
import type { AgentOutput } from '../../../types.js'
import { TASK_TRANSITIVE_DEPENDENCIES, TASK_CALL_CHAIN_TRACING, TASK_API_IMPACT_ANALYSIS } from '../index.js'

describe('Relationship Discovery Tasks', () => {
  describe('Task Structure Validation', () => {
    it('TASK_TRANSITIVE_DEPENDENCIES has all required fields', () => {
      expect(TASK_TRANSITIVE_DEPENDENCIES.id).toBe('relationship-transitive-deps')
      expect(TASK_TRANSITIVE_DEPENDENCIES.name).toBeTruthy()
      expect(TASK_TRANSITIVE_DEPENDENCIES.description).toBeTruthy()
      expect(TASK_TRANSITIVE_DEPENDENCIES.category).toBe('relationship-discovery')
      expect(TASK_TRANSITIVE_DEPENDENCIES.difficulty).toBe('hard')
      expect(TASK_TRANSITIVE_DEPENDENCIES.searchTarget).toBeDefined()
      expect(TASK_TRANSITIVE_DEPENDENCIES.followUpTask).toBeDefined()
      expect(TASK_TRANSITIVE_DEPENDENCIES.successValidator).toBeTypeOf('function')
      expect(TASK_TRANSITIVE_DEPENDENCIES.maxSearchAttempts).toBe(10)
      expect(TASK_TRANSITIVE_DEPENDENCIES.maxTimeSeconds).toBe(300)
    })

    it('TASK_CALL_CHAIN_TRACING has all required fields', () => {
      expect(TASK_CALL_CHAIN_TRACING.id).toBe('relationship-call-chain')
      expect(TASK_CALL_CHAIN_TRACING.name).toBeTruthy()
      expect(TASK_CALL_CHAIN_TRACING.description).toBeTruthy()
      expect(TASK_CALL_CHAIN_TRACING.category).toBe('relationship-discovery')
      expect(TASK_CALL_CHAIN_TRACING.difficulty).toBe('hard')
      expect(TASK_CALL_CHAIN_TRACING.searchTarget).toBeDefined()
      expect(TASK_CALL_CHAIN_TRACING.followUpTask).toBeDefined()
      expect(TASK_CALL_CHAIN_TRACING.successValidator).toBeTypeOf('function')
      expect(TASK_CALL_CHAIN_TRACING.maxSearchAttempts).toBe(10)
      expect(TASK_CALL_CHAIN_TRACING.maxTimeSeconds).toBe(300)
    })

    it('TASK_API_IMPACT_ANALYSIS has all required fields', () => {
      expect(TASK_API_IMPACT_ANALYSIS.id).toBe('relationship-impact-analysis')
      expect(TASK_API_IMPACT_ANALYSIS.name).toBeTruthy()
      expect(TASK_API_IMPACT_ANALYSIS.description).toBeTruthy()
      expect(TASK_API_IMPACT_ANALYSIS.category).toBe('relationship-discovery')
      expect(TASK_API_IMPACT_ANALYSIS.difficulty).toBe('hard')
      expect(TASK_API_IMPACT_ANALYSIS.searchTarget).toBeDefined()
      expect(TASK_API_IMPACT_ANALYSIS.followUpTask).toBeDefined()
      expect(TASK_API_IMPACT_ANALYSIS.successValidator).toBeTypeOf('function')
      expect(TASK_API_IMPACT_ANALYSIS.maxSearchAttempts).toBe(10)
      expect(TASK_API_IMPACT_ANALYSIS.maxTimeSeconds).toBe(300)
    })
  })

  describe('Expected Success Rates', () => {
    it('all tasks expect low grep success (<=30%)', () => {
      expect(TASK_TRANSITIVE_DEPENDENCIES.expectedGrepSuccess).toBeLessThanOrEqual(0.3)
      expect(TASK_CALL_CHAIN_TRACING.expectedGrepSuccess).toBeLessThanOrEqual(0.3)
      expect(TASK_API_IMPACT_ANALYSIS.expectedGrepSuccess).toBeLessThanOrEqual(0.3)
    })

    it('all tasks expect high search success (>70%)', () => {
      expect(TASK_TRANSITIVE_DEPENDENCIES.expectedSearchSuccess).toBeGreaterThan(0.7)
      expect(TASK_CALL_CHAIN_TRACING.expectedSearchSuccess).toBeGreaterThan(0.7)
      expect(TASK_API_IMPACT_ANALYSIS.expectedSearchSuccess).toBeGreaterThan(0.7)
    })
  })

  describe('Search Targets', () => {
    it('TASK_TRANSITIVE_DEPENDENCIES has pattern search target', () => {
      expect(TASK_TRANSITIVE_DEPENDENCIES.searchTarget.type).toBe('pattern')
      expect(TASK_TRANSITIVE_DEPENDENCIES.searchTarget.pattern).toBeInstanceOf(RegExp)
      // Should match key dependency chain components
      expect(TASK_TRANSITIVE_DEPENDENCIES.searchTarget.pattern?.test('WorktreeService')).toBe(true)
      expect(TASK_TRANSITIVE_DEPENDENCIES.searchTarget.pattern?.test('createWorktree')).toBe(true)
      expect(TASK_TRANSITIVE_DEPENDENCIES.searchTarget.pattern?.test('Scheduler')).toBe(true)
    })

    it('TASK_CALL_CHAIN_TRACING has pattern search target', () => {
      expect(TASK_CALL_CHAIN_TRACING.searchTarget.type).toBe('pattern')
      expect(TASK_CALL_CHAIN_TRACING.searchTarget.pattern).toBeInstanceOf(RegExp)
      // Should match call chain components
      expect(TASK_CALL_CHAIN_TRACING.searchTarget.pattern?.test('worktree create')).toBe(true)
      expect(TASK_CALL_CHAIN_TRACING.searchTarget.pattern?.test('Command worktree')).toBe(true)
      expect(TASK_CALL_CHAIN_TRACING.searchTarget.pattern?.test('WorktreeService')).toBe(true)
    })

    it('TASK_API_IMPACT_ANALYSIS has pattern search target', () => {
      expect(TASK_API_IMPACT_ANALYSIS.searchTarget.type).toBe('pattern')
      expect(TASK_API_IMPACT_ANALYSIS.searchTarget.pattern).toBeInstanceOf(RegExp)
      // Should match API usage patterns
      expect(TASK_API_IMPACT_ANALYSIS.searchTarget.pattern?.test('createWorktree')).toBe(true)
      expect(TASK_API_IMPACT_ANALYSIS.searchTarget.pattern?.test('wt.createWorktree')).toBe(true)
      expect(TASK_API_IMPACT_ANALYSIS.searchTarget.pattern?.test('mock createWorktree')).toBe(true)
    })
  })

  describe('Follow-up Task Validators', () => {
    it('TASK_TRANSITIVE_DEPENDENCIES validator requires dependency chain files', () => {
      const validator = TASK_TRANSITIVE_DEPENDENCIES.followUpTask.validator
      expect(validator.type).toBe('explanation')
      expect(validator.mentionsFiles).toContain('worktrees.ts')
      expect(validator.mentionsFiles).toContain('scheduler.ts')
      expect(validator.mentionsPattern).toBeInstanceOf(RegExp)
      // Should match dependency-related terms
      expect(validator.mentionsPattern?.test('depends on createWorktree')).toBe(true)
      expect(validator.mentionsPattern?.test('calls createWorktree')).toBe(true)
      expect(validator.mentionsPattern?.test('transitive dependencies')).toBe(true)
    })

    it('TASK_CALL_CHAIN_TRACING validator requires flow description', () => {
      const validator = TASK_CALL_CHAIN_TRACING.followUpTask.validator
      expect(validator.type).toBe('explanation')
      expect(validator.mentionsFiles).toContain('worktree.ts')
      expect(validator.mentionsFiles).toContain('worktrees.ts')
      expect(validator.mentionsPattern).toBeInstanceOf(RegExp)
      // Should match flow-related terms
      expect(validator.mentionsPattern?.test('CLI command')).toBe(true)
      expect(validator.mentionsPattern?.test('handler action')).toBe(true)
      expect(validator.mentionsPattern?.test('git raw')).toBe(true)
      expect(validator.mentionsPattern?.test('flow sequence')).toBe(true)
    })

    it('TASK_API_IMPACT_ANALYSIS validator requires impact description', () => {
      const validator = TASK_API_IMPACT_ANALYSIS.followUpTask.validator
      expect(validator.type).toBe('explanation')
      expect(validator.mentionsFiles).toContain('scheduler.ts')
      expect(validator.mentionsFiles).toContain('worktree.ts')
      expect(validator.mentionsPattern).toBeInstanceOf(RegExp)
      // Should match impact-related terms
      expect(validator.mentionsPattern?.test('impact of changes')).toBe(true)
      expect(validator.mentionsPattern?.test('test mock')).toBe(true)
      expect(validator.mentionsPattern?.test('API signature')).toBe(true)
    })
  })

  describe('Success Validator Functions', () => {
    it('TASK_TRANSITIVE_DEPENDENCIES validator correctly scores success', () => {
      const validator = TASK_TRANSITIVE_DEPENDENCIES.successValidator

      // Successful result: found target and completed task
      const successOutput: AgentOutput = {
        searchResults: [
          {
            query: 'transitive dependencies of createWorktree',
            results: [
              {
                relpath: 'src/git/worktrees.ts',
                content: 'WorktreeService class with createWorktree method',
              },
              {
                relpath: 'src/orchestrator/scheduler.ts',
                content: 'Scheduler calls createWorktree',
              },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The dependency chain: worktrees.ts contains createWorktree, which is called by scheduler.ts. ' +
            'This creates a transitive dependency where any code using Scheduler indirectly depends on createWorktree.',
        },
        searchCount: 3,
        toolCallCount: 10,
        durationSeconds: 60,
      }

      const score = validator(successOutput)
      expect(score.searchQuality).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.7)
      expect(score.total).toBeGreaterThan(0.5)
    })

    it('TASK_TRANSITIVE_DEPENDENCIES validator correctly scores failure', () => {
      const validator = TASK_TRANSITIVE_DEPENDENCIES.successValidator

      // Failed result: didn't find target or complete task
      const failureOutput: AgentOutput = {
        searchResults: [
          {
            query: 'worktree',
            results: [{ relpath: 'README.md', content: 'worktree documentation' }],
          },
        ],
        workResult: {
          success: false,
          explanationText: 'Could not find the dependency information',
        },
        searchCount: 5,
        toolCallCount: 20,
        durationSeconds: 200,
      }

      const score = validator(failureOutput)
      expect(score.searchQuality).toBeLessThan(0.5)
      expect(score.taskCompletion).toBe(0)
      expect(score.total).toBeLessThan(0.5)
    })

    it('TASK_CALL_CHAIN_TRACING validator correctly scores partial success', () => {
      const validator = TASK_CALL_CHAIN_TRACING.successValidator

      // Partial result: found target but incomplete explanation (missing pattern match)
      const partialOutput: AgentOutput = {
        searchResults: [
          {
            query: 'worktree creation flow',
            results: [
              { relpath: 'src/cli/worktree.ts', content: 'CLI command definition' },
              { relpath: 'src/git/worktrees.ts', content: 'WorktreeService implementation' },
            ],
          },
        ],
        workResult: {
          success: true,
          // Mentions both files but doesn't describe the flow/chain properly
          explanationText: 'The worktree.ts and worktrees.ts files contain related code',
        },
        searchCount: 2,
        toolCallCount: 8,
        durationSeconds: 45,
      }

      const score = validator(partialOutput)
      expect(score.searchQuality).toBeGreaterThan(0.5)
      // Task completion of 0.8 because mentions all files but doesn't match pattern
      expect(score.taskCompletion).toBe(0.8)
    })

    it('TASK_API_IMPACT_ANALYSIS validator scores based on completeness', () => {
      const validator = TASK_API_IMPACT_ANALYSIS.successValidator

      // Complete result with production code and tests
      const completeOutput: AgentOutput = {
        searchResults: [
          {
            query: 'createWorktree API usage',
            results: [
              { relpath: 'src/orchestrator/scheduler.ts', content: 'wt.createWorktree(...)' },
              { relpath: 'src/cli/worktree.ts', content: 'await wt.createWorktree(...)' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'Impact analysis: scheduler.ts and worktree.ts both call createWorktree. ' +
            'Tests in worktrees.test.ts would need updating. ' +
            'API signature changes would break both production callers.',
        },
        searchCount: 4,
        toolCallCount: 12,
        durationSeconds: 80,
      }

      const score = validator(completeOutput)
      expect(score.searchQuality).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.7)
      expect(score.total).toBeGreaterThan(0.5)
    })
  })

  describe('Internal Notes and Documentation', () => {
    it('all tasks have internal notes explaining value', () => {
      expect(TASK_TRANSITIVE_DEPENDENCIES.internalNotes).toBeTruthy()
      expect(TASK_TRANSITIVE_DEPENDENCIES.internalNotes?.toLowerCase()).toContain('grep')
      expect(TASK_CALL_CHAIN_TRACING.internalNotes).toBeTruthy()
      expect(TASK_CALL_CHAIN_TRACING.internalNotes?.toLowerCase()).toContain('grep')
      expect(TASK_API_IMPACT_ANALYSIS.internalNotes).toBeTruthy()
      expect(TASK_API_IMPACT_ANALYSIS.internalNotes?.toLowerCase()).toContain('grep')
    })

    it('task descriptions are clear and actionable', () => {
      // Each description should specify what to find
      expect(TASK_TRANSITIVE_DEPENDENCIES.description).toContain('createWorktree')
      expect(TASK_TRANSITIVE_DEPENDENCIES.description).toContain('2 levels')

      expect(TASK_CALL_CHAIN_TRACING.description).toContain('CLI')
      expect(TASK_CALL_CHAIN_TRACING.description).toContain('git')

      expect(TASK_API_IMPACT_ANALYSIS.description).toContain('signature')
      expect(TASK_API_IMPACT_ANALYSIS.description).toContain('test')
    })
  })

  describe('Edge Cases', () => {
    it('validators handle empty search results', () => {
      const emptyOutput: AgentOutput = {
        searchResults: [],
        workResult: { success: false },
        searchCount: 0,
        toolCallCount: 0,
        durationSeconds: 0,
      }

      const score1 = TASK_TRANSITIVE_DEPENDENCIES.successValidator(emptyOutput)
      // With 0 searches, efficiency component gives some score
      expect(score1.total).toBeLessThan(0.3)

      const score2 = TASK_CALL_CHAIN_TRACING.successValidator(emptyOutput)
      expect(score2.total).toBeLessThan(0.3)

      const score3 = TASK_API_IMPACT_ANALYSIS.successValidator(emptyOutput)
      expect(score3.total).toBeLessThan(0.3)
    })

    it('validators handle missing explanationText', () => {
      const noExplanationOutput: AgentOutput = {
        searchResults: [
          {
            query: 'test',
            results: [{ relpath: 'test.ts', content: 'test' }],
          },
        ],
        workResult: { success: true }, // No explanationText
        searchCount: 1,
        toolCallCount: 5,
        durationSeconds: 30,
      }

      const score1 = TASK_TRANSITIVE_DEPENDENCIES.successValidator(noExplanationOutput)
      expect(score1.taskCompletion).toBe(0)

      const score2 = TASK_CALL_CHAIN_TRACING.successValidator(noExplanationOutput)
      expect(score2.taskCompletion).toBe(0)

      const score3 = TASK_API_IMPACT_ANALYSIS.successValidator(noExplanationOutput)
      expect(score3.taskCompletion).toBe(0)
    })

    it('validators reward efficiency', () => {
      const createOutput = (searchCount: number, toolCallCount: number, durationSeconds: number): AgentOutput => ({
        searchResults: [
          {
            query: 'test',
            results: [
              { relpath: 'src/git/worktrees.ts', content: 'createWorktree' },
              { relpath: 'src/orchestrator/scheduler.ts', content: 'scheduler' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'worktrees.ts contains createWorktree. scheduler.ts calls it. ' + 'Transitive dependencies exist.',
        },
        searchCount,
        toolCallCount,
        durationSeconds,
      })

      // Efficient execution
      const efficientScore = TASK_TRANSITIVE_DEPENDENCIES.successValidator(createOutput(2, 6, 40))

      // Inefficient execution
      const inefficientScore = TASK_TRANSITIVE_DEPENDENCIES.successValidator(createOutput(15, 50, 280))

      expect(efficientScore.efficiency).toBeGreaterThan(inefficientScore.efficiency)
      expect(efficientScore.total).toBeGreaterThan(inefficientScore.total)
    })
  })
})
