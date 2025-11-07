import { describe, it, expect } from 'vitest'
import {
  DifficultyLevel,
  DIFFICULTY_THRESHOLDS,
  classifyDifficulty,
  validateDifficulty,
  getDifficultyRange,
  isTooEasy,
  isTooHard,
} from '../difficulty.js'

describe('Difficulty', () => {
  describe('DifficultyLevel enum', () => {
    it('should have three difficulty levels', () => {
      expect(DifficultyLevel.GREP_IMPOSSIBLE).toBe('grep-impossible')
      expect(DifficultyLevel.GREP_HARD).toBe('grep-hard')
      expect(DifficultyLevel.GREP_POSSIBLE).toBe('grep-possible')
    })
  })

  describe('DIFFICULTY_THRESHOLDS', () => {
    it('should have consistent threshold values', () => {
      expect(DIFFICULTY_THRESHOLDS.IMPOSSIBLE_MAX).toBe(0.3)
      expect(DIFFICULTY_THRESHOLDS.HARD_MIN).toBe(0.3)
      expect(DIFFICULTY_THRESHOLDS.HARD_MAX).toBe(0.6)
      expect(DIFFICULTY_THRESHOLDS.POSSIBLE_MIN).toBe(0.6)
    })

    it('should have non-overlapping ranges', () => {
      expect(DIFFICULTY_THRESHOLDS.IMPOSSIBLE_MAX).toBeLessThanOrEqual(DIFFICULTY_THRESHOLDS.HARD_MIN)
      expect(DIFFICULTY_THRESHOLDS.HARD_MAX).toBeLessThanOrEqual(DIFFICULTY_THRESHOLDS.POSSIBLE_MIN)
    })

    it('should cover the full 0-1 range', () => {
      expect(DIFFICULTY_THRESHOLDS.IMPOSSIBLE_MAX).toBeGreaterThan(0)
      expect(DIFFICULTY_THRESHOLDS.POSSIBLE_MIN).toBeLessThan(1)
    })
  })

  describe('classifyDifficulty', () => {
    it('should classify low success rates as impossible', () => {
      expect(classifyDifficulty(0.0)).toBe(DifficultyLevel.GREP_IMPOSSIBLE)
      expect(classifyDifficulty(0.1)).toBe(DifficultyLevel.GREP_IMPOSSIBLE)
      expect(classifyDifficulty(0.25)).toBe(DifficultyLevel.GREP_IMPOSSIBLE)
      expect(classifyDifficulty(0.29)).toBe(DifficultyLevel.GREP_IMPOSSIBLE)
    })

    it('should classify medium success rates as hard', () => {
      expect(classifyDifficulty(0.3)).toBe(DifficultyLevel.GREP_HARD)
      expect(classifyDifficulty(0.4)).toBe(DifficultyLevel.GREP_HARD)
      expect(classifyDifficulty(0.5)).toBe(DifficultyLevel.GREP_HARD)
      expect(classifyDifficulty(0.59)).toBe(DifficultyLevel.GREP_HARD)
    })

    it('should classify high success rates as possible', () => {
      expect(classifyDifficulty(0.6)).toBe(DifficultyLevel.GREP_POSSIBLE)
      expect(classifyDifficulty(0.7)).toBe(DifficultyLevel.GREP_POSSIBLE)
      expect(classifyDifficulty(0.9)).toBe(DifficultyLevel.GREP_POSSIBLE)
      expect(classifyDifficulty(1.0)).toBe(DifficultyLevel.GREP_POSSIBLE)
    })

    it('should handle edge cases at thresholds', () => {
      // At threshold boundaries
      expect(classifyDifficulty(0.3)).toBe(DifficultyLevel.GREP_HARD)
      expect(classifyDifficulty(0.6)).toBe(DifficultyLevel.GREP_POSSIBLE)

      // Just below thresholds
      expect(classifyDifficulty(0.299)).toBe(DifficultyLevel.GREP_IMPOSSIBLE)
      expect(classifyDifficulty(0.599)).toBe(DifficultyLevel.GREP_HARD)
    })
  })

  describe('validateDifficulty', () => {
    it('should validate correct difficulty classifications', () => {
      expect(validateDifficulty(DifficultyLevel.GREP_IMPOSSIBLE, 0.2)).toBeTruthy()
      expect(validateDifficulty(DifficultyLevel.GREP_HARD, 0.45)).toBeTruthy()
      expect(validateDifficulty(DifficultyLevel.GREP_POSSIBLE, 0.8)).toBeTruthy()
    })

    it('should reject incorrect difficulty classifications', () => {
      // Task designed as impossible but grep succeeds too often
      expect(validateDifficulty(DifficultyLevel.GREP_IMPOSSIBLE, 0.65)).toBeFalsy()

      // Task designed as hard but grep fails too often
      expect(validateDifficulty(DifficultyLevel.GREP_HARD, 0.2)).toBeFalsy()

      // Task designed as possible but grep fails too often
      expect(validateDifficulty(DifficultyLevel.GREP_POSSIBLE, 0.4)).toBeFalsy()
    })

    it('should validate edge cases', () => {
      // Right at threshold boundaries
      expect(validateDifficulty(DifficultyLevel.GREP_HARD, 0.3)).toBeTruthy()
      expect(validateDifficulty(DifficultyLevel.GREP_POSSIBLE, 0.6)).toBeTruthy()

      // Just below thresholds - should fail validation
      expect(validateDifficulty(DifficultyLevel.GREP_HARD, 0.299)).toBeFalsy()
      expect(validateDifficulty(DifficultyLevel.GREP_POSSIBLE, 0.599)).toBeFalsy()
    })
  })

  describe('getDifficultyRange', () => {
    it('should return correct range for impossible', () => {
      const range = getDifficultyRange(DifficultyLevel.GREP_IMPOSSIBLE)
      expect(range.min).toBe(0)
      expect(range.max).toBe(0.3)
    })

    it('should return correct range for hard', () => {
      const range = getDifficultyRange(DifficultyLevel.GREP_HARD)
      expect(range.min).toBe(0.3)
      expect(range.max).toBe(0.6)
    })

    it('should return correct range for possible', () => {
      const range = getDifficultyRange(DifficultyLevel.GREP_POSSIBLE)
      expect(range.min).toBe(0.6)
      expect(range.max).toBe(1.0)
    })

    it('should have non-overlapping ranges', () => {
      const impossible = getDifficultyRange(DifficultyLevel.GREP_IMPOSSIBLE)
      const hard = getDifficultyRange(DifficultyLevel.GREP_HARD)
      const possible = getDifficultyRange(DifficultyLevel.GREP_POSSIBLE)

      expect(impossible.max).toBeLessThanOrEqual(hard.min)
      expect(hard.max).toBeLessThanOrEqual(possible.min)
    })

    it('should cover the full range 0-1', () => {
      const impossible = getDifficultyRange(DifficultyLevel.GREP_IMPOSSIBLE)
      const possible = getDifficultyRange(DifficultyLevel.GREP_POSSIBLE)

      expect(impossible.min).toBe(0)
      expect(possible.max).toBe(1.0)
    })
  })

  describe('isTooEasy', () => {
    it('should identify tasks that are too easy', () => {
      expect(isTooEasy(0.8)).toBeTruthy()
      expect(isTooEasy(0.9)).toBeTruthy()
      expect(isTooEasy(1.0)).toBeTruthy()
      expect(isTooEasy(0.71)).toBeTruthy()
    })

    it('should not flag appropriate difficulty tasks', () => {
      expect(isTooEasy(0.7)).toBeFalsy()
      expect(isTooEasy(0.6)).toBeFalsy()
      expect(isTooEasy(0.5)).toBeFalsy()
      expect(isTooEasy(0.3)).toBeFalsy()
      expect(isTooEasy(0.0)).toBeFalsy()
    })

    it('should handle edge cases', () => {
      // Right at threshold
      expect(isTooEasy(0.7)).toBeFalsy()

      // Just above threshold
      expect(isTooEasy(0.701)).toBeTruthy()
    })
  })

  describe('isTooHard', () => {
    it('should identify tasks where both tools fail', () => {
      expect(isTooHard(0.05, 0.05)).toBeTruthy()
      expect(isTooHard(0.0, 0.0)).toBeTruthy()
      expect(isTooHard(0.09, 0.09)).toBeTruthy()
    })

    it('should not flag tasks where grep fails but search succeeds', () => {
      expect(isTooHard(0.05, 0.5)).toBeFalsy()
      expect(isTooHard(0.0, 0.8)).toBeFalsy()
      expect(isTooHard(0.09, 0.7)).toBeFalsy()
    })

    it('should not flag tasks where grep succeeds', () => {
      expect(isTooHard(0.5, 0.05)).toBeFalsy()
      expect(isTooHard(0.3, 0.3)).toBeFalsy()
      expect(isTooHard(0.7, 0.0)).toBeFalsy()
    })

    it('should handle edge cases', () => {
      // Right at threshold
      expect(isTooHard(0.1, 0.1)).toBeFalsy()

      // Just below threshold
      expect(isTooHard(0.09, 0.09)).toBeTruthy()

      // Mixed cases at boundary
      expect(isTooHard(0.09, 0.11)).toBeFalsy()
      expect(isTooHard(0.11, 0.09)).toBeFalsy()
    })
  })

  describe('threshold coherence', () => {
    it('should classify all valid success rates consistently', () => {
      // Test across the full range in small increments
      for (let rate = 0; rate <= 1.0; rate += 0.01) {
        const difficulty = classifyDifficulty(rate)
        const range = getDifficultyRange(difficulty)

        // The rate should fall within the range for its classification
        expect(rate).toBeGreaterThanOrEqual(range.min)
        expect(rate).toBeLessThanOrEqual(range.max)
      }
    })

    it('should have validation consistent with classification', () => {
      const testRates = [0.0, 0.15, 0.3, 0.45, 0.6, 0.75, 0.9, 1.0]

      testRates.forEach((rate) => {
        const difficulty = classifyDifficulty(rate)
        expect(validateDifficulty(difficulty, rate)).toBeTruthy()
      })
    })
  })
})
