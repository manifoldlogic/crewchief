/**
 * Leaderboard Tracking System
 *
 * Maintains a global leaderboard of top 10 variants across all optimization runs.
 * Provides atomic file operations to prevent data corruption during concurrent writes.
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync, renameSync } from 'fs'
import { join } from 'path'
import type { Variant } from '../../../../maproom-mcp/test/tool-description-optimization/types.js'
import type { MultiTierScore } from '../multi-tier-scoring.js'

/**
 * Schema version for leaderboard format
 */
export const LEADERBOARD_SCHEMA_VERSION = 1

/**
 * Entry in the leaderboard
 */
export interface LeaderboardEntry {
  rank: number
  variantId: string
  name: string
  compositeScore: number
  tierScores: {
    tier1: number
    tier2: number
    tier3: number
  }
  runId: string
  generation: number
  converged: boolean
  timestamp: Date
  taskCoverage: {
    total: number
    passed: number // Tasks with score >= 0.6
  }
  toolSelectionAccuracy: number // Overall tool selection accuracy (0-1)
}

/**
 * Global leaderboard structure
 */
export interface Leaderboard {
  schemaVersion: number
  allTimeTopVariants: LeaderboardEntry[]
  productionVariant: string | null
  productionDeployedAt: Date | null
  lastUpdated: Date
}

/**
 * Default leaderboard path
 */
export function getLeaderboardPath(baseDir = '.crewchief'): string {
  return join(baseDir, 'leaderboard.json')
}

/**
 * Load leaderboard from disk
 */
export function loadLeaderboard(baseDir = '.crewchief'): Leaderboard {
  const path = getLeaderboardPath(baseDir)

  if (!existsSync(path)) {
    // Initialize empty leaderboard
    return {
      schemaVersion: LEADERBOARD_SCHEMA_VERSION,
      allTimeTopVariants: [],
      productionVariant: null,
      productionDeployedAt: null,
      lastUpdated: new Date(),
    }
  }

  const content = readFileSync(path, 'utf-8')
  const data = JSON.parse(content)

  // Convert date strings back to Date objects
  return {
    ...data,
    allTimeTopVariants: data.allTimeTopVariants.map((entry: LeaderboardEntry) => ({
      ...entry,
      timestamp: new Date(entry.timestamp),
    })),
    productionDeployedAt: data.productionDeployedAt ? new Date(data.productionDeployedAt) : null,
    lastUpdated: new Date(data.lastUpdated),
  }
}

/**
 * Save leaderboard to disk using atomic write-then-rename
 */
export function saveLeaderboard(leaderboard: Leaderboard, baseDir = '.crewchief'): void {
  const path = getLeaderboardPath(baseDir)
  const dir = join(baseDir)

  // Ensure directory exists
  mkdirSync(dir, { recursive: true })

  // Write to temporary file first
  const tmpPath = `${path}.tmp`
  writeFileSync(tmpPath, JSON.stringify(leaderboard, null, 2))

  // Atomic rename
  renameSync(tmpPath, path)
}

/**
 * Update leaderboard with new variant result
 *
 * Inserts variant if it qualifies for top 10, maintains ranking
 */
export function updateLeaderboard(
  variant: Variant,
  multiTierScore: MultiTierScore,
  runId: string,
  converged: boolean,
  baseDir = '.crewchief',
): Leaderboard {
  const leaderboard = loadLeaderboard(baseDir)

  // Create new entry
  const newEntry: LeaderboardEntry = {
    rank: 0, // Will be assigned after sorting
    variantId: variant.id,
    name: variant.name,
    compositeScore: multiTierScore.composite,
    tierScores: {
      tier1: multiTierScore.tierMetrics.tier1.avgScore,
      tier2: multiTierScore.tierMetrics.tier2.avgScore,
      tier3: multiTierScore.tierMetrics.tier3.avgScore,
    },
    runId,
    generation: variant.generation,
    converged,
    timestamp: new Date(),
    taskCoverage: {
      total: multiTierScore.taskCoverage.total,
      passed: multiTierScore.taskCoverage.passed,
    },
    toolSelectionAccuracy: multiTierScore.toolSelection.overallAccuracy,
  }

  // Add to top variants
  leaderboard.allTimeTopVariants.push(newEntry)

  // Sort by composite score (highest first)
  leaderboard.allTimeTopVariants.sort((a, b) => b.compositeScore - a.compositeScore)

  // Keep only top 10
  leaderboard.allTimeTopVariants = leaderboard.allTimeTopVariants.slice(0, 10)

  // Reassign ranks
  leaderboard.allTimeTopVariants.forEach((entry, i) => {
    entry.rank = i + 1
  })

  // Update timestamp
  leaderboard.lastUpdated = new Date()

  // Save atomically
  saveLeaderboard(leaderboard, baseDir)

  return leaderboard
}

/**
 * Save variant to leaderboard if it qualifies for top 10
 *
 * @returns Updated leaderboard, or null if variant didn't qualify
 */
export function saveToLeaderboard(
  variant: Variant,
  multiTierScore: MultiTierScore,
  runId: string,
  converged: boolean,
  baseDir = '.crewchief',
): Leaderboard | null {
  const leaderboard = loadLeaderboard(baseDir)

  // Check if variant qualifies for top 10
  const wouldQualify =
    leaderboard.allTimeTopVariants.length < 10 ||
    multiTierScore.composite > leaderboard.allTimeTopVariants[leaderboard.allTimeTopVariants.length - 1].compositeScore

  if (!wouldQualify) {
    console.log(
      `Variant ${variant.name} (${(multiTierScore.composite * 100).toFixed(1)}%) did not qualify for leaderboard`,
    )
    return null
  }

  console.log(`Adding ${variant.name} to leaderboard (${(multiTierScore.composite * 100).toFixed(1)}%)`)

  return updateLeaderboard(variant, multiTierScore, runId, converged, baseDir)
}

/**
 * Get leaderboard entry by rank
 */
export function getLeaderboardEntry(rank: number, baseDir = '.crewchief'): LeaderboardEntry | null {
  const leaderboard = loadLeaderboard(baseDir)

  if (rank < 1 || rank > leaderboard.allTimeTopVariants.length) {
    return null
  }

  return leaderboard.allTimeTopVariants[rank - 1]
}

/**
 * Get leaderboard entry by variant ID
 */
export function getLeaderboardEntryByVariantId(variantId: string, baseDir = '.crewchief'): LeaderboardEntry | null {
  const leaderboard = loadLeaderboard(baseDir)

  return leaderboard.allTimeTopVariants.find((entry) => entry.variantId === variantId) || null
}

/**
 * Generate human-readable leaderboard report
 */
export function generateLeaderboardReport(baseDir = '.crewchief'): string {
  const leaderboard = loadLeaderboard(baseDir)
  const lines: string[] = []

  lines.push('GENETIC OPTIMIZATION LEADERBOARD')
  lines.push('='.repeat(80))
  lines.push('')
  lines.push(`Last Updated: ${leaderboard.lastUpdated.toLocaleString()}`)
  lines.push(`Total Entries: ${leaderboard.allTimeTopVariants.length}`)

  if (leaderboard.productionVariant) {
    lines.push(`Production Variant: ${leaderboard.productionVariant}`)
    if (leaderboard.productionDeployedAt) {
      lines.push(`  Deployed: ${leaderboard.productionDeployedAt.toLocaleString()}`)
    }
  }

  lines.push('')
  lines.push('TOP VARIANTS')
  lines.push('-'.repeat(80))

  if (leaderboard.allTimeTopVariants.length === 0) {
    lines.push('No variants yet - run genetic optimization to populate leaderboard')
  } else {
    leaderboard.allTimeTopVariants.forEach((entry) => {
      const isProd = entry.variantId === leaderboard.productionVariant ? ' [PRODUCTION]' : ''
      lines.push(`${entry.rank}. ${entry.name}${isProd}`)
      lines.push(`   Composite Score: ${(entry.compositeScore * 100).toFixed(1)}%`)
      lines.push(
        `   Tier Scores: T1=${(entry.tierScores.tier1 * 100).toFixed(0)}% T2=${(entry.tierScores.tier2 * 100).toFixed(0)}% T3=${(entry.tierScores.tier3 * 100).toFixed(0)}%`,
      )
      lines.push(`   Tool Selection: ${(entry.toolSelectionAccuracy * 100).toFixed(0)}% accurate`)
      lines.push(`   Task Coverage: ${entry.taskCoverage.passed}/${entry.taskCoverage.total} passed`)
      lines.push(`   Generation: ${entry.generation} | Converged: ${entry.converged ? 'Yes' : 'No'}`)
      lines.push(`   Run ID: ${entry.runId}`)
      lines.push(`   Timestamp: ${entry.timestamp.toLocaleString()}`)
      lines.push('')
    })
  }

  return lines.join('\n')
}
