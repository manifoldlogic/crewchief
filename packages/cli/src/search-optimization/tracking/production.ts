/**
 * Production Variant Management System
 *
 * Manages production variant designation, deployment tracking, and rollback capabilities.
 * Maintains deployment log in markdown format for auditability.
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync, renameSync, appendFileSync } from 'fs'
import { join } from 'path'
import { loadLeaderboard, saveLeaderboard } from './leaderboard.js'
import type { Variant } from '../../../../maproom-mcp/test/tool-description-optimization/types.js'

/**
 * Production variant pointer schema
 */
export interface ProductionPointer {
  schemaVersion: number
  currentVariantId: string
  deployedAt: Date
  deployedBy?: string // Optional: user who deployed
  reason?: string // Optional: deployment reason
  previousVariantId?: string // For rollback tracking
}

/**
 * Deployment log entry
 */
export interface DeploymentLogEntry {
  timestamp: Date
  action: 'promote' | 'rollback'
  variantId: string
  previousVariantId?: string
  reason?: string
  deployedBy?: string
}

/**
 * Get production directory path
 */
export function getProductionDir(baseDir = '.crewchief'): string {
  return join(baseDir, 'production')
}

/**
 * Get production pointer path
 */
export function getProductionPointerPath(baseDir = '.crewchief'): string {
  return join(getProductionDir(baseDir), 'current.json')
}

/**
 * Get production variants directory path
 */
export function getProductionVariantsDir(baseDir = '.crewchief'): string {
  return join(getProductionDir(baseDir), 'variants')
}

/**
 * Get deployment log path
 */
export function getDeploymentLogPath(baseDir = '.crewchief'): string {
  return join(getProductionDir(baseDir), 'deployment-log.md')
}

/**
 * Load current production pointer
 */
export function getCurrentProduction(baseDir = '.crewchief'): ProductionPointer | null {
  const path = getProductionPointerPath(baseDir)

  if (!existsSync(path)) {
    return null
  }

  const content = readFileSync(path, 'utf-8')
  const data = JSON.parse(content)

  return {
    ...data,
    deployedAt: new Date(data.deployedAt),
  }
}

/**
 * Save production pointer using atomic write
 */
function saveProductionPointer(pointer: ProductionPointer, baseDir = '.crewchief'): void {
  const path = getProductionPointerPath(baseDir)
  const dir = getProductionDir(baseDir)

  mkdirSync(dir, { recursive: true })

  const tmpPath = `${path}.tmp`
  writeFileSync(tmpPath, JSON.stringify(pointer, null, 2))
  renameSync(tmpPath, path)
}

/**
 * Append entry to deployment log
 */
function appendToDeploymentLog(entry: DeploymentLogEntry, baseDir = '.crewchief'): void {
  const logPath = getDeploymentLogPath(baseDir)
  const dir = getProductionDir(baseDir)

  mkdirSync(dir, { recursive: true })

  // Initialize log if it doesn't exist
  if (!existsSync(logPath)) {
    const header = [
      '# Production Deployment Log',
      '',
      'Track all production variant deployments and rollbacks.',
      '',
      '## Deployments',
      '',
    ].join('\n')
    writeFileSync(logPath, header)
  }

  // Format log entry
  const lines: string[] = []
  lines.push(`### ${entry.action === 'promote' ? 'Deployment' : 'Rollback'}: ${entry.timestamp.toISOString()}`)
  lines.push('')
  lines.push(`- **Action**: ${entry.action}`)
  lines.push(`- **Variant**: ${entry.variantId}`)
  if (entry.previousVariantId) {
    lines.push(`- **Previous**: ${entry.previousVariantId}`)
  }
  if (entry.reason) {
    lines.push(`- **Reason**: ${entry.reason}`)
  }
  if (entry.deployedBy) {
    lines.push(`- **Deployed By**: ${entry.deployedBy}`)
  }
  lines.push('')

  appendFileSync(logPath, lines.join('\n'))
}

/**
 * Promote variant to production
 *
 * Copies variant JSON to production/variants/ and updates production pointer
 */
export function promoteToProduction(
  variant: Variant,
  reason?: string,
  deployedBy?: string,
  baseDir = '.crewchief',
): ProductionPointer {
  const variantsDir = getProductionVariantsDir(baseDir)
  mkdirSync(variantsDir, { recursive: true })

  // Get current production (for rollback tracking)
  const current = getCurrentProduction(baseDir)
  const previousVariantId = current?.currentVariantId

  // Copy variant to production variants directory
  const variantPath = join(variantsDir, `${variant.id}.json`)
  writeFileSync(variantPath, JSON.stringify(variant, null, 2))

  // Create production pointer
  const pointer: ProductionPointer = {
    schemaVersion: 1,
    currentVariantId: variant.id,
    deployedAt: new Date(),
    deployedBy,
    reason,
    previousVariantId,
  }

  // Save pointer
  saveProductionPointer(pointer, baseDir)

  // Update leaderboard
  const leaderboard = loadLeaderboard(baseDir)
  leaderboard.productionVariant = variant.id
  leaderboard.productionDeployedAt = pointer.deployedAt
  saveLeaderboard(leaderboard, baseDir)

  // Append to deployment log
  appendToDeploymentLog(
    {
      timestamp: pointer.deployedAt,
      action: 'promote',
      variantId: variant.id,
      previousVariantId,
      reason,
      deployedBy,
    },
    baseDir,
  )

  console.log(`✓ Promoted ${variant.name} to production`)
  console.log(`  Variant ID: ${variant.id}`)
  console.log(`  Deployed at: ${pointer.deployedAt.toISOString()}`)
  if (previousVariantId) {
    console.log(`  Replaced: ${previousVariantId}`)
  }

  return pointer
}

/**
 * Rollback to previous production variant
 *
 * Reverts to the previous variant ID stored in current production pointer
 */
export function rollbackProduction(reason?: string, deployedBy?: string, baseDir = '.crewchief'): ProductionPointer {
  const current = getCurrentProduction(baseDir)

  if (!current) {
    throw new Error('No production variant to rollback from')
  }

  if (!current.previousVariantId) {
    throw new Error('No previous variant ID available for rollback')
  }

  // Load previous variant
  const variantsDir = getProductionVariantsDir(baseDir)
  const previousVariantPath = join(variantsDir, `${current.previousVariantId}.json`)

  if (!existsSync(previousVariantPath)) {
    throw new Error(`Previous variant ${current.previousVariantId} not found in production variants`)
  }

  const variantContent = readFileSync(previousVariantPath, 'utf-8')
  const previousVariant: Variant = JSON.parse(variantContent)

  // Create new pointer
  const pointer: ProductionPointer = {
    schemaVersion: 1,
    currentVariantId: current.previousVariantId,
    deployedAt: new Date(),
    deployedBy,
    reason: reason || 'Rollback',
    previousVariantId: current.currentVariantId, // Allow rolling back from rollback
  }

  // Save pointer
  saveProductionPointer(pointer, baseDir)

  // Update leaderboard
  const leaderboard = loadLeaderboard(baseDir)
  leaderboard.productionVariant = pointer.currentVariantId
  leaderboard.productionDeployedAt = pointer.deployedAt
  saveLeaderboard(leaderboard, baseDir)

  // Append to deployment log
  appendToDeploymentLog(
    {
      timestamp: pointer.deployedAt,
      action: 'rollback',
      variantId: pointer.currentVariantId,
      previousVariantId: current.currentVariantId,
      reason,
      deployedBy,
    },
    baseDir,
  )

  console.log(`✓ Rolled back to ${previousVariant.name}`)
  console.log(`  Variant ID: ${pointer.currentVariantId}`)
  console.log(`  Deployed at: ${pointer.deployedAt.toISOString()}`)
  console.log(`  Rolled back from: ${current.currentVariantId}`)

  return pointer
}

/**
 * Load production variant
 */
export function loadProductionVariant(baseDir = '.crewchief'): Variant | null {
  const current = getCurrentProduction(baseDir)

  if (!current) {
    return null
  }

  const variantsDir = getProductionVariantsDir(baseDir)
  const variantPath = join(variantsDir, `${current.currentVariantId}.json`)

  if (!existsSync(variantPath)) {
    console.error(`Production variant ${current.currentVariantId} not found`)
    return null
  }

  const content = readFileSync(variantPath, 'utf-8')
  const data = JSON.parse(content)

  return {
    ...data,
    created_at: new Date(data.created_at),
  }
}

/**
 * Get production deployment history
 */
export function getProductionHistory(baseDir = '.crewchief'): string | null {
  const logPath = getDeploymentLogPath(baseDir)

  if (!existsSync(logPath)) {
    return null
  }

  return readFileSync(logPath, 'utf-8')
}

/**
 * Generate production status report
 */
export function generateProductionReport(baseDir = '.crewchief'): string {
  const current = getCurrentProduction(baseDir)
  const variant = loadProductionVariant(baseDir)

  const lines: string[] = []

  lines.push('PRODUCTION VARIANT STATUS')
  lines.push('='.repeat(80))
  lines.push('')

  if (!current || !variant) {
    lines.push('No production variant currently deployed')
    lines.push('')
    lines.push('Use `promoteToProduction(variant)` to deploy a variant to production')
  } else {
    lines.push(`Current Variant: ${variant.name}`)
    lines.push(`Variant ID: ${variant.id}`)
    lines.push(`Generation: ${variant.generation}`)
    lines.push(`Deployed At: ${current.deployedAt.toLocaleString()}`)
    if (current.deployedBy) {
      lines.push(`Deployed By: ${current.deployedBy}`)
    }
    if (current.reason) {
      lines.push(`Reason: ${current.reason}`)
    }
    if (current.previousVariantId) {
      lines.push(`Previous Variant: ${current.previousVariantId} (available for rollback)`)
    }
    lines.push('')
    lines.push('Description Preview:')
    lines.push('-'.repeat(80))
    lines.push(variant.description.substring(0, 300) + '...')
  }

  lines.push('')

  return lines.join('\n')
}
