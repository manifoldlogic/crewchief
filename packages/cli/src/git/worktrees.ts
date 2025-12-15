import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import simpleGit, { SimpleGit } from 'simple-git'
import { copyIgnoredFiles } from './copy-ignored-files'
import { loadConfig } from '../config/loader'
import { CrewChiefConfig } from '../config/schema'
import { ensureDirSync, removeDirSync } from '../utils/fs'
import { logger } from '../utils/logger'
import { findMaproomBinary } from '../utils/maproom-binary.js'
import { expandWorktreePath } from '../utils/paths'
import { WorktreeMetadataService } from '../utils/worktree-metadata'

export interface WorktreeListItem {
  path: string
  branch?: string
}

export class WorktreeService {
  private git: SimpleGit
  private cwd: string

  constructor(cwd: string = process.cwd()) {
    this.cwd = cwd
    this.git = simpleGit({ baseDir: cwd })
  }

  /**
   * Run maproom indexing scan for a worktree.
   *
   * This method is called conditionally based on the `autoScanOnWorktreeUse` config setting.
   * By default, automatic scanning is disabled to keep worktree creation fast.
   *
   * @param worktreePath - Absolute path to the worktree directory
   * @private
   */
  private async runMaproomScan(worktreePath: string): Promise<void> {
    try {
      const config = await loadConfig()
      const result = findMaproomBinary({
        configPath: config.repository.maproomBinaryPath,
      })

      if (!result.path) {
        console.log('⚠️  Maproom binary not found, skipping indexing for new worktree')
        return
      }

      console.log('🔍 Running maproom scan for new worktree...')

      // Run maproom scan with automatic detection (it will detect repo, worktree, and commit from the worktree path)
      const scanResult = spawnSync(result.path, ['scan'], {
        cwd: worktreePath,
        encoding: 'utf8',
        stdio: ['pipe', 'pipe', 'pipe'],
      })

      if (scanResult.status === 0) {
        // Parse output to show summary
        const output = scanResult.stdout
        if (output.includes('files processed') || output.includes('chunks created')) {
          console.log('✅ Maproom indexing completed successfully')
        } else {
          console.log('✅ Maproom scan completed')
        }
      } else {
        const errorMsg = scanResult.stderr || scanResult.stdout || 'Unknown error'
        console.warn(`⚠️  Maproom scan failed: ${errorMsg.split('\n')[0]}`)
      }
    } catch (error) {
      console.warn('⚠️  Failed to run maproom scan:', error instanceof Error ? error.message : error)
    }
  }

  async initRepository(storagePath: string): Promise<void> {
    ensureDirSync(path.join(this.cwd, storagePath))
  }

  /**
   * Create a new git worktree.
   *
   * Conditionally triggers maproom scanning based on the `autoScanOnWorktreeUse` config setting.
   * By default, automatic scanning is disabled (changed in v2.0) to improve performance.
   * Users can enable it by setting `worktree.autoScanOnWorktreeUse: true` in their config.
   *
   * @param name - Name of the worktree/branch
   * @param baseBranch - Base branch to create from
   * @param basePath - Base directory path for worktrees
   * @param skipCopyIgnored - Skip copying ignored files (e.g., .env)
   * @param purpose - Purpose of the worktree: 'agent' or 'manual'
   * @returns Absolute path to the created worktree
   */
  async createWorktree(
    name: string,
    baseBranch: string,
    basePath: string,
    skipCopyIgnored?: boolean,
    purpose: 'agent' | 'manual' = 'manual',
  ): Promise<string> {
    // Expand path before construction
    let expandedBasePath: string
    try {
      expandedBasePath = await expandWorktreePath(basePath, this.cwd)
    } catch (error) {
      throw new Error(`Invalid worktree path "${basePath}": ${error instanceof Error ? error.message : String(error)}`)
    }

    // Don't join with cwd - expansion already resolved absolute paths
    const wtPath = path.join(expandedBasePath, name)
    ensureDirSync(wtPath)

    // Get current branch before creating worktree
    const currentBranch = await this.getCurrentBranch()

    // Try to fetch, but don't fail if we can't (e.g., in devcontainer without credentials)
    try {
      await this.git.fetch()
    } catch {
      console.warn('⚠️  Could not fetch from remote (this is normal in devcontainers or offline mode)')
    }

    await this.git.raw(['worktree', 'add', '-B', name, wtPath, baseBranch])

    // Save worktree metadata
    const metadataService = new WorktreeMetadataService()
    await metadataService.save(wtPath, {
      sourceBranch: currentBranch,
      createdAt: new Date().toISOString(),
      createdFrom: this.cwd,
      baseBranch,
      purpose,
    })

    // Load config once for both operations (efficiency + consistency)
    let config: CrewChiefConfig | null = null
    try {
      config = await loadConfig()
    } catch (error) {
      console.warn('⚠️  Failed to load config:', error instanceof Error ? error.message : error)
    }

    // Copy ignored files if configured and not skipped
    if (!skipCopyIgnored && config?.worktree?.copyIgnoredFiles?.length) {
      try {
        console.log('\n📁 Copying ignored files to worktree...')
        await copyIgnoredFiles({
          sourceRoot: this.cwd,
          worktreeRoot: wtPath,
          config,
        })
      } catch (error) {
        console.warn('⚠️  Failed to copy ignored files:', error instanceof Error ? error.message : error)
      }
    }

    // Run maproom scan if configured (opt-in)
    if (config?.worktree?.autoScanOnWorktreeUse) {
      await this.runMaproomScan(wtPath)
    }

    return wtPath
  }

  async listWorktrees(): Promise<WorktreeListItem[]> {
    const out = await this.git.raw(['worktree', 'list', '--porcelain'])
    const lines = out.split('\n')
    const items: WorktreeListItem[] = []
    let current: Partial<WorktreeListItem> = {}
    for (const line of lines) {
      if (line.startsWith('worktree ')) {
        if (current.path) items.push(current as WorktreeListItem)
        current = { path: line.replace('worktree ', '').trim() }
      } else if (line.startsWith('branch ')) {
        current.branch = line.replace('branch refs/heads/', '').trim()
      }
    }
    if (current.path) items.push(current as WorktreeListItem)
    return items
  }

  async getCurrentBranch(): Promise<string> {
    try {
      const branch = await this.git.revparse(['--abbrev-ref', 'HEAD'])
      return branch.trim()
    } catch {
      return 'main'
    }
  }

  async pruneWorktrees(opts?: { mode?: 'stale' | 'all'; keepDir?: boolean }): Promise<void> {
    if (!opts || opts.mode === 'stale') {
      await this.git.raw(['worktree', 'prune'])
      return
    }
    if (opts.mode === 'all') {
      const list = await this.listWorktrees()
      // Use real paths so symlinks do not bypass the protection
      const cwdReal = safeRealpath(this.cwd)
      for (const item of list) {
        const p = path.resolve(item.path)
        const pReal = safeRealpath(p)
        // Skip if current working directory is the same as, or inside, this worktree
        const rel = path.relative(pReal, cwdReal)
        const isCwdInside = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel))
        if (isCwdInside) continue // never remove current working tree
        try {
          // Force remove to handle unmerged/untracked files in the worktree
          await this.git.raw(['worktree', 'remove', '--force', p])
          // Delete the directory unless --keep-dir was specified
          if (!opts.keepDir) {
            removeDirSync(p)
          }
        } catch {
          // ignore failures, continue best-effort
        }
      }
    }
  }

  async removeWorktree(worktreePath: string): Promise<void> {
    // Guard against deleting the current worktree (or its ancestor) even if asked
    const targetPath = path.resolve(worktreePath)
    const targetReal = safeRealpath(targetPath)
    const cwdReal = safeRealpath(this.cwd)
    const rel = path.relative(targetReal, cwdReal)
    const isCwdInsideTarget = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel))
    if (isCwdInsideTarget) {
      throw new Error('Refusing to remove the current worktree. Change directories and try again.')
    }
    await this.git.raw(['worktree', 'remove', '--force', targetPath])
  }
}

/**
 * Clean up stale worktree records from the maproom database.
 * Uses batch cleanup to remove all stale worktree records at once.
 *
 * @param config - Optional config object. If not provided, will attempt to load from disk.
 * @throws Error if maproom binary not found or cleanup fails (exit code 1)
 */
export async function cleanMaproomRecords(config?: CrewChiefConfig): Promise<void> {
  // Resolve config - use provided or load from disk
  let resolvedConfig = config
  if (!resolvedConfig) {
    try {
      resolvedConfig = await loadConfig()
    } catch {
      // Config not found or invalid - continue without it
      // Binary resolution will fall back to env var/global/packaged
    }
  }

  // Find maproom binary
  const result = findMaproomBinary({
    configPath: resolvedConfig?.repository.maproomBinaryPath,
  })

  if (!result.path) {
    throw new Error('Maproom binary not found')
  }

  // Run cleanup command
  const cleanupResult = spawnSync(result.path, ['db', 'cleanup-stale', '--confirm'], {
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'pipe'],
  })

  if (cleanupResult.status !== 0 && cleanupResult.status !== 2) {
    // Exit code 2 means "no stale worktrees", which is fine
    const errorMsg = cleanupResult.stderr || cleanupResult.stdout || 'Unknown error'
    throw new Error(errorMsg.split('\n')[0])
  }

  // Parse output for user feedback
  const output = cleanupResult.stdout
  if (output.includes('Deleted')) {
    logger.info('Cleaned maproom database records')
  }
}

function safeRealpath(p: string): string {
  try {
    return fs.realpathSync(p)
  } catch {
    return path.resolve(p)
  }
}

// Deterministic branch naming per spec: derive from agent id, optional task, and timestamp
export function buildDeterministicBranchName(params: {
  agentTypeId: string
  taskDescription?: string
  now?: Date
}): string {
  const safeId = slugify(params.agentTypeId)
  const taskPart = params.taskDescription ? '-' + slugify(params.taskDescription).slice(0, 32) : ''
  const d = params.now ?? new Date()
  const ts =
    d.getUTCFullYear().toString() +
    pad2(d.getUTCMonth() + 1) +
    pad2(d.getUTCDate()) +
    pad2(d.getUTCHours()) +
    pad2(d.getUTCMinutes()) +
    pad2(d.getUTCSeconds())
  return `cc-${safeId}${taskPart}-${ts}`
}

function pad2(n: number): string {
  return n < 10 ? `0${n}` : `${n}`
}

function slugify(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .replace(/-{2,}/g, '-')
}
