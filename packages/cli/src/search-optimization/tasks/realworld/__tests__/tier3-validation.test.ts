/**
 * Tier 3 Real-World Task Validation Tests
 *
 * Comprehensive validation of all 9 Tier 3 tasks across 5 quality dimensions:
 * 1. Task structure and metadata
 * 2. Real scenario verification
 * 3. Success criteria objectivity
 * 4. Tool-agnostic descriptions (no coercion)
 * 5. Ecological validity
 *
 * 20+ tests ensuring all tasks meet Tier 3 requirements.
 */

import { describe, expect, it } from 'vitest'
import { TIER3_REALWORLD_SUITE } from '../../../benchmarks/tier3-realworld.js'
import type { SearchTask } from '../../../types.js'
import { validateEcologicalValidity } from '../../../validation/ecological.js'
import {
  TASK_API_IMPACT_ANALYSIS,
  TASK_AUTH_PERMISSION_CHECK,
  TASK_CACHE_INVALIDATION,
  TASK_DATABASE_MIGRATION_SAFETY,
  TASK_DEPRECATE_FUNCTION,
  TASK_DUPLICATE_ENTRIES,
  TASK_ERROR_HANDLING_CONSISTENCY,
  TASK_EXTRACT_PATTERN,
  TASK_INTERMITTENT_TIMEOUT,
  TIER3_REALWORLD_TASKS,
} from '../index.js'

describe('Tier 3 Real-World Tasks - Structure Validation', () => {
  it('should have exactly 9 tasks', () => {
    expect(TIER3_REALWORLD_TASKS).toHaveLength(9)
  })

  it('should have 3 code-review tasks', () => {
    const codeReviewTasks = TIER3_REALWORLD_TASKS.filter((task) => task.category === 'code-review')
    expect(codeReviewTasks).toHaveLength(3)
  })

  it('should have 3 debugging tasks', () => {
    const debuggingTasks = TIER3_REALWORLD_TASKS.filter((task) => task.category === 'debugging')
    expect(debuggingTasks).toHaveLength(3)
  })

  it('should have 3 refactoring tasks', () => {
    const refactoringTasks = TIER3_REALWORLD_TASKS.filter((task) => task.category === 'refactoring')
    expect(refactoringTasks).toHaveLength(3)
  })

  it('all tasks should have tier3-realworld tier', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect((task as SearchTask & Record<string, unknown>).tier).toBe('tier3-realworld')
    }
  })

  it('all tasks should have valid difficulty levels', () => {
    const validDifficulties = ['easy', 'medium']
    for (const task of TIER3_REALWORLD_TASKS) {
      expect(validDifficulties).toContain(task.difficulty)
    }
  })

  it('all tasks should have unique IDs', () => {
    const ids = TIER3_REALWORLD_TASKS.map((task) => task.id)
    const uniqueIds = new Set(ids)
    expect(uniqueIds.size).toBe(TIER3_REALWORLD_TASKS.length)
  })

  it('all task IDs should follow tier3-{category}-{name} pattern', () => {
    const pattern = /^tier3-(code-review|debugging|refactoring)-[a-z-]+$/
    for (const task of TIER3_REALWORLD_TASKS) {
      expect(task.id).toMatch(pattern)
    }
  })
})

describe('Tier 3 Real-World Tasks - Real Scenario Validation', () => {
  it('all tasks should have basedOnRealScenario flag set to true', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect((task as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    }
  })

  it('all tasks should have frequency classification', () => {
    const validFrequencies = ['daily', 'weekly', 'monthly', 'rare']
    for (const task of TIER3_REALWORLD_TASKS) {
      const frequency = (task as SearchTask & Record<string, unknown>).frequency
      expect(validFrequencies).toContain(frequency)
    }
  })

  it('all tasks should have realWorldScenario description', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect((task as SearchTask & Record<string, unknown>).realWorldScenario).toBeDefined()
      expect(typeof (task as SearchTask & Record<string, unknown>).realWorldScenario).toBe('string')
      expect((task as SearchTask & Record<string, unknown>).realWorldScenario).toContain('Based on')
    }
  })

  it('tasks should cover common developer frequencies', () => {
    const frequencies = TIER3_REALWORLD_TASKS.map((task) => (task as SearchTask & Record<string, unknown>).frequency)

    // Should have weekly and monthly tasks (most common)
    expect(frequencies).toContain('weekly')
    expect(frequencies).toContain('monthly')
  })
})

describe('Tier 3 Real-World Tasks - Tool Neutrality Validation', () => {
  it('descriptions should not mention "semantic search"', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect(task.description.toLowerCase()).not.toContain('semantic search')
    }
  })

  it('descriptions should not mention "grep"', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect(task.description.toLowerCase()).not.toContain('grep')
    }
  })

  it('descriptions should not hint at specific tools', () => {
    const toolHints = ['use search', 'use grep', 'search for', 'grep for', 'try using']
    for (const task of TIER3_REALWORLD_TASKS) {
      const lowerDesc = task.description.toLowerCase()
      for (const hint of toolHints) {
        expect(lowerDesc).not.toContain(hint)
      }
    }
  })

  it('tasks should NOT have expectedGrepSuccess field', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect((task as SearchTask & Record<string, unknown>).expectedGrepSuccess).toBeUndefined()
    }
  })

  it('tasks should NOT have expectedSearchSuccess field', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect((task as SearchTask & Record<string, unknown>).expectedSearchSuccess).toBeUndefined()
    }
  })
})

describe('Tier 3 Real-World Tasks - Success Criteria Validation', () => {
  it('all tasks should have objective validators', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect(task.followUpTask.validator).toBeDefined()
      expect(task.followUpTask.validator.type).toBeDefined()
    }
  })

  it('all tasks should have pattern-based success criteria', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      const validator = task.followUpTask.validator
      expect(validator.mentionsPattern).toBeDefined()
      expect(validator.mentionsPattern).toBeInstanceOf(RegExp)
    }
  })

  it('all tasks should have searchTarget patterns', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect(task.searchTarget.type).toBe('pattern')
      expect(task.searchTarget.pattern).toBeInstanceOf(RegExp)
    }
  })

  it('all tasks should have successValidator function', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect(task.successValidator).toBeDefined()
      expect(typeof task.successValidator).toBe('function')
    }
  })
})

describe('Tier 3 Real-World Tasks - Ecological Validity', () => {
  it('all tasks should pass ecological validation', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      const result = validateEcologicalValidity(task)
      expect(result.passed).toBe(true)
    }
  })

  it('all tasks should have ecological score >= 60%', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      const result = validateEcologicalValidity(task)
      expect(result.score).toBeGreaterThanOrEqual(0.6)
    }
  })

  it('all tasks should have clear descriptions', () => {
    for (const task of TIER3_REALWORLD_TASKS) {
      expect(task.description.length).toBeGreaterThan(50)
      expect(task.description).toContain('.')
    }
  })
})

describe('Tier 3 Real-World Tasks - Individual Task Validation', () => {
  describe('Code Review Tasks', () => {
    it('TASK_AUTH_PERMISSION_CHECK should be properly configured', () => {
      expect(TASK_AUTH_PERMISSION_CHECK.id).toBe('tier3-code-review-auth-permissions')
      expect(TASK_AUTH_PERMISSION_CHECK.category).toBe('code-review')
      expect(TASK_AUTH_PERMISSION_CHECK.difficulty).toBe('medium')
      expect((TASK_AUTH_PERMISSION_CHECK as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })

    it('TASK_DATABASE_MIGRATION_SAFETY should be properly configured', () => {
      expect(TASK_DATABASE_MIGRATION_SAFETY.id).toBe('tier3-code-review-db-migration')
      expect(TASK_DATABASE_MIGRATION_SAFETY.category).toBe('code-review')
      expect(TASK_DATABASE_MIGRATION_SAFETY.difficulty).toBe('medium')
      expect((TASK_DATABASE_MIGRATION_SAFETY as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })

    it('TASK_ERROR_HANDLING_CONSISTENCY should be properly configured', () => {
      expect(TASK_ERROR_HANDLING_CONSISTENCY.id).toBe('tier3-code-review-error-consistency')
      expect(TASK_ERROR_HANDLING_CONSISTENCY.category).toBe('code-review')
      expect(TASK_ERROR_HANDLING_CONSISTENCY.difficulty).toBe('easy')
      expect((TASK_ERROR_HANDLING_CONSISTENCY as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })
  })

  describe('Debugging Tasks', () => {
    it('TASK_INTERMITTENT_TIMEOUT should be properly configured', () => {
      expect(TASK_INTERMITTENT_TIMEOUT.id).toBe('tier3-debugging-timeout')
      expect(TASK_INTERMITTENT_TIMEOUT.category).toBe('debugging')
      expect(TASK_INTERMITTENT_TIMEOUT.difficulty).toBe('medium')
      expect((TASK_INTERMITTENT_TIMEOUT as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })

    it('TASK_DUPLICATE_ENTRIES should be properly configured', () => {
      expect(TASK_DUPLICATE_ENTRIES.id).toBe('tier3-debugging-duplicates')
      expect(TASK_DUPLICATE_ENTRIES.category).toBe('debugging')
      expect(TASK_DUPLICATE_ENTRIES.difficulty).toBe('medium')
      expect((TASK_DUPLICATE_ENTRIES as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })

    it('TASK_CACHE_INVALIDATION should be properly configured', () => {
      expect(TASK_CACHE_INVALIDATION.id).toBe('tier3-debugging-cache')
      expect(TASK_CACHE_INVALIDATION.category).toBe('debugging')
      expect(TASK_CACHE_INVALIDATION.difficulty).toBe('medium')
      expect((TASK_CACHE_INVALIDATION as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })
  })

  describe('Refactoring Tasks', () => {
    it('TASK_DEPRECATE_FUNCTION should be properly configured', () => {
      expect(TASK_DEPRECATE_FUNCTION.id).toBe('tier3-refactoring-deprecate')
      expect(TASK_DEPRECATE_FUNCTION.category).toBe('refactoring')
      expect(TASK_DEPRECATE_FUNCTION.difficulty).toBe('easy')
      expect((TASK_DEPRECATE_FUNCTION as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })

    it('TASK_EXTRACT_PATTERN should be properly configured', () => {
      expect(TASK_EXTRACT_PATTERN.id).toBe('tier3-refactoring-extract')
      expect(TASK_EXTRACT_PATTERN.category).toBe('refactoring')
      expect(TASK_EXTRACT_PATTERN.difficulty).toBe('medium')
      expect((TASK_EXTRACT_PATTERN as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })

    it('TASK_API_IMPACT_ANALYSIS should be properly configured', () => {
      expect(TASK_API_IMPACT_ANALYSIS.id).toBe('tier3-refactoring-api-impact')
      expect(TASK_API_IMPACT_ANALYSIS.category).toBe('refactoring')
      expect(TASK_API_IMPACT_ANALYSIS.difficulty).toBe('medium')
      expect((TASK_API_IMPACT_ANALYSIS as SearchTask & Record<string, unknown>).basedOnRealScenario).toBe(true)
    })
  })
})

describe('Tier 3 Benchmark Suite Validation', () => {
  it('suite should contain all 9 tasks', () => {
    expect(TIER3_REALWORLD_SUITE.tasks).toHaveLength(9)
  })

  it('suite should be tier 3', () => {
    expect(TIER3_REALWORLD_SUITE.tier).toBe(3)
  })

  it('suite should have correct name', () => {
    expect(TIER3_REALWORLD_SUITE.name).toBe('Tier 3: Real-World Tasks')
  })

  it('suite should have version', () => {
    expect(TIER3_REALWORLD_SUITE.version).toBeDefined()
    expect(TIER3_REALWORLD_SUITE.version).toMatch(/^\d+\.\d+\.\d+$/)
  })

  it('suite metadata should have correct task count', () => {
    expect(TIER3_REALWORLD_SUITE.metadata.totalTasks).toBe(9)
  })

  it('suite metadata should have all 3 categories', () => {
    expect(TIER3_REALWORLD_SUITE.metadata.categories).toContain('code-review')
    expect(TIER3_REALWORLD_SUITE.metadata.categories).toContain('debugging')
    expect(TIER3_REALWORLD_SUITE.metadata.categories).toContain('refactoring')
  })

  it('suite should have frequency distribution', () => {
    expect(TIER3_REALWORLD_SUITE.metadata.frequencyDistribution).toBeDefined()
    expect(Object.keys(TIER3_REALWORLD_SUITE.metadata.frequencyDistribution).length).toBeGreaterThan(0)
  })

  it('suite should have scenario types', () => {
    expect(TIER3_REALWORLD_SUITE.metadata.scenarioTypes).toBeDefined()
    expect(TIER3_REALWORLD_SUITE.metadata.scenarioTypes.length).toBeGreaterThan(0)
  })
})
