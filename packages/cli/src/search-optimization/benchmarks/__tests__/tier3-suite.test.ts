/**
 * Tier 3 Real-World Suite Tests
 *
 * Tests the Tier 3 benchmark suite structure, metadata, and statistics.
 */

import { describe, expect, it } from 'vitest'
import type { SearchTask } from '../../types.js'
import {
  TIER3_REALWORLD_SUITE,
  getSuiteStatistics,
  getTasksByCategory,
  getTasksByDifficulty,
  getTasksByFrequency,
} from '../tier3-realworld.js'

describe('TIER3_REALWORLD_SUITE', () => {
  it('should have correct suite properties', () => {
    expect(TIER3_REALWORLD_SUITE.name).toBe('Tier 3: Real-World Tasks')
    expect(TIER3_REALWORLD_SUITE.version).toBe('1.0.0')
    expect(TIER3_REALWORLD_SUITE.tier).toBe(3)
  })

  it('should contain exactly 9 tasks', () => {
    expect(TIER3_REALWORLD_SUITE.tasks).toHaveLength(9)
  })

  it('should have correct metadata', () => {
    expect(TIER3_REALWORLD_SUITE.metadata.totalTasks).toBe(9)
    expect(TIER3_REALWORLD_SUITE.metadata.categories).toContain('code-review')
    expect(TIER3_REALWORLD_SUITE.metadata.categories).toContain('debugging')
    expect(TIER3_REALWORLD_SUITE.metadata.categories).toContain('refactoring')
    expect(TIER3_REALWORLD_SUITE.metadata.description).toBeDefined()
  })

  it('should have frequency distribution', () => {
    expect(TIER3_REALWORLD_SUITE.metadata.frequencyDistribution).toBeDefined()
    expect(Object.keys(TIER3_REALWORLD_SUITE.metadata.frequencyDistribution).length).toBeGreaterThan(0)
  })

  it('should have scenario types', () => {
    expect(TIER3_REALWORLD_SUITE.metadata.scenarioTypes).toBeDefined()
    expect(TIER3_REALWORLD_SUITE.metadata.scenarioTypes.length).toBe(3)
  })
})

describe('getTasksByCategory', () => {
  it('should organize tasks by category', () => {
    const byCategory = getTasksByCategory(TIER3_REALWORLD_SUITE)

    expect(byCategory.size).toBe(3)
    expect(byCategory.has('code-review')).toBe(true)
    expect(byCategory.has('debugging')).toBe(true)
    expect(byCategory.has('refactoring')).toBe(true)
  })

  it('should have 3 tasks in each category', () => {
    const byCategory = getTasksByCategory(TIER3_REALWORLD_SUITE)

    expect(byCategory.get('code-review')?.length).toBe(3)
    expect(byCategory.get('debugging')?.length).toBe(3)
    expect(byCategory.get('refactoring')?.length).toBe(3)
  })
})

describe('getTasksByDifficulty', () => {
  it('should organize tasks by difficulty', () => {
    const byDifficulty = getTasksByDifficulty(TIER3_REALWORLD_SUITE)

    expect(byDifficulty.size).toBeGreaterThan(0)
    expect(byDifficulty.has('easy') || byDifficulty.has('medium')).toBe(true)
  })

  it('should only have easy and medium tasks', () => {
    const byDifficulty = getTasksByDifficulty(TIER3_REALWORLD_SUITE)

    expect(byDifficulty.has('hard')).toBe(false)
  })

  it('difficulty distributions should sum to 9 tasks', () => {
    const byDifficulty = getTasksByDifficulty(TIER3_REALWORLD_SUITE)
    let total = 0

    for (const tasks of byDifficulty.values()) {
      total += tasks.length
    }

    expect(total).toBe(9)
  })
})

describe('getTasksByFrequency', () => {
  it('should organize tasks by frequency', () => {
    const byFrequency = getTasksByFrequency(TIER3_REALWORLD_SUITE)

    expect(byFrequency.size).toBeGreaterThan(0)
  })

  it('should have weekly and monthly tasks', () => {
    const byFrequency = getTasksByFrequency(TIER3_REALWORLD_SUITE)

    expect(byFrequency.has('weekly') || byFrequency.has('monthly')).toBe(true)
  })

  it('frequency distributions should sum to 9 tasks', () => {
    const byFrequency = getTasksByFrequency(TIER3_REALWORLD_SUITE)
    let total = 0

    for (const tasks of byFrequency.values()) {
      total += tasks.length
    }

    expect(total).toBe(9)
  })
})

describe('getSuiteStatistics', () => {
  it('should calculate correct overall statistics', () => {
    const stats = getSuiteStatistics(TIER3_REALWORLD_SUITE)

    expect(stats.totalTasks).toBe(9)
    expect(stats.byCategory.size).toBe(3)
    expect(stats.byDifficulty.size).toBeGreaterThan(0)
  })

  it('should have category statistics for each category', () => {
    const stats = getSuiteStatistics(TIER3_REALWORLD_SUITE)

    expect(stats.byCategory.has('code-review')).toBe(true)
    expect(stats.byCategory.has('debugging')).toBe(true)
    expect(stats.byCategory.has('refactoring')).toBe(true)
  })

  it('category statistics should have correct structure', () => {
    const stats = getSuiteStatistics(TIER3_REALWORLD_SUITE)
    const codeReview = stats.byCategory.get('code-review')

    expect(codeReview).toBeDefined()
    expect(codeReview?.category).toBe('code-review')
    expect(codeReview?.taskCount).toBe(3)
    expect(codeReview?.taskIds).toHaveLength(3)
    expect(codeReview?.difficultyDistribution).toBeDefined()
    expect(codeReview?.frequencyDistribution).toBeDefined()
  })

  it('difficulty statistics should have correct structure', () => {
    const stats = getSuiteStatistics(TIER3_REALWORLD_SUITE)
    const difficulties = Array.from(stats.byDifficulty.values())

    expect(difficulties.length).toBeGreaterThan(0)

    for (const diff of difficulties) {
      expect(diff.difficulty).toBeDefined()
      expect(diff.taskCount).toBeGreaterThan(0)
      expect(diff.taskIds.length).toBe(diff.taskCount)
      expect(diff.categories.length).toBeGreaterThan(0)
    }
  })

  it('should have frequency distribution', () => {
    const stats = getSuiteStatistics(TIER3_REALWORLD_SUITE)

    expect(stats.frequencyDistribution).toBeDefined()
    expect(Object.keys(stats.frequencyDistribution).length).toBeGreaterThan(0)
  })

  it('should have scenario types', () => {
    const stats = getSuiteStatistics(TIER3_REALWORLD_SUITE)

    expect(stats.scenarioTypes).toBeDefined()
    expect(stats.scenarioTypes.length).toBe(3)
  })
})

describe('Tier 3 Suite Characteristics', () => {
  interface Tier3Task extends SearchTask {
    tier: string
    realWorldScenario: string
    basedOnRealScenario: boolean
    frequency: string
  }

  it('tasks should NOT have expectedGrepSuccess', () => {
    for (const task of TIER3_REALWORLD_SUITE.tasks) {
      expect((task as Tier3Task).expectedGrepSuccess).toBeUndefined()
    }
  })

  it('tasks should NOT have expectedSearchSuccess', () => {
    for (const task of TIER3_REALWORLD_SUITE.tasks) {
      expect((task as Tier3Task).expectedSearchSuccess).toBeUndefined()
    }
  })

  it('tasks should have basedOnRealScenario flag', () => {
    for (const task of TIER3_REALWORLD_SUITE.tasks) {
      expect((task as Tier3Task).basedOnRealScenario).toBe(true)
    }
  })

  it('tasks should have frequency classification', () => {
    const validFrequencies = ['daily', 'weekly', 'monthly', 'rare']

    for (const task of TIER3_REALWORLD_SUITE.tasks) {
      expect(validFrequencies).toContain((task as Tier3Task).frequency)
    }
  })

  it('tasks should have tier3-realworld tier', () => {
    for (const task of TIER3_REALWORLD_SUITE.tasks) {
      expect((task as Tier3Task).tier).toBe('tier3-realworld')
    }
  })
})
