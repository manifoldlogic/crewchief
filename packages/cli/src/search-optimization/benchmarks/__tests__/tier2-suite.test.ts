/**
 * Tests for Tier 2 Grep-Hard benchmark suite
 */

import { describe, it, expect } from 'vitest'
import { TIER2_GREP_HARD_SUITE, getTasksByCategory, getTasksByDifficulty, getSuiteStatistics } from '../tier2-hard.js'

describe('Tier 2 Grep-Hard Benchmark Suite', () => {
  describe('Suite Structure', () => {
    it('should have correct metadata', () => {
      expect(TIER2_GREP_HARD_SUITE.name).toBe('Tier 2: Grep-Hard Tasks')
      expect(TIER2_GREP_HARD_SUITE.version).toBe('1.0.0')
      expect(TIER2_GREP_HARD_SUITE.tier).toBe(2)
      expect(TIER2_GREP_HARD_SUITE.tasks).toHaveLength(11)
    })

    it('should have 11 tasks total', () => {
      expect(TIER2_GREP_HARD_SUITE.metadata.totalTasks).toBe(11)
    })

    it('should have 3 categories', () => {
      expect(TIER2_GREP_HARD_SUITE.metadata.categories).toHaveLength(3)
      expect(TIER2_GREP_HARD_SUITE.metadata.categories).toContain('conceptual-similarity')
      expect(TIER2_GREP_HARD_SUITE.metadata.categories).toContain('ambiguity-resolution')
      expect(TIER2_GREP_HARD_SUITE.metadata.categories).toContain('cross-cutting-concerns')
    })

    it('should have description', () => {
      expect(TIER2_GREP_HARD_SUITE.metadata.description).toBeTruthy()
      expect(TIER2_GREP_HARD_SUITE.metadata.description).toContain('grep')
      expect(TIER2_GREP_HARD_SUITE.metadata.description).toContain('30-60%')
    })
  })

  describe('Grep Success Rates', () => {
    it('should have average grep success in tier 2 range (30-60%)', () => {
      const grepRate = TIER2_GREP_HARD_SUITE.metadata.expectedGrepSuccessRate
      expect(grepRate).toBeGreaterThanOrEqual(0.3)
      expect(grepRate).toBeLessThanOrEqual(0.6)
    })

    it('should have all tasks with grep success in tier 2 range', () => {
      for (const task of TIER2_GREP_HARD_SUITE.tasks) {
        const grepSuccess = (task as any).expectedGrepSuccess
        expect(grepSuccess).toBeGreaterThanOrEqual(0.3)
        expect(grepSuccess).toBeLessThanOrEqual(0.6)
      }
    })
  })

  describe('Search Success Rates', () => {
    it('should have average search success >70%', () => {
      const searchRate = TIER2_GREP_HARD_SUITE.metadata.expectedSearchSuccessRate
      expect(searchRate).toBeGreaterThan(0.7)
    })

    it('should have all tasks with search success >70%', () => {
      for (const task of TIER2_GREP_HARD_SUITE.tasks) {
        const searchSuccess = (task as any).expectedSearchSuccess
        expect(searchSuccess).toBeGreaterThan(0.7)
      }
    })
  })

  describe('Search Advantage', () => {
    it('should have >30% average improvement', () => {
      const improvement = TIER2_GREP_HARD_SUITE.metadata.expectedImprovement
      expect(improvement).toBeGreaterThan(0.3)
    })

    it('should have all tasks with >30% improvement', () => {
      for (const task of TIER2_GREP_HARD_SUITE.tasks) {
        const grep = (task as any).expectedGrepSuccess
        const search = (task as any).expectedSearchSuccess
        const improvement = search - grep
        expect(improvement).toBeGreaterThan(0.3)
      }
    })

    it('should have expected improvement matching calculation', () => {
      const expected =
        TIER2_GREP_HARD_SUITE.metadata.expectedSearchSuccessRate -
        TIER2_GREP_HARD_SUITE.metadata.expectedGrepSuccessRate
      expect(TIER2_GREP_HARD_SUITE.metadata.expectedImprovement).toBeCloseTo(expected, 2)
    })
  })

  describe('Category Distribution', () => {
    it('should have 4 conceptual similarity tasks', () => {
      const byCategory = getTasksByCategory(TIER2_GREP_HARD_SUITE)
      const conceptualTasks = byCategory.get('conceptual-similarity')
      expect(conceptualTasks).toHaveLength(4)
    })

    it('should have 4 ambiguity resolution tasks', () => {
      const byCategory = getTasksByCategory(TIER2_GREP_HARD_SUITE)
      const ambiguityTasks = byCategory.get('ambiguity-resolution')
      expect(ambiguityTasks).toHaveLength(4)
    })

    it('should have 3 cross-cutting concerns tasks', () => {
      const byCategory = getTasksByCategory(TIER2_GREP_HARD_SUITE)
      const crossCuttingTasks = byCategory.get('cross-cutting-concerns')
      expect(crossCuttingTasks).toHaveLength(3)
    })
  })

  describe('Difficulty Distribution', () => {
    it('should have all tasks at medium difficulty', () => {
      const byDifficulty = getTasksByDifficulty(TIER2_GREP_HARD_SUITE)
      const mediumTasks = byDifficulty.get('medium')
      expect(mediumTasks).toHaveLength(11)
    })

    it('should have no easy or hard tasks', () => {
      const byDifficulty = getTasksByDifficulty(TIER2_GREP_HARD_SUITE)
      expect(byDifficulty.has('easy')).toBe(false)
      expect(byDifficulty.has('hard')).toBe(false)
    })
  })

  describe('Suite Statistics', () => {
    it('should calculate correct overall statistics', () => {
      const stats = getSuiteStatistics(TIER2_GREP_HARD_SUITE)
      expect(stats.totalTasks).toBe(11)
      expect(stats.overallGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(stats.overallGrepSuccess).toBeLessThanOrEqual(0.6)
      expect(stats.overallSearchSuccess).toBeGreaterThan(0.7)
      expect(stats.expectedImprovement).toBeGreaterThan(0.3)
    })

    it('should have statistics for all 3 categories', () => {
      const stats = getSuiteStatistics(TIER2_GREP_HARD_SUITE)
      expect(stats.byCategory.size).toBe(3)
      expect(stats.byCategory.has('conceptual-similarity')).toBe(true)
      expect(stats.byCategory.has('ambiguity-resolution')).toBe(true)
      expect(stats.byCategory.has('cross-cutting-concerns')).toBe(true)
    })

    it('should calculate correct category statistics', () => {
      const stats = getSuiteStatistics(TIER2_GREP_HARD_SUITE)

      for (const [category, categoryStats] of stats.byCategory) {
        expect(categoryStats.category).toBe(category)
        expect(categoryStats.taskCount).toBeGreaterThan(0)
        expect(categoryStats.avgGrepSuccess).toBeGreaterThanOrEqual(0.3)
        expect(categoryStats.avgGrepSuccess).toBeLessThanOrEqual(0.6)
        expect(categoryStats.avgSearchSuccess).toBeGreaterThan(0.7)
        expect(categoryStats.avgImprovement).toBeGreaterThan(0.3)
        expect(categoryStats.taskIds).toHaveLength(categoryStats.taskCount)
      }
    })

    it('should have statistics for medium difficulty', () => {
      const stats = getSuiteStatistics(TIER2_GREP_HARD_SUITE)
      expect(stats.byDifficulty.size).toBe(1)
      expect(stats.byDifficulty.has('medium')).toBe(true)

      const mediumStats = stats.byDifficulty.get('medium')!
      expect(mediumStats.taskCount).toBe(11)
      expect(mediumStats.avgGrepSuccess).toBeGreaterThanOrEqual(0.3)
      expect(mediumStats.avgGrepSuccess).toBeLessThanOrEqual(0.6)
      expect(mediumStats.avgSearchSuccess).toBeGreaterThan(0.7)
      expect(mediumStats.avgImprovement).toBeGreaterThan(0.3)
    })
  })

  describe('Task Validation', () => {
    it('should have all tasks with required fields', () => {
      for (const task of TIER2_GREP_HARD_SUITE.tasks) {
        expect(task.id).toBeTruthy()
        expect(task.name).toBeTruthy()
        expect(task.description).toBeTruthy()
        expect(task.category).toBeTruthy()
        expect(task.difficulty).toBe('medium')
        expect(task.searchTarget).toBeTruthy()
        expect(task.followUpTask).toBeTruthy()
        expect(task.successValidator).toBeTypeOf('function')
      }
    })

    it('should have all tasks with expected success rates', () => {
      for (const task of TIER2_GREP_HARD_SUITE.tasks) {
        expect((task as any).expectedGrepSuccess).toBeGreaterThan(0)
        expect((task as any).expectedSearchSuccess).toBeGreaterThan(0)
        expect((task as any).expectedSearchSuccess).toBeGreaterThan((task as any).expectedGrepSuccess)
      }
    })

    it('should have all tasks with internal notes', () => {
      for (const task of TIER2_GREP_HARD_SUITE.tasks) {
        expect((task as any).internalNotes).toBeTruthy()
      }
    })

    it('should have unique task IDs', () => {
      const ids = TIER2_GREP_HARD_SUITE.tasks.map((t) => t.id)
      const uniqueIds = new Set(ids)
      expect(uniqueIds.size).toBe(ids.length)
    })

    it('should have task IDs with tier2 prefix', () => {
      for (const task of TIER2_GREP_HARD_SUITE.tasks) {
        expect(task.id).toMatch(/^tier2-/)
      }
    })
  })

  describe('Helper Functions', () => {
    it('should organize tasks by category correctly', () => {
      const byCategory = getTasksByCategory(TIER2_GREP_HARD_SUITE)
      let totalTasks = 0

      for (const tasks of byCategory.values()) {
        totalTasks += tasks.length
      }

      expect(totalTasks).toBe(11)
    })

    it('should organize tasks by difficulty correctly', () => {
      const byDifficulty = getTasksByDifficulty(TIER2_GREP_HARD_SUITE)
      let totalTasks = 0

      for (const tasks of byDifficulty.values()) {
        totalTasks += tasks.length
      }

      expect(totalTasks).toBe(11)
    })
  })

  describe('Performance Expectations', () => {
    it('should demonstrate grep struggles with 30-60% success', () => {
      const avgGrep = TIER2_GREP_HARD_SUITE.metadata.expectedGrepSuccessRate
      expect(avgGrep).toBeGreaterThanOrEqual(0.3)
      expect(avgGrep).toBeLessThanOrEqual(0.6)
    })

    it('should demonstrate search advantage >70% success', () => {
      const avgSearch = TIER2_GREP_HARD_SUITE.metadata.expectedSearchSuccessRate
      expect(avgSearch).toBeGreaterThan(0.7)
    })

    it('should demonstrate significant improvement margin', () => {
      const improvement = TIER2_GREP_HARD_SUITE.metadata.expectedImprovement
      // Should be well above 30% threshold, aiming for ~37%
      expect(improvement).toBeGreaterThan(0.3)
      expect(improvement).toBeLessThan(0.5) // But not unrealistically high
    })
  })
})
