/**
 * Tests for leaderboard tracking system
 */

import { mkdirSync, rmSync, existsSync } from 'fs'
import { join } from 'path'
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import type { Variant } from '../../../../../maproom-mcp/test/tool-description-optimization/types.js'
import type { MultiTierScore } from '../../multi-tier-scoring.js'
import {
  loadLeaderboard,
  saveLeaderboard,
  updateLeaderboard,
  saveToLeaderboard,
  getLeaderboardEntry,
  getLeaderboardEntryByVariantId,
  generateLeaderboardReport,
  type Leaderboard,
} from '../leaderboard.js'

const TEST_BASE_DIR = join('/tmp', 'tracking-test-leaderboard')

// Helper to create a mock variant
function createMockVariant(id: string, name: string, generation: number): Variant {
  return {
    id,
    name,
    description: `Mock variant ${name}`,
    tokens: 500,
    generation,
    parent_ids: [],
    created_at: new Date(),
  }
}

// Helper to create a mock multi-tier score
function createMockScore(composite: number): MultiTierScore {
  return {
    composite,
    tierMetrics: {
      tier1: { avgScore: composite * 0.9, searchUsageRate: 0.8, appropriateUsage: 0.8, taskCompletionRate: 0.7 },
      tier2: { avgScore: composite * 0.95, searchUsageRate: 0.85, appropriateUsage: 0.85, taskCompletionRate: 0.75 },
      tier3: { avgScore: composite * 1.05, searchUsageRate: 0.5, appropriateUsage: 0.9, taskCompletionRate: 0.8 },
    },
    toolSelection: {
      tier1SearchRate: 0.8,
      tier2SearchRate: 0.85,
      tier3SearchRate: 0.5,
      tier1Accuracy: 0.8,
      tier2Accuracy: 0.85,
      tier3Accuracy: 0.9,
      overallAccuracy: 0.85,
    },
    taskCoverage: {
      total: 30,
      passed: 22,
    },
  }
}

describe('Leaderboard System', () => {
  beforeEach(() => {
    // Clean up test directory
    if (existsSync(TEST_BASE_DIR)) {
      rmSync(TEST_BASE_DIR, { recursive: true, force: true })
    }
    mkdirSync(TEST_BASE_DIR, { recursive: true })
  })

  afterEach(() => {
    // Clean up after tests
    if (existsSync(TEST_BASE_DIR)) {
      rmSync(TEST_BASE_DIR, { recursive: true, force: true })
    }
  })

  it('should initialize empty leaderboard', () => {
    const leaderboard = loadLeaderboard(TEST_BASE_DIR)

    expect(leaderboard.allTimeTopVariants).toHaveLength(0)
    expect(leaderboard.productionVariant).toBeNull()
    expect(leaderboard.productionDeployedAt).toBeNull()
    expect(leaderboard.schemaVersion).toBe(1)
  })

  it('should save and load leaderboard', () => {
    const leaderboard: Leaderboard = {
      schemaVersion: 1,
      allTimeTopVariants: [],
      productionVariant: null,
      productionDeployedAt: null,
      lastUpdated: new Date(),
    }

    saveLeaderboard(leaderboard, TEST_BASE_DIR)

    const loaded = loadLeaderboard(TEST_BASE_DIR)
    expect(loaded.schemaVersion).toBe(1)
    expect(loaded.allTimeTopVariants).toHaveLength(0)
  })

  it('should update leaderboard with new variant', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)
    const score = createMockScore(0.85)

    const updated = updateLeaderboard(variant, score, 'run-123', true, TEST_BASE_DIR)

    expect(updated.allTimeTopVariants).toHaveLength(1)
    expect(updated.allTimeTopVariants[0].variantId).toBe('v1')
    expect(updated.allTimeTopVariants[0].rank).toBe(1)
    expect(updated.allTimeTopVariants[0].compositeScore).toBe(0.85)
    expect(updated.allTimeTopVariants[0].converged).toBe(true)
  })

  it('should maintain top 10 limit', () => {
    // Add 15 variants
    for (let i = 1; i <= 15; i++) {
      const variant = createMockVariant(`v${i}`, `Variant ${i}`, 1)
      const score = createMockScore(0.5 + i * 0.01) // Increasing scores
      updateLeaderboard(variant, score, `run-${i}`, false, TEST_BASE_DIR)
    }

    const leaderboard = loadLeaderboard(TEST_BASE_DIR)

    // Should only keep top 10
    expect(leaderboard.allTimeTopVariants).toHaveLength(10)

    // Should be sorted by score (highest first)
    for (let i = 0; i < 9; i++) {
      expect(leaderboard.allTimeTopVariants[i].compositeScore).toBeGreaterThanOrEqual(
        leaderboard.allTimeTopVariants[i + 1].compositeScore,
      )
    }

    // Best variant should be at rank 1
    expect(leaderboard.allTimeTopVariants[0].rank).toBe(1)
    expect(leaderboard.allTimeTopVariants[0].variantId).toBe('v15')
  })

  it('should correctly rank variants', () => {
    const variant1 = createMockVariant('v1', 'Variant 1', 1)
    const variant2 = createMockVariant('v2', 'Variant 2', 1)
    const variant3 = createMockVariant('v3', 'Variant 3', 1)

    updateLeaderboard(variant1, createMockScore(0.7), 'run-1', false, TEST_BASE_DIR)
    updateLeaderboard(variant2, createMockScore(0.9), 'run-2', false, TEST_BASE_DIR)
    updateLeaderboard(variant3, createMockScore(0.8), 'run-3', false, TEST_BASE_DIR)

    const leaderboard = loadLeaderboard(TEST_BASE_DIR)

    expect(leaderboard.allTimeTopVariants[0].variantId).toBe('v2') // 0.9
    expect(leaderboard.allTimeTopVariants[0].rank).toBe(1)
    expect(leaderboard.allTimeTopVariants[1].variantId).toBe('v3') // 0.8
    expect(leaderboard.allTimeTopVariants[1].rank).toBe(2)
    expect(leaderboard.allTimeTopVariants[2].variantId).toBe('v1') // 0.7
    expect(leaderboard.allTimeTopVariants[2].rank).toBe(3)
  })

  it('should not save variant that does not qualify for top 10', () => {
    // Fill leaderboard with 10 variants scoring 0.6-0.69
    for (let i = 0; i < 10; i++) {
      const variant = createMockVariant(`v${i}`, `Variant ${i}`, 1)
      const score = createMockScore(0.6 + i * 0.01)
      updateLeaderboard(variant, score, `run-${i}`, false, TEST_BASE_DIR)
    }

    // Try to add variant with lower score
    const lowVariant = createMockVariant('v-low', 'Low Variant', 1)
    const lowScore = createMockScore(0.5)

    const result = saveToLeaderboard(lowVariant, lowScore, 'run-low', false, TEST_BASE_DIR)

    expect(result).toBeNull()

    const leaderboard = loadLeaderboard(TEST_BASE_DIR)
    expect(leaderboard.allTimeTopVariants).toHaveLength(10)
    expect(leaderboard.allTimeTopVariants.find((e) => e.variantId === 'v-low')).toBeUndefined()
  })

  it('should save variant that qualifies for top 10', () => {
    // Fill leaderboard with 10 variants scoring 0.6-0.69
    for (let i = 0; i < 10; i++) {
      const variant = createMockVariant(`v${i}`, `Variant ${i}`, 1)
      const score = createMockScore(0.6 + i * 0.01)
      updateLeaderboard(variant, score, `run-${i}`, false, TEST_BASE_DIR)
    }

    // Add variant with higher score
    const highVariant = createMockVariant('v-high', 'High Variant', 1)
    const highScore = createMockScore(0.8)

    const result = saveToLeaderboard(highVariant, highScore, 'run-high', false, TEST_BASE_DIR)

    expect(result).not.toBeNull()
    expect(result?.allTimeTopVariants).toHaveLength(10)
    expect(result?.allTimeTopVariants[0].variantId).toBe('v-high')
  })

  it('should get leaderboard entry by rank', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)
    const score = createMockScore(0.85)
    updateLeaderboard(variant, score, 'run-1', false, TEST_BASE_DIR)

    const entry = getLeaderboardEntry(1, TEST_BASE_DIR)

    expect(entry).not.toBeNull()
    expect(entry?.variantId).toBe('v1')
    expect(entry?.rank).toBe(1)
  })

  it('should get leaderboard entry by variant ID', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)
    const score = createMockScore(0.85)
    updateLeaderboard(variant, score, 'run-1', false, TEST_BASE_DIR)

    const entry = getLeaderboardEntryByVariantId('v1', TEST_BASE_DIR)

    expect(entry).not.toBeNull()
    expect(entry?.variantId).toBe('v1')
    expect(entry?.rank).toBe(1)
  })

  it('should return null for invalid rank', () => {
    const entry = getLeaderboardEntry(1, TEST_BASE_DIR)
    expect(entry).toBeNull()
  })

  it('should return null for non-existent variant ID', () => {
    const entry = getLeaderboardEntryByVariantId('non-existent', TEST_BASE_DIR)
    expect(entry).toBeNull()
  })

  it('should generate leaderboard report', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)
    const score = createMockScore(0.85)
    updateLeaderboard(variant, score, 'run-1', true, TEST_BASE_DIR)

    const report = generateLeaderboardReport(TEST_BASE_DIR)

    expect(report).toContain('GENETIC OPTIMIZATION LEADERBOARD')
    expect(report).toContain('Variant 1')
    expect(report).toContain('85.0%')
    expect(report).toContain('Converged: Yes')
  })

  it('should store task coverage in leaderboard entry', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)
    const score = createMockScore(0.85)

    updateLeaderboard(variant, score, 'run-1', false, TEST_BASE_DIR)

    const entry = getLeaderboardEntry(1, TEST_BASE_DIR)

    expect(entry?.taskCoverage.total).toBe(30)
    expect(entry?.taskCoverage.passed).toBe(22)
  })

  it('should store tool selection accuracy in leaderboard entry', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)
    const score = createMockScore(0.85)

    updateLeaderboard(variant, score, 'run-1', false, TEST_BASE_DIR)

    const entry = getLeaderboardEntry(1, TEST_BASE_DIR)

    expect(entry?.toolSelectionAccuracy).toBe(0.85)
  })

  it('should preserve timestamps correctly', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)
    const score = createMockScore(0.85)
    const beforeTime = new Date()

    updateLeaderboard(variant, score, 'run-1', false, TEST_BASE_DIR)

    const afterTime = new Date()
    const entry = getLeaderboardEntry(1, TEST_BASE_DIR)

    expect(entry?.timestamp).toBeInstanceOf(Date)
    expect(entry?.timestamp.getTime()).toBeGreaterThanOrEqual(beforeTime.getTime())
    expect(entry?.timestamp.getTime()).toBeLessThanOrEqual(afterTime.getTime())
  })
})
