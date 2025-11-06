/**
 * Tests for A/B Test Variant Assignment
 */

import { describe, it, expect } from 'vitest'
import {
  hashToBucket,
  assignVariant,
  create50_50Split,
  create90_10Split,
  testAssignmentStability,
  analyzeDistribution,
  type TrafficSplit
} from './assigner.js'

describe('hashToBucket', () => {
  it('should return deterministic bucket for same user', () => {
    const userId = 'user-123'
    const bucket1 = hashToBucket(userId)
    const bucket2 = hashToBucket(userId)
    expect(bucket1).toBe(bucket2)
  })

  it('should return bucket in range 0-99', () => {
    const buckets = Array.from({ length: 100 }, (_, i) => hashToBucket(`user-${i}`))
    for (const bucket of buckets) {
      expect(bucket).toBeGreaterThanOrEqual(0)
      expect(bucket).toBeLessThan(100)
    }
  })

  it('should produce different buckets for different users', () => {
    const bucket1 = hashToBucket('user-1')
    const bucket2 = hashToBucket('user-2')
    const bucket3 = hashToBucket('user-3')

    // Very unlikely all three are identical
    const uniqueBuckets = new Set([bucket1, bucket2, bucket3])
    expect(uniqueBuckets.size).toBeGreaterThan(1)
  })

  it('should handle special characters in user ID', () => {
    const bucket = hashToBucket('user@email.com')
    expect(bucket).toBeGreaterThanOrEqual(0)
    expect(bucket).toBeLessThan(100)
  })
})

describe('assignVariant', () => {
  const splits: TrafficSplit[] = [
    { variant_id: 'control', variant_name: 'Control', percentage: 50 },
    { variant_id: 'treatment', variant_name: 'Treatment', percentage: 50 }
  ]

  it('should assign variant based on user ID', () => {
    const assignment = assignVariant('user-123', splits)
    expect(['control', 'treatment']).toContain(assignment.variant_id)
    expect(assignment.bucket).toBeGreaterThanOrEqual(0)
    expect(assignment.bucket).toBeLessThan(100)
  })

  it('should assign same variant to same user consistently', () => {
    const assignment1 = assignVariant('user-456', splits)
    const assignment2 = assignVariant('user-456', splits)

    expect(assignment1.variant_id).toBe(assignment2.variant_id)
    expect(assignment1.bucket).toBe(assignment2.bucket)
  })

  it('should respect traffic split percentages', () => {
    const unbalancedSplits: TrafficSplit[] = [
      { variant_id: 'control', variant_name: 'Control', percentage: 90 },
      { variant_id: 'treatment', variant_name: 'Treatment', percentage: 10 }
    ]

    // Test with 1000 users
    const assignments = Array.from({ length: 1000 }, (_, i) =>
      assignVariant(`user-${i}`, unbalancedSplits)
    )

    const controlCount = assignments.filter(a => a.variant_id === 'control').length
    const treatmentCount = assignments.filter(a => a.variant_id === 'treatment').length

    // Should be roughly 90/10 split (allow for some variance)
    expect(controlCount).toBeGreaterThan(850)
    expect(controlCount).toBeLessThan(950)
    expect(treatmentCount).toBeGreaterThan(50)
    expect(treatmentCount).toBeLessThan(150)
  })

  it('should throw error if splits do not sum to 100', () => {
    const invalidSplits: TrafficSplit[] = [
      { variant_id: 'control', variant_name: 'Control', percentage: 50 },
      { variant_id: 'treatment', variant_name: 'Treatment', percentage: 40 }
    ]

    expect(() => assignVariant('user-123', invalidSplits)).toThrow('must sum to 100')
  })

  it('should throw error if splits array is empty', () => {
    expect(() => assignVariant('user-123', [])).toThrow('Must provide at least one variant')
  })

  it('should handle three-way split', () => {
    const threeWaySplits: TrafficSplit[] = [
      { variant_id: 'A', variant_name: 'A', percentage: 33.33 },
      { variant_id: 'B', variant_name: 'B', percentage: 33.33 },
      { variant_id: 'C', variant_name: 'C', percentage: 33.34 }
    ]

    const assignment = assignVariant('user-789', threeWaySplits)
    expect(['A', 'B', 'C']).toContain(assignment.variant_id)
  })
})

describe('create50_50Split', () => {
  it('should create 50/50 split configuration', () => {
    const splits = create50_50Split('control', 'treatment')

    expect(splits).toHaveLength(2)
    expect(splits[0].variant_id).toBe('control')
    expect(splits[0].percentage).toBe(50)
    expect(splits[1].variant_id).toBe('treatment')
    expect(splits[1].percentage).toBe(50)
  })
})

describe('create90_10Split', () => {
  it('should create 90/10 split configuration', () => {
    const splits = create90_10Split('control', 'experiment')

    expect(splits).toHaveLength(2)
    expect(splits[0].variant_id).toBe('control')
    expect(splits[0].percentage).toBe(90)
    expect(splits[1].variant_id).toBe('experiment')
    expect(splits[1].percentage).toBe(10)
  })
})

describe('testAssignmentStability', () => {
  const splits = create50_50Split('A', 'B')

  it('should confirm assignment stability for single user', () => {
    const stable = testAssignmentStability('user-123', splits)
    expect(stable).toBe(true)
  })

  it('should test stability over many iterations', () => {
    const stable = testAssignmentStability('user-456', splits, 1000)
    expect(stable).toBe(true)
  })

  it('should confirm stability with unbalanced splits', () => {
    const unbalancedSplits = create90_10Split('control', 'treatment')
    const stable = testAssignmentStability('user-789', unbalancedSplits, 500)
    expect(stable).toBe(true)
  })
})

describe('analyzeDistribution', () => {
  it('should count variant assignments', () => {
    const splits = create50_50Split('control', 'treatment')
    const userIds = Array.from({ length: 100 }, (_, i) => `user-${i}`)

    const distribution = analyzeDistribution(userIds, splits)

    expect(distribution.size).toBe(2)
    expect(distribution.has('control')).toBe(true)
    expect(distribution.has('treatment')).toBe(true)

    const controlCount = distribution.get('control') || 0
    const treatmentCount = distribution.get('treatment') || 0

    // Should be roughly balanced
    expect(controlCount + treatmentCount).toBe(100)
    expect(controlCount).toBeGreaterThan(30)
    expect(controlCount).toBeLessThan(70)
  })

  it('should handle 90/10 split distribution', () => {
    const splits = create90_10Split('control', 'treatment')
    const userIds = Array.from({ length: 1000 }, (_, i) => `user-${i}`)

    const distribution = analyzeDistribution(userIds, splits)

    const controlCount = distribution.get('control') || 0
    const treatmentCount = distribution.get('treatment') || 0

    expect(controlCount + treatmentCount).toBe(1000)

    // Should be roughly 90/10 (allow for variance)
    expect(controlCount).toBeGreaterThan(850)
    expect(controlCount).toBeLessThan(950)
    expect(treatmentCount).toBeGreaterThan(50)
    expect(treatmentCount).toBeLessThan(150)
  })

  it('should handle three-way split', () => {
    const splits: TrafficSplit[] = [
      { variant_id: 'A', variant_name: 'A', percentage: 33.33 },
      { variant_id: 'B', variant_name: 'B', percentage: 33.33 },
      { variant_id: 'C', variant_name: 'C', percentage: 33.34 }
    ]
    const userIds = Array.from({ length: 900 }, (_, i) => `user-${i}`)

    const distribution = analyzeDistribution(userIds, splits)

    expect(distribution.size).toBe(3)
    const aCount = distribution.get('A') || 0
    const bCount = distribution.get('B') || 0
    const cCount = distribution.get('C') || 0

    expect(aCount + bCount + cCount).toBe(900)

    // Each should be roughly 300 (allow for variance)
    expect(aCount).toBeGreaterThan(250)
    expect(aCount).toBeLessThan(350)
    expect(bCount).toBeGreaterThan(250)
    expect(bCount).toBeLessThan(350)
    expect(cCount).toBeGreaterThan(250)
    expect(cCount).toBeLessThan(350)
  })
})
