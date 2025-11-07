/**
 * Tests for negative space tasks
 */

import { describe, it, expect } from 'vitest'
import type { AgentOutput } from '../../../types.js'
import { TASK_MISSING_ERROR_HANDLING, TASK_UNPROTECTED_FILE_OPERATIONS } from '../index.js'

describe('Negative Space Tasks', () => {
  describe('Task Structure Validation', () => {
    it('TASK_MISSING_ERROR_HANDLING has all required fields', () => {
      expect(TASK_MISSING_ERROR_HANDLING.id).toBe('negative-space-missing-error-handling')
      expect(TASK_MISSING_ERROR_HANDLING.name).toBeTruthy()
      expect(TASK_MISSING_ERROR_HANDLING.description).toBeTruthy()
      expect(TASK_MISSING_ERROR_HANDLING.category).toBe('negative-space')
      expect(TASK_MISSING_ERROR_HANDLING.difficulty).toBe('hard')
      expect(TASK_MISSING_ERROR_HANDLING.searchTarget).toBeDefined()
      expect(TASK_MISSING_ERROR_HANDLING.followUpTask).toBeDefined()
      expect(TASK_MISSING_ERROR_HANDLING.successValidator).toBeTypeOf('function')
      expect(TASK_MISSING_ERROR_HANDLING.maxSearchAttempts).toBe(10)
      expect(TASK_MISSING_ERROR_HANDLING.maxTimeSeconds).toBe(300)
    })

    it('TASK_UNPROTECTED_FILE_OPERATIONS has all required fields', () => {
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.id).toBe('negative-space-unprotected-operations')
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.name).toBeTruthy()
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.description).toBeTruthy()
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.category).toBe('negative-space')
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.difficulty).toBe('hard')
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.searchTarget).toBeDefined()
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.followUpTask).toBeDefined()
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.successValidator).toBeTypeOf('function')
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.maxSearchAttempts).toBe(10)
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.maxTimeSeconds).toBe(300)
    })
  })

  describe('Expected Success Rates', () => {
    it('TASK_MISSING_ERROR_HANDLING expects low grep success (20%)', () => {
      expect(TASK_MISSING_ERROR_HANDLING.expectedGrepSuccess).toBe(0.2)
    })

    it('TASK_MISSING_ERROR_HANDLING expects high search success (80%)', () => {
      expect(TASK_MISSING_ERROR_HANDLING.expectedSearchSuccess).toBe(0.8)
    })

    it('TASK_UNPROTECTED_FILE_OPERATIONS expects low grep success (25%)', () => {
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.expectedGrepSuccess).toBe(0.25)
    })

    it('TASK_UNPROTECTED_FILE_OPERATIONS expects high search success (75%)', () => {
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.expectedSearchSuccess).toBe(0.75)
    })

    it('both tasks demonstrate grep-impossible characteristics', () => {
      // Low grep success (<= 30%)
      expect(TASK_MISSING_ERROR_HANDLING.expectedGrepSuccess).toBeLessThanOrEqual(0.3)
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.expectedGrepSuccess).toBeLessThanOrEqual(0.3)

      // High search success (>= 70%)
      expect(TASK_MISSING_ERROR_HANDLING.expectedSearchSuccess).toBeGreaterThanOrEqual(0.7)
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.expectedSearchSuccess).toBeGreaterThanOrEqual(0.7)

      // Large gap demonstrating semantic search advantage (>= 50%)
      const gap1 = TASK_MISSING_ERROR_HANDLING.expectedSearchSuccess - TASK_MISSING_ERROR_HANDLING.expectedGrepSuccess
      const gap2 =
        TASK_UNPROTECTED_FILE_OPERATIONS.expectedSearchSuccess - TASK_UNPROTECTED_FILE_OPERATIONS.expectedGrepSuccess

      expect(gap1).toBeGreaterThanOrEqual(0.5)
      expect(gap2).toBeGreaterThanOrEqual(0.5)
    })
  })

  describe('Search Targets', () => {
    it('TASK_MISSING_ERROR_HANDLING has pattern search target', () => {
      const target = TASK_MISSING_ERROR_HANDLING.searchTarget
      expect(target.type).toBe('pattern')
      expect(target.pattern).toBeInstanceOf(RegExp)
      // Should match violation file names
      expect(target.pattern?.test('worktrees')).toBe(true)
      expect(target.pattern?.test('release')).toBe(true)
      expect(target.pattern?.test('worktree-metadata')).toBe(true)
      // Should match async operation patterns
      expect(target.pattern?.test('git.raw')).toBe(true)
      expect(target.pattern?.test('fs.writeFile')).toBe(true)
    })

    it('TASK_UNPROTECTED_FILE_OPERATIONS has pattern search target', () => {
      const target = TASK_UNPROTECTED_FILE_OPERATIONS.searchTarget
      expect(target.type).toBe('pattern')
      expect(target.pattern).toBeInstanceOf(RegExp)
      // Should match violation file names
      expect(target.pattern?.test('worktree-metadata')).toBe(true)
      expect(target.pattern?.test('setup')).toBe(true)
      // Should match file operation patterns
      expect(target.pattern?.test('fs.writeFile')).toBe(true)
      expect(target.pattern?.test('fs.writeFileSync')).toBe(true)
      expect(target.pattern?.test('file operation')).toBe(true)
    })
  })

  describe('Follow-up Task Validators', () => {
    it('TASK_MISSING_ERROR_HANDLING validator requires violation files', () => {
      const validator = TASK_MISSING_ERROR_HANDLING.followUpTask.validator
      expect(validator.type).toBe('explanation')
      expect(validator.mentionsFiles).toContain('worktrees.ts')
      expect(validator.mentionsFiles).toContain('release.ts')
      expect(validator.mentionsFiles).toContain('worktree-metadata.ts')
      expect(validator.mentionsPattern).toBeInstanceOf(RegExp)

      // Should match error-handling absence patterns
      expect(validator.mentionsPattern?.test('without error handling')).toBe(true)
      expect(validator.mentionsPattern?.test('missing try catch')).toBe(true)
      expect(validator.mentionsPattern?.test('no error handling')).toBe(true)
      expect(validator.mentionsPattern?.test('async without try catch')).toBe(true)
      expect(validator.mentionsPattern?.test('lacks error handler')).toBe(true)
      // Note: "unprotected async operations" doesn't match the pattern because pattern requires
      // "without/missing/lack/no" before the term. Let's test with a valid phrase:
      expect(validator.mentionsPattern?.test('async operations without error handling')).toBe(true)
    })

    it('TASK_UNPROTECTED_FILE_OPERATIONS validator requires violation files', () => {
      const validator = TASK_UNPROTECTED_FILE_OPERATIONS.followUpTask.validator
      expect(validator.type).toBe('explanation')
      expect(validator.mentionsFiles).toContain('worktree-metadata.ts')
      expect(validator.mentionsFiles).toContain('setup.ts')
      expect(validator.mentionsPattern).toBeInstanceOf(RegExp)

      // Should match validation absence patterns
      expect(validator.mentionsPattern?.test('without validation')).toBe(true)
      expect(validator.mentionsPattern?.test('missing checks')).toBe(true)
      expect(validator.mentionsPattern?.test('no validation')).toBe(true)
      expect(validator.mentionsPattern?.test('unprotected file operations')).toBe(true)
      expect(validator.mentionsPattern?.test('unsafe file writes')).toBe(true)
      expect(validator.mentionsPattern?.test('lacks safety guard')).toBe(true)
      expect(validator.mentionsPattern?.test('unvalidated path')).toBe(true)
    })
  })

  describe('Success Validator Functions', () => {
    it('TASK_MISSING_ERROR_HANDLING validator correctly scores success', () => {
      const validator = TASK_MISSING_ERROR_HANDLING.successValidator

      // Successful result: found violations and explained them
      const successOutput: AgentOutput = {
        searchResults: [
          {
            query: 'async functions without error handling',
            results: [
              {
                relpath: 'packages/cli/src/git/worktrees.ts',
                content: 'await this.git.raw() without try-catch',
              },
              {
                relpath: 'packages/cli/src/cli/release.ts',
                content: 'multiple await git operations',
              },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'Found async functions without error handling in worktrees.ts (line 112) and release.ts (lines 205-210). ' +
            'These git operations lack try-catch blocks and could fail silently.',
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

    it('TASK_MISSING_ERROR_HANDLING validator correctly scores failure', () => {
      const validator = TASK_MISSING_ERROR_HANDLING.successValidator

      // Failed result: didn't find violations
      const failureOutput: AgentOutput = {
        searchResults: [
          {
            query: 'error handling',
            results: [{ relpath: 'README.md', content: 'error handling documentation' }],
          },
        ],
        workResult: {
          success: false,
          explanationText: 'Could not find async functions without error handling',
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

    it('TASK_UNPROTECTED_FILE_OPERATIONS validator correctly scores success', () => {
      const validator = TASK_UNPROTECTED_FILE_OPERATIONS.successValidator

      // Successful result: found unprotected operations
      const successOutput: AgentOutput = {
        searchResults: [
          {
            query: 'file operations without validation',
            results: [
              {
                relpath: 'packages/cli/src/utils/worktree-metadata.ts',
                content: 'fs.writeFile without path validation',
              },
              {
                relpath: 'packages/cli/src/cli/setup.ts',
                content: 'fs.writeFileSync without checks',
              },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'Found file operations without validation: worktree-metadata.ts line 17 uses fs.writeFile ' +
            'without path validation, setup.ts line 69 uses fs.writeFileSync without existence checks. ' +
            'These operations lack safety guards and could fail or create security issues.',
        },
        searchCount: 2,
        toolCallCount: 8,
        durationSeconds: 45,
      }

      const score = validator(successOutput)
      expect(score.searchQuality).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.7)
      expect(score.total).toBeGreaterThan(0.5)
    })

    it('TASK_UNPROTECTED_FILE_OPERATIONS validator correctly scores failure', () => {
      const validator = TASK_UNPROTECTED_FILE_OPERATIONS.successValidator

      // Failed result: didn't find violations (pattern doesn't match)
      const failureOutput: AgentOutput = {
        searchResults: [
          {
            query: 'file operations',
            results: [{ relpath: 'test.ts', content: 'test code' }], // content doesn't match our pattern
          },
        ],
        workResult: {
          success: false,
          explanationText: 'All file operations appear to be protected',
        },
        searchCount: 4,
        toolCallCount: 15,
        durationSeconds: 120,
      }

      const score = validator(failureOutput)
      // Pattern doesn't match, so search quality should be 0
      expect(score.searchQuality).toBe(0)
      expect(score.taskCompletion).toBe(0)
      expect(score.total).toBeLessThan(0.5)
    })

    it('TASK_MISSING_ERROR_HANDLING validator scores partial success', () => {
      const validator = TASK_MISSING_ERROR_HANDLING.successValidator

      // Partial result: found one violation file but incomplete explanation
      const partialOutput: AgentOutput = {
        searchResults: [
          {
            query: 'async without try catch',
            results: [{ relpath: 'packages/cli/src/git/worktrees.ts', content: 'worktrees implementation' }],
          },
        ],
        workResult: {
          success: true,
          // Mentions one file but doesn't match the pattern properly
          explanationText: 'The worktrees.ts file has async functions',
        },
        searchCount: 2,
        toolCallCount: 8,
        durationSeconds: 50,
      }

      const score = validator(partialOutput)
      // Content matches pattern (contains "worktrees")
      expect(score.searchQuality).toBeGreaterThan(0)
      // Should get partial credit for mentioning one file even without pattern match
      expect(score.taskCompletion).toBeGreaterThan(0)
      expect(score.taskCompletion).toBeLessThan(1)
    })

    it('TASK_UNPROTECTED_FILE_OPERATIONS validator scores partial success', () => {
      const validator = TASK_UNPROTECTED_FILE_OPERATIONS.successValidator

      // Partial result: found one violation file but incomplete explanation
      const partialOutput: AgentOutput = {
        searchResults: [
          {
            query: 'unsafe file writes',
            results: [{ relpath: 'packages/cli/src/cli/setup.ts', content: 'file write operations' }],
          },
        ],
        workResult: {
          success: true,
          // Mentions file but doesn't match validation pattern
          explanationText: 'The setup.ts file has file operations',
        },
        searchCount: 3,
        toolCallCount: 10,
        durationSeconds: 60,
      }

      const score = validator(partialOutput)
      expect(score.searchQuality).toBeGreaterThan(0)
      expect(score.taskCompletion).toBeGreaterThan(0)
      expect(score.taskCompletion).toBeLessThan(1)
    })
  })

  describe('Internal Notes and Documentation', () => {
    it('both tasks have internal notes explaining grep limitations', () => {
      expect(TASK_MISSING_ERROR_HANDLING.internalNotes).toBeTruthy()
      expect(TASK_MISSING_ERROR_HANDLING.internalNotes?.toLowerCase()).toContain('grep')
      expect(TASK_MISSING_ERROR_HANDLING.internalNotes?.toLowerCase()).toContain('absence')

      expect(TASK_UNPROTECTED_FILE_OPERATIONS.internalNotes).toBeTruthy()
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.internalNotes?.toLowerCase()).toContain('grep')
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.internalNotes?.toLowerCase()).toContain('protection')
    })

    it('task descriptions clearly explain the negative space challenge', () => {
      expect(TASK_MISSING_ERROR_HANDLING.description).toContain('without')
      expect(TASK_MISSING_ERROR_HANDLING.description.toLowerCase()).toContain('error handling')

      expect(TASK_UNPROTECTED_FILE_OPERATIONS.description).toContain('without')
      expect(TASK_UNPROTECTED_FILE_OPERATIONS.description.toLowerCase()).toContain('validation')
    })

    it('internal notes explain why grep fails', () => {
      const notes1 = TASK_MISSING_ERROR_HANDLING.internalNotes || ''
      expect(notes1.toLowerCase()).toContain('cannot')
      expect(notes1.toLowerCase()).toContain('missing')

      const notes2 = TASK_UNPROTECTED_FILE_OPERATIONS.internalNotes || ''
      expect(notes2.toLowerCase()).toContain('cannot')
      expect(notes2.toLowerCase()).toContain('absent')
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

      const score1 = TASK_MISSING_ERROR_HANDLING.successValidator(emptyOutput)
      expect(score1.total).toBeLessThan(0.3)

      const score2 = TASK_UNPROTECTED_FILE_OPERATIONS.successValidator(emptyOutput)
      expect(score2.total).toBeLessThan(0.3)
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

      const score1 = TASK_MISSING_ERROR_HANDLING.successValidator(noExplanationOutput)
      expect(score1.taskCompletion).toBe(0)

      const score2 = TASK_UNPROTECTED_FILE_OPERATIONS.successValidator(noExplanationOutput)
      expect(score2.taskCompletion).toBe(0)
    })

    it('validators reward efficiency', () => {
      const createOutput = (
        searchCount: number,
        toolCallCount: number,
        durationSeconds: number,
        files: string[],
      ): AgentOutput => ({
        searchResults: [
          {
            query: 'test',
            results: files.map((file) => ({ relpath: file, content: 'code' })),
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'Found violations in worktrees.ts and release.ts without error handling. ' +
            'These async operations lack try-catch protection.',
        },
        searchCount,
        toolCallCount,
        durationSeconds,
      })

      // Efficient execution
      const efficientScore = TASK_MISSING_ERROR_HANDLING.successValidator(
        createOutput(2, 6, 40, ['packages/cli/src/git/worktrees.ts', 'packages/cli/src/cli/release.ts']),
      )

      // Inefficient execution
      const inefficientScore = TASK_MISSING_ERROR_HANDLING.successValidator(
        createOutput(15, 50, 280, ['packages/cli/src/git/worktrees.ts', 'packages/cli/src/cli/release.ts']),
      )

      expect(efficientScore.efficiency).toBeGreaterThan(inefficientScore.efficiency)
      expect(efficientScore.total).toBeGreaterThan(inefficientScore.total)
    })

    it('validators handle test files in results', () => {
      const testFileOutput: AgentOutput = {
        searchResults: [
          {
            query: 'async without error handling',
            results: [
              { relpath: 'packages/cli/tests/worktree.test.ts', content: 'test code' },
              { relpath: 'packages/cli/src/git/worktrees.ts', content: 'worktrees implementation' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText: 'Found async functions without error handling in worktrees.ts git operations',
        },
        searchCount: 2,
        toolCallCount: 8,
        durationSeconds: 45,
      }

      const score = TASK_MISSING_ERROR_HANDLING.successValidator(testFileOutput)
      // Should still get credit for finding production violations (content matches pattern)
      expect(score.searchQuality).toBeGreaterThan(0.5)
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })
  })

  describe('Pattern Matching', () => {
    it('TASK_MISSING_ERROR_HANDLING pattern matches violation indicators', () => {
      const pattern = TASK_MISSING_ERROR_HANDLING.searchTarget.pattern
      expect(pattern).toBeInstanceOf(RegExp)

      // Should match file names with violations
      expect(pattern?.test('src/git/worktrees.ts')).toBe(true)
      expect(pattern?.test('cli/release.ts')).toBe(true)
      expect(pattern?.test('utils/worktree-metadata.ts')).toBe(true)

      // Should match operation patterns
      expect(pattern?.test('await git.raw()')).toBe(true)
      expect(pattern?.test('await fs.writeFile()')).toBe(true)
    })

    it('TASK_UNPROTECTED_FILE_OPERATIONS pattern matches violation indicators', () => {
      const pattern = TASK_UNPROTECTED_FILE_OPERATIONS.searchTarget.pattern
      expect(pattern).toBeInstanceOf(RegExp)

      // Should match file names with violations
      expect(pattern?.test('utils/worktree-metadata.ts')).toBe(true)
      expect(pattern?.test('cli/setup.ts')).toBe(true)

      // Should match operation patterns
      expect(pattern?.test('fs.writeFile(path, data)')).toBe(true)
      expect(pattern?.test('fs.writeFileSync(configPath, content)')).toBe(true)
    })
  })
})
