/**
 * Startup reconciliation for Maproom extension
 *
 * Catches up on file changes that occurred while the extension was inactive
 * by using git diff to find changed files and the upsert CLI command to index them.
 */

import { execFile, spawn } from 'node:child_process'
import { promisify } from 'node:util'
import * as vscode from 'vscode'
import path from 'node:path'
import { getRepoName, getBranchName } from '../utils/git'
import { detectPlatform, getBinaryExtension } from '../utils/platform'

const execFileAsync = promisify(execFile)

/** Key for storing last indexed commit in workspace state */
const LAST_COMMIT_KEY = 'maproom.lastIndexedCommit'

/**
 * Configuration for reconciliation
 */
export interface ReconcileConfig {
  /** Path to extension root (where bin/ directory is located) */
  extensionRoot: string
  /** Database URL for the maproom binary */
  databaseUrl: string
  /** Optional progress callback */
  onProgress?: (message: string) => void
}

/**
 * Result of reconciliation operation
 */
export interface ReconcileResult {
  /** Whether reconciliation was performed */
  performed: boolean
  /** Number of files reconciled */
  filesReconciled: number
  /** Previous commit hash (if any) */
  previousCommit?: string
  /** Current commit hash */
  currentCommit?: string
  /** Error if reconciliation failed (but was gracefully handled) */
  error?: string
}

/**
 * Reconcile file changes since last extension run
 *
 * This function:
 * 1. Reads the last indexed commit from workspace state
 * 2. Uses git diff to find files changed since that commit
 * 3. Runs the upsert CLI command to index changed files
 * 4. Updates the stored commit to current HEAD
 *
 * On first run (no stored commit), it skips reconciliation and lets
 * the watch process handle initial indexing.
 *
 * @param context - VSCode extension context for state storage
 * @param config - Reconciliation configuration
 * @returns Result object describing what was done
 *
 * @example
 * ```typescript
 * const result = await reconcileChanges(context, {
 *   extensionRoot: context.extensionPath,
 *   databaseUrl: 'sqlite:///path/to/db',
 *   onProgress: (msg) => statusBar.setState('reconciling', msg),
 * })
 * ```
 */
export async function reconcileChanges(
  context: vscode.ExtensionContext,
  config: ReconcileConfig
): Promise<ReconcileResult> {
  const workspaceFolders = vscode.workspace.workspaceFolders
  if (!workspaceFolders || workspaceFolders.length === 0) {
    return { performed: false, filesReconciled: 0, error: 'No workspace folder' }
  }

  const workspaceRoot = workspaceFolders[0].uri.fsPath

  try {
    // Get repository and branch information
    const repoName = await getRepoName(workspaceRoot)
    const branchName = await getBranchName(workspaceRoot)

    // 1. Get last indexed commit from workspace state
    const lastCommit = context.workspaceState.get<string>(LAST_COMMIT_KEY)

    // 2. Get current HEAD commit
    const currentCommit = await getCurrentCommit(workspaceRoot)
    if (!currentCommit) {
      return { performed: false, filesReconciled: 0, error: 'Failed to get current commit' }
    }

    // First run - skip reconciliation, watch will handle initial indexing
    if (!lastCommit) {
      config.onProgress?.('First run - skipping reconciliation')
      // Store current commit so next startup can reconcile
      await context.workspaceState.update(LAST_COMMIT_KEY, currentCommit)
      return {
        performed: false,
        filesReconciled: 0,
        currentCommit,
      }
    }

    // No changes since last run
    if (lastCommit === currentCommit) {
      config.onProgress?.('No changes since last run')
      return {
        performed: false,
        filesReconciled: 0,
        previousCommit: lastCommit,
        currentCommit,
      }
    }

    // 3. Get changed files via git diff
    config.onProgress?.('Finding changed files...')
    const changedFiles = await getChangedFiles(workspaceRoot, lastCommit, currentCommit)

    if (changedFiles.length === 0) {
      // Update commit even if no files changed (e.g., merge commits, deletions only)
      await context.workspaceState.update(LAST_COMMIT_KEY, currentCommit)
      return {
        performed: false,
        filesReconciled: 0,
        previousCommit: lastCommit,
        currentCommit,
      }
    }

    // 4. Run upsert for changed files
    config.onProgress?.(`Reconciling ${changedFiles.length} files...`)
    await runUpsert(changedFiles, {
      extensionRoot: config.extensionRoot,
      databaseUrl: config.databaseUrl,
      repo: repoName,
      worktree: branchName,
      root: workspaceRoot,
      commit: currentCommit,
    })

    // 5. Update last indexed commit
    await context.workspaceState.update(LAST_COMMIT_KEY, currentCommit)

    config.onProgress?.(`Reconciled ${changedFiles.length} files`)

    return {
      performed: true,
      filesReconciled: changedFiles.length,
      previousCommit: lastCommit,
      currentCommit,
    }
  } catch (error) {
    // Gracefully handle errors - don't block extension startup
    const errorMessage = error instanceof Error ? error.message : 'Unknown error'
    config.onProgress?.(`Reconciliation skipped: ${errorMessage}`)
    return {
      performed: false,
      filesReconciled: 0,
      error: errorMessage,
    }
  }
}

/**
 * Update the last indexed commit in workspace state
 *
 * Called after successful watch completion to mark current state as indexed.
 *
 * @param context - VSCode extension context
 * @param workspaceRoot - Workspace root directory
 */
export async function updateLastIndexedCommit(
  context: vscode.ExtensionContext,
  workspaceRoot: string
): Promise<void> {
  const currentCommit = await getCurrentCommit(workspaceRoot)
  if (currentCommit) {
    await context.workspaceState.update(LAST_COMMIT_KEY, currentCommit)
  }
}

/**
 * Get the current HEAD commit hash
 *
 * @param workspaceRoot - Workspace root directory
 * @returns Commit hash or undefined if not a git repo
 */
async function getCurrentCommit(workspaceRoot: string): Promise<string | undefined> {
  try {
    const { stdout } = await execFileAsync('git', ['rev-parse', 'HEAD'], {
      cwd: workspaceRoot,
    })
    return stdout.trim()
  } catch {
    return undefined
  }
}

/**
 * Get list of files changed between two commits
 *
 * @param workspaceRoot - Workspace root directory
 * @param fromCommit - Starting commit hash
 * @param toCommit - Ending commit hash
 * @returns Array of changed file paths
 */
async function getChangedFiles(
  workspaceRoot: string,
  fromCommit: string,
  toCommit: string
): Promise<string[]> {
  try {
    const { stdout } = await execFileAsync(
      'git',
      ['diff', '--name-only', `${fromCommit}..${toCommit}`],
      { cwd: workspaceRoot }
    )
    return stdout.split('\n').filter(Boolean)
  } catch {
    return []
  }
}

/**
 * Configuration for upsert operation
 */
interface UpsertConfig {
  extensionRoot: string
  databaseUrl: string
  repo: string
  worktree: string
  root: string
  commit: string
}

/**
 * Maximum number of files to process in a single upsert batch.
 * This prevents E2BIG errors from exceeding command line argument limits.
 */
const UPSERT_BATCH_SIZE = 100

/**
 * Run the upsert CLI command to index changed files
 *
 * Batches files to avoid E2BIG errors when there are many changed files.
 *
 * @param files - Array of file paths to index
 * @param config - Upsert configuration
 */
async function runUpsert(files: string[], config: UpsertConfig): Promise<void> {
  // Construct binary path
  const platform = detectPlatform()
  const binaryName = `maproom${getBinaryExtension()}`
  const binaryPath = path.join(config.extensionRoot, 'bin', platform, binaryName)

  // Process files in batches to avoid E2BIG errors
  for (let i = 0; i < files.length; i += UPSERT_BATCH_SIZE) {
    const batch = files.slice(i, i + UPSERT_BATCH_SIZE)
    await runUpsertBatch(binaryPath, batch, config)
  }
}

/**
 * Run a single batch of files through upsert
 */
async function runUpsertBatch(
  binaryPath: string,
  files: string[],
  config: UpsertConfig
): Promise<void> {
  return new Promise((resolve, reject) => {
    const proc = spawn(
      binaryPath,
      [
        'upsert',
        '--commit',
        config.commit,
        '--repo',
        config.repo,
        '--worktree',
        config.worktree,
        '--root',
        config.root,
        '--paths',
        files.join(','),
      ],
      {
        env: {
          ...process.env,
          MAPROOM_DATABASE_URL: config.databaseUrl,
        },
        stdio: ['ignore', 'pipe', 'pipe'],
      }
    )

    let stderr = ''

    proc.stderr?.on('data', (chunk: Buffer) => {
      stderr += chunk.toString('utf8')
    })

    proc.on('close', (code) => {
      if (code === 0) {
        resolve()
      } else {
        reject(new Error(`upsert exited with code ${code}: ${stderr.trim()}`))
      }
    })

    proc.on('error', (error) => {
      reject(new Error(`Failed to spawn upsert: ${error.message}`))
    })
  })
}
