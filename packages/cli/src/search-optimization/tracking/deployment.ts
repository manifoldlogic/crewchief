/**
 * Automated Variant Deployment System
 *
 * Deploys winning variants to the live MCP server by updating the tool description
 * in source code, rebuilding, and optionally restarting the server.
 */

import { spawn } from 'child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync, readdirSync, unlinkSync } from 'fs'
import { join } from 'path'
import { getLeaderboardEntryByVariantId } from './leaderboard.js'
import { promoteToProduction, getCurrentProduction } from './production.js'
import type { Variant } from '../../../../maproom-mcp/test/tool-description-optimization/types.js'

/**
 * Deployment result
 */
export interface DeploymentResult {
  success: boolean
  variantId: string
  previousDescription: string
  newDescription: string
  backupPath: string
  buildSuccess: boolean
  serverRestarted: boolean
  errors?: string[]
}

/**
 * Deployment options
 */
export interface DeploymentOptions {
  dryRun?: boolean
  skipBuild?: boolean
  autoRestart?: boolean
}

/**
 * Get backups directory path
 */
export function getBackupsDir(baseDir = '.crewchief'): string {
  return join(baseDir, 'production', 'backups')
}

/**
 * Backup current tool description
 *
 * Saves the current description to .crewchief/production/backups/
 * with metadata header
 */
export async function backupCurrentDescription(baseDir = '.crewchief', variantId?: string): Promise<string> {
  const backupsDir = getBackupsDir(baseDir)
  mkdirSync(backupsDir, { recursive: true })

  // Read current description from source
  const currentDescription = await readCurrentDescription()

  // Create backup file with timestamp
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
  const backupPath = join(backupsDir, `description-${timestamp}.txt`)

  // Create metadata header
  const metadata = [
    '# Tool Description Backup',
    `Timestamp: ${new Date().toISOString()}`,
    `Variant ID: ${variantId || 'unknown'}`,
    `Backup Path: ${backupPath}`,
    '',
    '---',
    '',
    currentDescription,
  ].join('\n')

  writeFileSync(backupPath, metadata, 'utf-8')

  return backupPath
}

/**
 * Prune old backups, keeping only the most recent N
 */
export function pruneOldBackups(baseDir = '.crewchief', keepCount = 10): void {
  const backupsDir = getBackupsDir(baseDir)

  if (!existsSync(backupsDir)) {
    return
  }

  const files = readdirSync(backupsDir)
    .filter((f) => f.startsWith('description-') && f.endsWith('.txt'))
    .map((f) => ({
      name: f,
      path: join(backupsDir, f),
    }))
    .sort((a, b) => b.name.localeCompare(a.name)) // Sort by timestamp descending

  // Remove old backups
  if (files.length > keepCount) {
    const toRemove = files.slice(keepCount)
    for (const file of toRemove) {
      try {
        unlinkSync(file.path)
      } catch (err) {
        console.warn(`Failed to remove old backup ${file.name}:`, err)
      }
    }
  }
}

/**
 * Read current tool description from MCP server source
 */
export async function readCurrentDescription(): Promise<string> {
  const indexPath = getIndexPath()

  if (!existsSync(indexPath)) {
    throw new Error(`MCP server index.ts not found at ${indexPath}. Ensure you're running from the repository root.`)
  }

  const content = readFileSync(indexPath, 'utf-8')

  // Find the search tool description using regex
  const toolSchemaRegex = /{\s*name:\s*'search',\s*description:\s*'([^']*(?:\\'[^']*)*)'/s
  const match = content.match(toolSchemaRegex)

  if (!match) {
    throw new Error('Could not find search tool description in index.ts. File structure may have changed.')
  }

  // Unescape the description
  const rawDescription = match[1]
  const unescaped = rawDescription.replace(/\\n/g, '\n').replace(/\\'/g, "'").replace(/\\\\/g, '\\')

  return unescaped
}

/**
 * Patch tool description in MCP server source
 */
export async function patchToolDescription(
  newDescription: string,
  dryRun = false,
): Promise<{ success: boolean; previousDescription: string; newDescription: string }> {
  const indexPath = getIndexPath()

  if (!existsSync(indexPath)) {
    throw new Error(`MCP server index.ts not found at ${indexPath}`)
  }

  const content = readFileSync(indexPath, 'utf-8')

  // Find and extract current description
  const toolSchemaRegex = /{\s*name:\s*'search',\s*description:\s*'([^']*(?:\\'[^']*)*)'/s
  const match = content.match(toolSchemaRegex)

  if (!match) {
    throw new Error('Could not find search tool description in index.ts')
  }

  const previousDescription = match[1].replace(/\\n/g, '\n').replace(/\\'/g, "'").replace(/\\\\/g, '\\')

  // Escape new description for TypeScript
  const escaped = escapeForTypeScript(newDescription)

  // Replace description
  const newContent = content.replace(toolSchemaRegex, `{\n    name: 'search',\n    description: '${escaped}'`)

  // Validate replacement worked
  if (!newContent.includes(escaped)) {
    throw new Error('Failed to update tool description. Replacement validation failed.')
  }

  // Write to file if not dry-run
  if (!dryRun) {
    writeFileSync(indexPath, newContent, 'utf-8')
  }

  return {
    success: true,
    previousDescription,
    newDescription,
  }
}

/**
 * Escape string for TypeScript source code
 */
function escapeForTypeScript(str: string): string {
  return str.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/\n/g, '\\n')
}

/**
 * Get path to MCP server index.ts
 */
function getIndexPath(): string {
  // Assume we're running from packages/cli or repository root
  const cwd = process.cwd()

  // Try from packages/cli
  let indexPath = join(cwd, '..', 'maproom-mcp', 'src', 'index.ts')
  if (existsSync(indexPath)) {
    return indexPath
  }

  // Try from repository root
  indexPath = join(cwd, 'packages', 'maproom-mcp', 'src', 'index.ts')
  if (existsSync(indexPath)) {
    return indexPath
  }

  throw new Error("Could not find packages/maproom-mcp/src/index.ts. Ensure you're running from the repository root.")
}

/**
 * Build MCP server
 */
export async function buildMCPServer(): Promise<{ success: boolean; stdout: string; stderr: string }> {
  const mcpDir = getMCPDir()

  if (!existsSync(mcpDir)) {
    throw new Error(`MCP server directory not found at ${mcpDir}`)
  }

  return new Promise((resolve) => {
    const buildProcess = spawn('pnpm', ['build'], {
      cwd: mcpDir,
      stdio: 'pipe',
    })

    let stdout = ''
    let stderr = ''

    buildProcess.stdout?.on('data', (chunk: Buffer) => {
      stdout += chunk.toString()
    })

    buildProcess.stderr?.on('data', (chunk: Buffer) => {
      stderr += chunk.toString()
    })

    buildProcess.on('close', (code) => {
      resolve({
        success: code === 0,
        stdout,
        stderr,
      })
    })
  })
}

/**
 * Get MCP server directory path
 */
function getMCPDir(): string {
  const cwd = process.cwd()

  // Try from packages/cli
  let mcpDir = join(cwd, '..', 'maproom-mcp')
  if (existsSync(mcpDir)) {
    return mcpDir
  }

  // Try from repository root
  mcpDir = join(cwd, 'packages', 'maproom-mcp')
  if (existsSync(mcpDir)) {
    return mcpDir
  }

  throw new Error('Could not find packages/maproom-mcp directory')
}

/**
 * Detect if MCP server is running
 */
export async function detectRunningServer(): Promise<boolean> {
  // This is a simple heuristic - check for node process with maproom-mcp
  // In production, this could be more sophisticated
  try {
    const { execSync } = await import('child_process')
    const output = execSync('ps aux | grep -i "maproom-mcp" | grep -v grep', {
      encoding: 'utf-8',
      stdio: 'pipe',
    })
    return output.trim().length > 0
  } catch {
    // ps command failed or no processes found
    return false
  }
}

/**
 * Rollback source code changes from backup
 */
export async function rollbackFromBackup(backupPath: string): Promise<void> {
  if (!existsSync(backupPath)) {
    throw new Error(`Backup file not found: ${backupPath}`)
  }

  const backupContent = readFileSync(backupPath, 'utf-8')

  // Extract description from backup (skip metadata header)
  const lines = backupContent.split('\n')
  const separatorIndex = lines.findIndex((line) => line.trim() === '---')

  if (separatorIndex === -1) {
    throw new Error('Invalid backup file format')
  }

  const description = lines.slice(separatorIndex + 2).join('\n')

  // Patch with backup description
  await patchToolDescription(description, false)
}

/**
 * Load variant from tracking system
 */
async function loadVariantFromTracking(variantId: string, baseDir = '.crewchief'): Promise<Variant> {
  // Try production variants first
  const { loadProductionVariant } = await import('./production.js')
  const productionVariant = loadProductionVariant(baseDir)

  if (productionVariant && productionVariant.id === variantId) {
    return productionVariant
  }

  // Try loading from leaderboard
  const entry = await getLeaderboardEntryByVariantId(variantId, baseDir)

  if (!entry) {
    throw new Error(
      `Variant ${variantId} not found. Run 'crewchief optimization leaderboard' to see available variants.`,
    )
  }

  // Load full variant from run directory
  // This requires finding the run directory and loading the variant JSON
  // For now, we'll need to implement loading from variants directory
  const { loadVariant } = await import('../genetic-iterator.js')
  return await loadVariant(variantId)
}

/**
 * Deploy a variant to the live MCP server
 *
 * Main deployment function that orchestrates the entire process
 */
export async function deployVariant(
  variantId: string,
  options: DeploymentOptions = {},
  baseDir = '.crewchief',
): Promise<DeploymentResult> {
  const { dryRun = false, skipBuild = false, autoRestart = false } = options

  const errors: string[] = []

  try {
    // 1. Load variant
    const variant = await loadVariantFromTracking(variantId, baseDir)

    if (!variant.description || variant.description.trim().length === 0) {
      throw new Error('Variant description is empty')
    }

    // 2. Check if already promoted (optional auto-promotion)
    const current = getCurrentProduction(baseDir)
    if (!current || current.currentVariantId !== variantId) {
      console.log(`\nℹ Variant ${variantId} is not currently marked as production.`)
      console.log('  Automatically promoting to production tracking...')
      promoteToProduction(variant, 'Automated deployment', undefined, baseDir)
    }

    // 3. Create backup
    console.log('\nCreating backup of current tool description...')
    const backupPath = await backupCurrentDescription(baseDir, variantId)
    console.log(`✓ Backup saved to: ${backupPath}`)

    // 4. Patch source code
    console.log('\nPatching MCP server source code...')
    const patchResult = await patchToolDescription(variant.description, dryRun)
    console.log(`✓ Source code ${dryRun ? 'would be' : 'has been'} updated`)

    if (dryRun) {
      console.log('\n--- DRY RUN MODE ---')
      console.log('Previous description length:', patchResult.previousDescription.length, 'chars')
      console.log('New description length:', patchResult.newDescription.length, 'chars')
      console.log('\nPreview of new description (first 200 chars):')
      console.log(patchResult.newDescription.substring(0, 200) + '...')
      console.log('\n--- END DRY RUN ---')

      // Prune backups
      pruneOldBackups(baseDir)

      return {
        success: true,
        variantId,
        previousDescription: patchResult.previousDescription,
        newDescription: patchResult.newDescription,
        backupPath,
        buildSuccess: true, // N/A for dry run
        serverRestarted: false,
      }
    }

    // 5. Build MCP server
    let buildSuccess = true
    if (!skipBuild) {
      console.log('\nBuilding MCP server...')
      const buildResult = await buildMCPServer()

      if (!buildResult.success) {
        console.error('✗ Build failed!')
        console.error('\nBuild errors:')
        console.error(buildResult.stderr)

        // Rollback changes
        console.log('\nRolling back source code changes...')
        await rollbackFromBackup(backupPath)
        console.log('✓ Rollback complete')

        errors.push('Build failed')
        errors.push(buildResult.stderr)
        buildSuccess = false

        return {
          success: false,
          variantId,
          previousDescription: patchResult.previousDescription,
          newDescription: patchResult.newDescription,
          backupPath,
          buildSuccess: false,
          serverRestarted: false,
          errors,
        }
      }

      console.log('✓ Build succeeded')
    } else {
      console.log('\nSkipping build (--skip-build flag set)')
    }

    // 6. Check for running server
    console.log('\nChecking for running MCP server...')
    const serverRunning = await detectRunningServer()

    let serverRestarted = false
    if (serverRunning) {
      console.log('⚠ MCP server is currently running')

      if (autoRestart) {
        console.log('  Attempting automatic restart...')
        console.log('  (Manual restart may be required)')
        serverRestarted = false // Placeholder - actual restart not implemented
      } else {
        console.log('\nNext steps:')
        console.log('1. Restart the MCP server to pick up the new description')
        console.log('2. Verify the deployment in your AI assistant')
      }
    } else {
      console.log('✓ No running MCP server detected')
    }

    // 7. Prune old backups
    pruneOldBackups(baseDir)

    // 8. Success!
    console.log('\n✓ Deployment complete!')
    console.log(`  Variant: ${variant.name}`)
    console.log(`  ID: ${variantId}`)
    console.log(`  Backup: ${backupPath}`)

    return {
      success: true,
      variantId,
      previousDescription: patchResult.previousDescription,
      newDescription: patchResult.newDescription,
      backupPath,
      buildSuccess,
      serverRestarted,
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    errors.push(errorMessage)

    return {
      success: false,
      variantId,
      previousDescription: '',
      newDescription: '',
      backupPath: '',
      buildSuccess: false,
      serverRestarted: false,
      errors,
    }
  }
}
