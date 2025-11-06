/**
 * Variant Assignment for A/B Testing
 *
 * Assigns users to variants using consistent hashing to ensure:
 * - Same user always gets same variant (persistence)
 * - Deterministic bucket assignment (0-99)
 * - Configurable traffic splits
 */

import { createHash } from 'node:crypto'

export interface VariantAssignment {
  variant_id: string
  variant_name: string
  bucket: number // 0-99
}

export interface TrafficSplit {
  variant_id: string
  variant_name: string
  percentage: number // 0-100
}

/**
 * Hash user_id to deterministic bucket (0-99)
 *
 * Uses SHA-256 to ensure uniform distribution across buckets
 */
export function hashToBucket(userId: string): number {
  const hash = createHash('sha256').update(userId).digest('hex')
  // Use first 8 hex characters (32 bits) for bucket assignment
  const hashInt = parseInt(hash.substring(0, 8), 16)
  return hashInt % 100
}

/**
 * Assign user to variant based on traffic split configuration
 *
 * @param userId - Unique user identifier (e.g., session_id, user_id)
 * @param splits - Array of traffic splits (must sum to 100)
 * @returns Assigned variant
 */
export function assignVariant(userId: string, splits: TrafficSplit[]): VariantAssignment {
  // Validate splits
  const totalPercentage = splits.reduce((sum, s) => sum + s.percentage, 0)
  if (Math.abs(totalPercentage - 100) > 0.01) {
    throw new Error(`Traffic splits must sum to 100, got ${totalPercentage}`)
  }

  if (splits.length === 0) {
    throw new Error('Must provide at least one variant split')
  }

  // Get deterministic bucket for user
  const bucket = hashToBucket(userId)

  // Determine which variant bucket falls into
  let cumulative = 0
  for (const split of splits) {
    cumulative += split.percentage
    if (bucket < cumulative) {
      return {
        variant_id: split.variant_id,
        variant_name: split.variant_name,
        bucket
      }
    }
  }

  // Fallback to last variant (handles floating point edge cases)
  const lastSplit = splits[splits.length - 1]
  return {
    variant_id: lastSplit.variant_id,
    variant_name: lastSplit.variant_name,
    bucket
  }
}

/**
 * Create 50/50 split configuration
 */
export function create50_50Split(variantA: string, variantB: string): TrafficSplit[] {
  return [
    { variant_id: variantA, variant_name: variantA, percentage: 50 },
    { variant_id: variantB, variant_name: variantB, percentage: 50 }
  ]
}

/**
 * Create 90/10 split (control vs experiment)
 */
export function create90_10Split(control: string, experiment: string): TrafficSplit[] {
  return [
    { variant_id: control, variant_name: control, percentage: 90 },
    { variant_id: experiment, variant_name: experiment, percentage: 10 }
  ]
}

/**
 * Test assignment stability (same user always gets same variant)
 */
export function testAssignmentStability(
  userId: string,
  splits: TrafficSplit[],
  iterations: number = 100
): boolean {
  const firstAssignment = assignVariant(userId, splits)

  for (let i = 0; i < iterations; i++) {
    const assignment = assignVariant(userId, splits)
    if (assignment.variant_id !== firstAssignment.variant_id) {
      return false
    }
  }

  return true
}

/**
 * Analyze traffic distribution across variants
 *
 * Useful for verifying hash function produces uniform distribution
 */
export function analyzeDistribution(
  userIds: string[],
  splits: TrafficSplit[]
): Map<string, number> {
  const counts = new Map<string, number>()

  for (const split of splits) {
    counts.set(split.variant_id, 0)
  }

  for (const userId of userIds) {
    const assignment = assignVariant(userId, splits)
    counts.set(assignment.variant_id, (counts.get(assignment.variant_id) || 0) + 1)
  }

  return counts
}
