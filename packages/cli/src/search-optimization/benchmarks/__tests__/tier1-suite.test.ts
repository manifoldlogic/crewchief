/**
 * Tests for Tier 1 Benchmark Suite composition and helper functions
 */

import { describe, it, expect } from 'vitest'
import {
  TIER1_GREP_IMPOSSIBLE_SUITE,
  getTasksByCategory,
  getTasksByDifficulty,
  getSuiteStatistics,
} from '../tier1-impossible.js'

describe('TIER1_GREP_IMPOSSIBLE_SUITE', () => {
  describe('Suite Composition', () => {
    it('should contain exactly 8 tasks', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.tasks).toHaveLength(8)
    })

    it('should be named "Tier 1: Grep-Impossible Tasks"', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.name).toBe('Tier 1: Grep-Impossible Tasks')
    })

    it('should be version 1.0.0', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.version).toBe('1.0.0')
    })

    it('should be tier 1', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.tier).toBe(1)
    })

    it('should have metadata with correct totalTasks', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.totalTasks).toBe(8)
    })

    it('should have a meaningful description', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.description).toBeTruthy()
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.description.length).toBeGreaterThan(50)
    })
  })

  describe('Category Coverage', () => {
    it('should include 3 categories', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.categories).toHaveLength(3)
    })

    it('should include relationship-discovery category', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.categories).toContain('relationship-discovery')
    })

    it('should include architectural-understanding category', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.categories).toContain('architectural-understanding')
    })

    it('should include negative-space category', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.categories).toContain('negative-space')
    })

    it('should have 3 relationship-discovery tasks', () => {
      const tasks = TIER1_GREP_IMPOSSIBLE_SUITE.tasks.filter((t) => t.category === 'relationship-discovery')
      expect(tasks).toHaveLength(3)
    })

    it('should have 3 architectural-understanding tasks', () => {
      const tasks = TIER1_GREP_IMPOSSIBLE_SUITE.tasks.filter((t) => t.category === 'architectural-understanding')
      expect(tasks).toHaveLength(3)
    })

    it('should have 2 negative-space tasks', () => {
      const tasks = TIER1_GREP_IMPOSSIBLE_SUITE.tasks.filter((t) => t.category === 'negative-space')
      expect(tasks).toHaveLength(2)
    })
  })

  describe('Task Metadata', () => {
    it('should have unique task IDs', () => {
      const ids = TIER1_GREP_IMPOSSIBLE_SUITE.tasks.map((t) => t.id)
      const uniqueIds = new Set(ids)
      expect(uniqueIds.size).toBe(ids.length)
    })

    it('should have all tasks with required fields', () => {
      for (const task of TIER1_GREP_IMPOSSIBLE_SUITE.tasks) {
        expect(task.id).toBeTruthy()
        expect(task.name).toBeTruthy()
        expect(task.description).toBeTruthy()
        expect(task.category).toBeTruthy()
        expect(task.difficulty).toBeTruthy()
        expect(task.searchTarget).toBeTruthy()
        expect(task.followUpTask).toBeTruthy()
        expect(task.successValidator).toBeTruthy()
      }
    })

    it('should have all tasks with expected success rates', () => {
      for (const task of TIER1_GREP_IMPOSSIBLE_SUITE.tasks) {
        expect((task as any).expectedGrepSuccess).toBeTypeOf('number')
        expect((task as any).expectedSearchSuccess).toBeTypeOf('number')
        expect((task as any).expectedGrepSuccess).toBeGreaterThanOrEqual(0)
        expect((task as any).expectedGrepSuccess).toBeLessThanOrEqual(1)
        expect((task as any).expectedSearchSuccess).toBeGreaterThanOrEqual(0)
        expect((task as any).expectedSearchSuccess).toBeLessThanOrEqual(1)
      }
    })

    it('should have all tasks with valid difficulty levels', () => {
      const validDifficulties = ['easy', 'medium', 'hard']
      for (const task of TIER1_GREP_IMPOSSIBLE_SUITE.tasks) {
        expect(validDifficulties).toContain(task.difficulty)
      }
    })
  })

  describe('Expected Success Rates', () => {
    it('should have expectedGrepSuccessRate calculated from tasks', () => {
      const tasks = TIER1_GREP_IMPOSSIBLE_SUITE.tasks
      const grepRates = tasks.map((t) => (t as any).expectedGrepSuccess ?? 0)
      const avgGrep = grepRates.reduce((sum, rate) => sum + rate, 0) / grepRates.length

      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.expectedGrepSuccessRate).toBeCloseTo(avgGrep, 5)
    })

    it('should have expectedSearchSuccessRate calculated from tasks', () => {
      const tasks = TIER1_GREP_IMPOSSIBLE_SUITE.tasks
      const searchRates = tasks.map((t) => (t as any).expectedSearchSuccess ?? 0)
      const avgSearch = searchRates.reduce((sum, rate) => sum + rate, 0) / searchRates.length

      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.expectedSearchSuccessRate).toBeCloseTo(avgSearch, 5)
    })

    it('should have grep success rate < 0.3 (truly grep-impossible)', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.expectedGrepSuccessRate).toBeLessThan(0.3)
    })

    it('should have search success rate > grep success rate', () => {
      expect(TIER1_GREP_IMPOSSIBLE_SUITE.metadata.expectedSearchSuccessRate).toBeGreaterThan(
        TIER1_GREP_IMPOSSIBLE_SUITE.metadata.expectedGrepSuccessRate,
      )
    })

    it('should have meaningful improvement (search - grep > 0.3)', () => {
      const improvement =
        TIER1_GREP_IMPOSSIBLE_SUITE.metadata.expectedSearchSuccessRate -
        TIER1_GREP_IMPOSSIBLE_SUITE.metadata.expectedGrepSuccessRate

      expect(improvement).toBeGreaterThan(0.3)
    })
  })
})

describe('getTasksByCategory', () => {
  it('should return a map with 3 categories', () => {
    const byCategory = getTasksByCategory(TIER1_GREP_IMPOSSIBLE_SUITE)
    expect(byCategory.size).toBe(3)
  })

  it('should have relationship-discovery category with 3 tasks', () => {
    const byCategory = getTasksByCategory(TIER1_GREP_IMPOSSIBLE_SUITE)
    const tasks = byCategory.get('relationship-discovery')
    expect(tasks).toBeDefined()
    expect(tasks).toHaveLength(3)
  })

  it('should have architectural-understanding category with 3 tasks', () => {
    const byCategory = getTasksByCategory(TIER1_GREP_IMPOSSIBLE_SUITE)
    const tasks = byCategory.get('architectural-understanding')
    expect(tasks).toBeDefined()
    expect(tasks).toHaveLength(3)
  })

  it('should have negative-space category with 2 tasks', () => {
    const byCategory = getTasksByCategory(TIER1_GREP_IMPOSSIBLE_SUITE)
    const tasks = byCategory.get('negative-space')
    expect(tasks).toBeDefined()
    expect(tasks).toHaveLength(2)
  })

  it('should not lose any tasks', () => {
    const byCategory = getTasksByCategory(TIER1_GREP_IMPOSSIBLE_SUITE)
    const totalTasks = Array.from(byCategory.values()).reduce((sum, tasks) => sum + tasks.length, 0)
    expect(totalTasks).toBe(8)
  })

  it('should have all tasks in correct category', () => {
    const byCategory = getTasksByCategory(TIER1_GREP_IMPOSSIBLE_SUITE)

    for (const [category, tasks] of byCategory) {
      for (const task of tasks) {
        expect(task.category).toBe(category)
      }
    }
  })
})

describe('getTasksByDifficulty', () => {
  it('should return a map', () => {
    const byDifficulty = getTasksByDifficulty(TIER1_GREP_IMPOSSIBLE_SUITE)
    expect(byDifficulty).toBeInstanceOf(Map)
    expect(byDifficulty.size).toBeGreaterThan(0)
  })

  it('should not lose any tasks', () => {
    const byDifficulty = getTasksByDifficulty(TIER1_GREP_IMPOSSIBLE_SUITE)
    const totalTasks = Array.from(byDifficulty.values()).reduce((sum, tasks) => sum + tasks.length, 0)
    expect(totalTasks).toBe(8)
  })

  it('should have all tasks in correct difficulty level', () => {
    const byDifficulty = getTasksByDifficulty(TIER1_GREP_IMPOSSIBLE_SUITE)

    for (const [difficulty, tasks] of byDifficulty) {
      for (const task of tasks) {
        expect(task.difficulty).toBe(difficulty)
      }
    }
  })

  it('should have at least one difficulty level', () => {
    const byDifficulty = getTasksByDifficulty(TIER1_GREP_IMPOSSIBLE_SUITE)
    expect(byDifficulty.size).toBeGreaterThanOrEqual(1)
  })
})

describe('getSuiteStatistics', () => {
  const stats = getSuiteStatistics(TIER1_GREP_IMPOSSIBLE_SUITE)

  it('should return correct total tasks', () => {
    expect(stats.totalTasks).toBe(8)
  })

  it('should have category statistics', () => {
    expect(stats.byCategory.size).toBe(3)
  })

  it('should have difficulty statistics', () => {
    expect(stats.byDifficulty.size).toBeGreaterThan(0)
  })

  it('should have overall grep success rate', () => {
    expect(stats.overallGrepSuccess).toBeTypeOf('number')
    expect(stats.overallGrepSuccess).toBeGreaterThanOrEqual(0)
    expect(stats.overallGrepSuccess).toBeLessThanOrEqual(1)
  })

  it('should have overall search success rate', () => {
    expect(stats.overallSearchSuccess).toBeTypeOf('number')
    expect(stats.overallSearchSuccess).toBeGreaterThanOrEqual(0)
    expect(stats.overallSearchSuccess).toBeLessThanOrEqual(1)
  })

  it('should have expected improvement', () => {
    expect(stats.expectedImprovement).toBeTypeOf('number')
    expect(stats.expectedImprovement).toBeGreaterThan(0)
  })

  it('should calculate improvement correctly', () => {
    expect(stats.expectedImprovement).toBeCloseTo(stats.overallSearchSuccess - stats.overallGrepSuccess, 5)
  })

  describe('Category Statistics', () => {
    it('should have statistics for relationship-discovery', () => {
      const catStats = stats.byCategory.get('relationship-discovery')
      expect(catStats).toBeDefined()
      expect(catStats!.taskCount).toBe(3)
      expect(catStats!.category).toBe('relationship-discovery')
      expect(catStats!.taskIds).toHaveLength(3)
    })

    it('should have statistics for architectural-understanding', () => {
      const catStats = stats.byCategory.get('architectural-understanding')
      expect(catStats).toBeDefined()
      expect(catStats!.taskCount).toBe(3)
      expect(catStats!.category).toBe('architectural-understanding')
      expect(catStats!.taskIds).toHaveLength(3)
    })

    it('should have statistics for negative-space', () => {
      const catStats = stats.byCategory.get('negative-space')
      expect(catStats).toBeDefined()
      expect(catStats!.taskCount).toBe(2)
      expect(catStats!.category).toBe('negative-space')
      expect(catStats!.taskIds).toHaveLength(2)
    })

    it('should have valid success rates for each category', () => {
      for (const [, catStats] of stats.byCategory) {
        expect(catStats.avgGrepSuccess).toBeGreaterThanOrEqual(0)
        expect(catStats.avgGrepSuccess).toBeLessThanOrEqual(1)
        expect(catStats.avgSearchSuccess).toBeGreaterThanOrEqual(0)
        expect(catStats.avgSearchSuccess).toBeLessThanOrEqual(1)
        expect(catStats.avgSearchSuccess).toBeGreaterThan(catStats.avgGrepSuccess)
      }
    })
  })

  describe('Difficulty Statistics', () => {
    it('should have valid success rates for each difficulty', () => {
      for (const [, diffStats] of stats.byDifficulty) {
        expect(diffStats.avgGrepSuccess).toBeGreaterThanOrEqual(0)
        expect(diffStats.avgGrepSuccess).toBeLessThanOrEqual(1)
        expect(diffStats.avgSearchSuccess).toBeGreaterThanOrEqual(0)
        expect(diffStats.avgSearchSuccess).toBeLessThanOrEqual(1)
      }
    })

    it('should have task IDs for each difficulty', () => {
      for (const [, diffStats] of stats.byDifficulty) {
        expect(diffStats.taskIds.length).toBe(diffStats.taskCount)
      }
    })
  })
})
