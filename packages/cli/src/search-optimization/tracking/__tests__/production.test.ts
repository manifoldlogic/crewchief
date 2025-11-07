/**
 * Tests for production variant management system
 */

import { mkdirSync, rmSync, existsSync, readFileSync } from 'fs'
import { join } from 'path'
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import type { Variant } from '../../../../../maproom-mcp/test/tool-description-optimization/types.js'
import { loadLeaderboard } from '../leaderboard.js'
import {
  getCurrentProduction,
  promoteToProduction,
  rollbackProduction,
  loadProductionVariant,
  getProductionHistory,
  generateProductionReport,
} from '../production.js'

const TEST_BASE_DIR = join('/tmp', 'tracking-test-production')

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

describe('Production Variant System', () => {
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

  it('should return null when no production variant exists', () => {
    const current = getCurrentProduction(TEST_BASE_DIR)
    expect(current).toBeNull()
  })

  it('should promote variant to production', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)

    const pointer = promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    expect(pointer.currentVariantId).toBe('v1')
    expect(pointer.deployedBy).toBe('test-user')
    expect(pointer.reason).toBe('Initial deployment')
    expect(pointer.previousVariantId).toBeUndefined()
  })

  it('should update leaderboard when promoting to production', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)

    promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    const leaderboard = loadLeaderboard(TEST_BASE_DIR)
    expect(leaderboard.productionVariant).toBe('v1')
    expect(leaderboard.productionDeployedAt).toBeInstanceOf(Date)
  })

  it('should save variant JSON to production directory', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)

    promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    const variantPath = join(TEST_BASE_DIR, 'production', 'variants', 'v1.json')
    expect(existsSync(variantPath)).toBe(true)

    const saved = JSON.parse(readFileSync(variantPath, 'utf-8'))
    expect(saved.id).toBe('v1')
    expect(saved.name).toBe('Variant 1')
  })

  it('should track previous variant ID on promotion', () => {
    const variant1 = createMockVariant('v1', 'Variant 1', 1)
    const variant2 = createMockVariant('v2', 'Variant 2', 2)

    promoteToProduction(variant1, 'Initial deployment', 'test-user', TEST_BASE_DIR)
    const pointer = promoteToProduction(variant2, 'Upgrade', 'test-user', TEST_BASE_DIR)

    expect(pointer.currentVariantId).toBe('v2')
    expect(pointer.previousVariantId).toBe('v1')
  })

  it('should append to deployment log', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)

    promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    const logPath = join(TEST_BASE_DIR, 'production', 'deployment-log.md')
    expect(existsSync(logPath)).toBe(true)

    const log = readFileSync(logPath, 'utf-8')
    expect(log).toContain('# Production Deployment Log')
    expect(log).toContain('Deployment:')
    expect(log).toContain('v1')
    expect(log).toContain('Initial deployment')
    expect(log).toContain('test-user')
  })

  it('should load current production variant', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)

    promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    const loaded = loadProductionVariant(TEST_BASE_DIR)

    expect(loaded).not.toBeNull()
    expect(loaded?.id).toBe('v1')
    expect(loaded?.name).toBe('Variant 1')
  })

  it('should get production history', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)

    promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    const history = getProductionHistory(TEST_BASE_DIR)

    expect(history).not.toBeNull()
    expect(history).toContain('Production Deployment Log')
    expect(history).toContain('v1')
  })

  it('should generate production report', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)

    promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    const report = generateProductionReport(TEST_BASE_DIR)

    expect(report).toContain('PRODUCTION VARIANT STATUS')
    expect(report).toContain('Variant 1')
    expect(report).toContain('v1')
    expect(report).toContain('test-user')
    expect(report).toContain('Initial deployment')
  })

  it('should handle rollback to previous variant', () => {
    const variant1 = createMockVariant('v1', 'Variant 1', 1)
    const variant2 = createMockVariant('v2', 'Variant 2', 2)

    promoteToProduction(variant1, 'Initial deployment', 'test-user', TEST_BASE_DIR)
    promoteToProduction(variant2, 'Upgrade', 'test-user', TEST_BASE_DIR)

    const pointer = rollbackProduction('Bug found', 'test-user', TEST_BASE_DIR)

    expect(pointer.currentVariantId).toBe('v1')
    expect(pointer.previousVariantId).toBe('v2')
    expect(pointer.reason).toBe('Bug found')
  })

  it('should update leaderboard on rollback', () => {
    const variant1 = createMockVariant('v1', 'Variant 1', 1)
    const variant2 = createMockVariant('v2', 'Variant 2', 2)

    promoteToProduction(variant1, 'Initial deployment', 'test-user', TEST_BASE_DIR)
    promoteToProduction(variant2, 'Upgrade', 'test-user', TEST_BASE_DIR)
    rollbackProduction('Bug found', 'test-user', TEST_BASE_DIR)

    const leaderboard = loadLeaderboard(TEST_BASE_DIR)
    expect(leaderboard.productionVariant).toBe('v1')
  })

  it('should append rollback to deployment log', () => {
    const variant1 = createMockVariant('v1', 'Variant 1', 1)
    const variant2 = createMockVariant('v2', 'Variant 2', 2)

    promoteToProduction(variant1, 'Initial deployment', 'test-user', TEST_BASE_DIR)
    promoteToProduction(variant2, 'Upgrade', 'test-user', TEST_BASE_DIR)
    rollbackProduction('Bug found', 'test-user', TEST_BASE_DIR)

    const log = getProductionHistory(TEST_BASE_DIR)

    expect(log).toContain('Rollback:')
    expect(log).toContain('Bug found')
    expect(log).toContain('v1')
  })

  it('should throw error when rolling back with no production variant', () => {
    expect(() => {
      rollbackProduction('Test', 'test-user', TEST_BASE_DIR)
    }).toThrow('No production variant to rollback from')
  })

  it('should throw error when rolling back with no previous variant', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)

    promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    expect(() => {
      rollbackProduction('Test', 'test-user', TEST_BASE_DIR)
    }).toThrow('No previous variant ID available for rollback')
  })

  it('should support rolling back from a rollback', () => {
    const variant1 = createMockVariant('v1', 'Variant 1', 1)
    const variant2 = createMockVariant('v2', 'Variant 2', 2)

    promoteToProduction(variant1, 'Initial deployment', 'test-user', TEST_BASE_DIR)
    promoteToProduction(variant2, 'Upgrade', 'test-user', TEST_BASE_DIR)
    rollbackProduction('Bug found', 'test-user', TEST_BASE_DIR)

    // Should be able to roll back to v2 again
    const pointer = rollbackProduction('False alarm', 'test-user', TEST_BASE_DIR)

    expect(pointer.currentVariantId).toBe('v2')
  })

  it('should preserve timestamps correctly', () => {
    const variant = createMockVariant('v1', 'Variant 1', 1)
    const beforeTime = new Date()

    const pointer = promoteToProduction(variant, 'Initial deployment', 'test-user', TEST_BASE_DIR)

    const afterTime = new Date()

    expect(pointer.deployedAt).toBeInstanceOf(Date)
    expect(pointer.deployedAt.getTime()).toBeGreaterThanOrEqual(beforeTime.getTime())
    expect(pointer.deployedAt.getTime()).toBeLessThanOrEqual(afterTime.getTime())
  })

  it('should handle production report with no variant', () => {
    const report = generateProductionReport(TEST_BASE_DIR)

    expect(report).toContain('PRODUCTION VARIANT STATUS')
    expect(report).toContain('No production variant currently deployed')
  })

  it('should return null for production history when no log exists', () => {
    const history = getProductionHistory(TEST_BASE_DIR)
    expect(history).toBeNull()
  })

  it('should return null when loading non-existent production variant', () => {
    const variant = loadProductionVariant(TEST_BASE_DIR)
    expect(variant).toBeNull()
  })
})
