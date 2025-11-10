import { spawn } from 'child_process'

export interface ScanConfig {
  worktreePath: string
  repo: string
  worktree: string
  commit: string
  baseDir: string
}

export interface ScanResult {
  success: boolean
  worktree: string
  chunkCount: number
  durationMs: number
  error?: string
}

/**
 * Scans a single worktree using crewchief-maproom scan command.
 * Uses spawn with args array for command injection protection.
 *
 * @param config - Configuration for the worktree to scan
 * @returns ScanResult with success status, chunk count, and duration
 */
export async function scanWorktree(config: ScanConfig): Promise<ScanResult> {
  const startTime = Date.now()

  console.log(`📊 Scanning worktree: ${config.worktree}`)
  console.log(`   Path: ${config.worktreePath}`)

  try {
    // CRITICAL: Use spawn with args array (command injection protection)
    const proc = spawn(
      'crewchief-maproom',
      [
        'scan',
        '--repo',
        config.repo,
        '--worktree',
        config.worktree,
        '--commit',
        config.commit,
        '--root',
        config.worktreePath,
      ],
      {
        stdio: 'pipe',
        shell: false, // NEVER use shell for security
      },
    )

    // Collect output
    let stdout = ''
    let stderr = ''
    proc.stdout.on('data', (data) => {
      stdout += data.toString()
    })
    proc.stderr.on('data', (data) => {
      stderr += data.toString()
    })

    // Wait for completion
    const exitCode = await new Promise<number>((resolve) => {
      proc.on('close', resolve)
    })

    if (exitCode !== 0) {
      throw new Error(`Scan failed with code ${exitCode}: ${stderr}`)
    }

    // Parse output for chunk count
    const match = stdout.match(/Total chunks: (\d+)/)
    const chunkCount = match ? parseInt(match[1]) : 0

    const durationMs = Date.now() - startTime

    console.log(`   ✅ Scan complete: ${chunkCount} chunks in ${durationMs}ms`)

    return {
      success: true,
      worktree: config.worktree,
      chunkCount,
      durationMs,
    }
  } catch (error) {
    const durationMs = Date.now() - startTime
    const errorMessage = error instanceof Error ? error.message : String(error)
    console.error(`   ❌ Scan failed: ${errorMessage}`)

    return {
      success: false,
      worktree: config.worktree,
      chunkCount: 0,
      durationMs,
      error: errorMessage,
    }
  }
}

/**
 * Scans multiple worktrees sequentially with progress logging.
 * Fails fast on any scan error.
 *
 * @param configs - Array of worktree configurations to scan
 * @returns Array of ScanResults for all worktrees
 * @throws Error if any scan fails
 */
export async function scanAllWorktrees(configs: ScanConfig[]): Promise<ScanResult[]> {
  const results: ScanResult[] = []

  console.log(`\n📊 Scanning ${configs.length} worktrees...`)
  console.log('='.repeat(60))

  // Scan sequentially (fast due to embedding reuse)
  for (const config of configs) {
    const result = await scanWorktree(config)
    results.push(result)

    // Fail fast on errors
    if (!result.success) {
      throw new Error(`Scan failed for ${config.worktree}: ${result.error}`)
    }
  }

  const totalDuration = results.reduce((sum, r) => sum + r.durationMs, 0)
  const totalChunks = results.reduce((sum, r) => sum + r.chunkCount, 0)

  console.log('='.repeat(60))
  console.log(`✅ All scans complete in ${(totalDuration / 1000).toFixed(1)}s`)
  console.log(`📊 Total chunks indexed: ${totalChunks}`)
  console.log()

  return results
}

/**
 * Waits for async scan completion by polling status.
 * Placeholder for future async scanning support.
 *
 * @param scanId - Identifier for the scan to wait for
 * @param timeout - Maximum time to wait in milliseconds (default: 60000)
 * @throws Error - Always throws as async scanning not implemented
 */
export async function waitForScanCompletion(scanId: string, timeout: number = 60000): Promise<void> {
  // Placeholder for async scanning support
  // Currently all scans are synchronous
  // This function enables future parallel scanning optimization

  const startTime = Date.now()

  while (Date.now() - startTime < timeout) {
    // Check scan status via database or status file
    const status = await checkScanStatus(scanId)

    if (status === 'complete') return
    if (status === 'failed') throw new Error('Scan failed')

    await new Promise((resolve) => setTimeout(resolve, 1000))
  }

  throw new Error(`Scan timeout after ${timeout}ms`)
}

/**
 * Helper for checking scan status in async mode.
 * Placeholder for future implementation.
 *
 * @param _scanId - Identifier for the scan to check
 * @throws Error - Always throws as async scanning not implemented
 */
async function checkScanStatus(_scanId: string): Promise<'pending' | 'complete' | 'failed'> {
  // Query database or check status file
  // Not implemented in MVP (all scans are synchronous)
  throw new Error('Async scanning not implemented')
}
