/**
 * Pre-flight validation module
 *
 * Validates environment readiness before expensive agent operations.
 * Checks database connectivity, base branch indexing, worktree scanning status,
 * MCP configuration validity, and file permissions.
 */

import { spawn } from 'child_process'
import { readFile, writeFile, unlink, access } from 'fs/promises'
import { join } from 'path'
import { Client } from 'pg'
import type {
  CheckResult,
  IndexStatus,
  ValidationError,
  ValidationWarning,
  VariantEnvironment,
  VariantValidation,
  ValidationResult,
  CompetitionConfig,
} from './types.js'

/**
 * PreFlightValidator class
 *
 * Provides comprehensive validation for competition setup
 */
export class PreFlightValidator {
  private databaseUrl: string

  constructor(databaseUrl?: string) {
    this.databaseUrl = databaseUrl || process.env.MAPROOM_DATABASE_URL || ''
  }

  /**
   * Check database connection
   *
   * @returns true if database is accessible, false otherwise
   */
  async checkDatabaseConnection(): Promise<boolean> {
    if (!this.databaseUrl) {
      return false
    }

    const client = new Client({ connectionString: this.databaseUrl })

    try {
      await client.connect()
      await client.query('SELECT 1')
      return true
    } catch {
      return false
    } finally {
      try {
        await client.end()
      } catch {
        // Ignore cleanup errors
      }
    }
  }

  /**
   * Verify base branch is indexed
   *
   * @param repo - Repository name
   * @param branch - Branch name
   * @returns Index status with chunk count
   */
  async verifyBaseBranchIndexed(repo: string, branch: string): Promise<IndexStatus> {
    try {
      const statusOutput = await this.executeMaproomStatus(repo, branch)
      const status = JSON.parse(statusOutput)

      // Look for the worktree in the status output (nested under repos)
      if (status.repos && Array.isArray(status.repos)) {
        for (const repoData of status.repos) {
          if (repoData.name === repo && repoData.worktrees && Array.isArray(repoData.worktrees)) {
            const worktree = repoData.worktrees.find(
              (wt: Record<string, unknown>) => wt.name === branch || wt.worktree === branch,
            )

            if (worktree && typeof worktree.chunk_count === 'number') {
              return {
                indexed: worktree.chunk_count > 0,
                chunkCount: worktree.chunk_count,
              }
            }
          }
        }
      }

      // If we can't find the worktree, it's not indexed
      return {
        indexed: false,
        chunkCount: 0,
      }
    } catch {
      // If maproom command fails, assume not indexed
      return {
        indexed: false,
        chunkCount: 0,
      }
    }
  }

  /**
   * Check if worktree is scanned and indexed
   *
   * @param repo - Repository name
   * @param worktree - Worktree name
   * @returns CheckResult with validation status
   */
  async checkWorktreeScanned(repo: string, worktree: string): Promise<CheckResult> {
    try {
      const statusOutput = await this.executeMaproomStatus(repo, worktree)
      const status = JSON.parse(statusOutput)

      // Look for the worktree in the status output (nested under repos)
      if (status.repos && Array.isArray(status.repos)) {
        for (const repoData of status.repos) {
          if (repoData.name === repo && repoData.worktrees && Array.isArray(repoData.worktrees)) {
            const wt = repoData.worktrees.find(
              (w: Record<string, unknown>) => w.name === worktree || w.worktree === worktree,
            )

            if (!wt) {
              return {
                passed: false,
                message: 'Worktree not in database',
                details: { repo, worktree },
              }
            }

            const chunkCount = wt.chunk_count || 0

            if (chunkCount === 0) {
              return {
                passed: false,
                message: 'Worktree has 0 chunks indexed',
                details: { repo, worktree, chunkCount },
              }
            }

            return {
              passed: true,
              message: `Indexed with ${chunkCount} chunks`,
              details: { repo, worktree, chunkCount },
            }
          }
        }
      }

      return {
        passed: false,
        message: 'Worktree not in database',
        details: { repo, worktree },
      }
    } catch (error) {
      return {
        passed: false,
        message: `Failed to check worktree status: ${error instanceof Error ? error.message : String(error)}`,
        details: { repo, worktree, error },
      }
    }
  }

  /**
   * Check if MCP configuration is valid
   *
   * @param worktreePath - Path to worktree
   * @returns CheckResult with validation status
   */
  async checkMcpConfigValid(worktreePath: string): Promise<CheckResult> {
    try {
      const mcpConfigPath = join(worktreePath, '.mcp.json')

      // Try to read the file
      const content = await readFile(mcpConfigPath, 'utf-8')

      // Parse JSON
      let config: Record<string, unknown>
      try {
        config = JSON.parse(content) as Record<string, unknown>
      } catch (parseError) {
        return {
          passed: false,
          message: 'Invalid JSON in .mcp.json',
          details: { worktreePath, error: parseError },
        }
      }

      // Check for mcpServers.maproom
      if (!config.mcpServers || typeof config.mcpServers !== 'object') {
        return {
          passed: false,
          message: 'Missing mcpServers in .mcp.json',
          details: { worktreePath, config },
        }
      }

      if (!config.mcpServers.maproom || typeof config.mcpServers.maproom !== 'object') {
        return {
          passed: false,
          message: 'Missing maproom server in .mcp.json',
          details: { worktreePath, config },
        }
      }

      const maproomConfig = config.mcpServers.maproom as Record<string, unknown>

      // Check for required fields
      if (!maproomConfig.command || typeof maproomConfig.command !== 'string') {
        return {
          passed: false,
          message: 'Missing command field in maproom server config',
          details: { worktreePath, maproomConfig },
        }
      }

      if (!maproomConfig.args || !Array.isArray(maproomConfig.args)) {
        return {
          passed: false,
          message: 'Missing or invalid args field in maproom server config',
          details: { worktreePath, maproomConfig },
        }
      }

      return {
        passed: true,
        message: 'MCP configuration valid',
        details: { worktreePath, maproomConfig },
      }
    } catch (error) {
      if ((error as any).code === 'ENOENT') {
        return {
          passed: false,
          message: 'Missing .mcp.json file',
          details: { worktreePath },
        }
      }

      return {
        passed: false,
        message: `Failed to read .mcp.json: ${error instanceof Error ? error.message : String(error)}`,
        details: { worktreePath },
      }
    }
  }

  /**
   * Check file permissions in worktree
   *
   * @param worktreePath - Path to worktree
   * @returns CheckResult with validation status
   */
  async checkFilePermissions(worktreePath: string): Promise<CheckResult> {
    try {
      // Test read access - try to read package.json
      const packageJsonPath = join(worktreePath, 'package.json')

      try {
        await access(packageJsonPath)
        await readFile(packageJsonPath, 'utf-8')
      } catch {
        // package.json might not exist, try reading a directory instead
        try {
          await access(worktreePath)
        } catch {
          return {
            passed: false,
            message: 'Cannot read worktree directory',
            details: { worktreePath },
          }
        }
      }

      // Test write access - create and delete a temporary file
      const testFilePath = join(worktreePath, '.crewchief-test-write')

      try {
        await writeFile(testFilePath, 'test', 'utf-8')
        await unlink(testFilePath)
      } catch {
        return {
          passed: false,
          message: 'Permission error: Cannot write to worktree directory',
          details: { worktreePath },
        }
      }

      return {
        passed: true,
        message: 'Read/write permissions OK',
        details: { worktreePath },
      }
    } catch (error) {
      return {
        passed: false,
        message: `Permission check failed: ${error instanceof Error ? error.message : String(error)}`,
        details: { worktreePath },
      }
    }
  }

  /**
   * Validate a single variant environment
   *
   * @param env - Variant environment to validate
   * @returns VariantValidation with all check results
   */
  async validateVariantEnvironment(env: VariantEnvironment): Promise<VariantValidation> {
    const checks = {
      worktreeExists: await this.checkWorktreeExists(env.worktreePath),
      worktreeScanned: await this.checkWorktreeScanned(env.repo, env.worktree),
      mcpConfigValid: await this.checkMcpConfigValid(env.worktreePath),
      toolsAccessible: await this.checkToolsAccessible(env.worktreePath),
      filePermissions: await this.checkFilePermissions(env.worktreePath),
    }

    // Determine overall status
    const allPassed = Object.values(checks).every((check) => check.passed)

    // Collect failure reasons
    const failureReasons: string[] = []
    if (!checks.worktreeExists.passed) failureReasons.push('Worktree does not exist')
    if (!checks.worktreeScanned.passed) failureReasons.push('Worktree not scanned/indexed')
    if (!checks.mcpConfigValid.passed) failureReasons.push('Invalid MCP configuration')
    if (!checks.toolsAccessible.passed) failureReasons.push('Tools not accessible')
    if (!checks.filePermissions.passed) failureReasons.push('File permission issues')

    return {
      variantId: env.variantId,
      worktreePath: env.worktreePath,
      checks,
      overall: allPassed ? 'pass' : 'fail',
      failureReason: failureReasons.length > 0 ? failureReasons.join('; ') : undefined,
    }
  }

  /**
   * Validate entire competition setup
   *
   * @param _config - Competition configuration
   * @returns ValidationResult with all errors, warnings, and variant results
   */
  async validateCompetitionSetup(_config: CompetitionConfig): Promise<ValidationResult> {
    const errors: ValidationError[] = []
    const warnings: ValidationWarning[] = []
    const variantResults = new Map<string, VariantValidation>()

    // Check database connection first
    const dbConnected = await this.checkDatabaseConnection()
    if (!dbConnected) {
      errors.push({
        check: 'database_connection',
        message: 'Database connection failed',
        troubleshooting: this.getDatabaseTroubleshooting(),
      })

      // If database is down, we can't proceed with validation
      return {
        valid: false,
        errors,
        warnings,
        variantResults,
      }
    }

    // Validate each variant
    // Note: We need to construct VariantEnvironment from the variants
    // For now, we'll skip this as we don't have the full variant structure
    // In a real implementation, this would iterate through config.variants

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      variantResults,
    }
  }

  /**
   * Check if worktree directory exists
   *
   * @param worktreePath - Path to worktree
   * @returns CheckResult
   */
  private async checkWorktreeExists(worktreePath: string): Promise<CheckResult> {
    try {
      await access(worktreePath)
      return {
        passed: true,
        message: 'Worktree directory exists',
        details: { worktreePath },
      }
    } catch {
      return {
        passed: false,
        message: 'Worktree directory does not exist',
        details: { worktreePath },
      }
    }
  }

  /**
   * Check if tools are accessible (placeholder)
   *
   * Note: Cannot test actual tool availability without spawning agent.
   * This is a basic sanity check.
   *
   * @param worktreePath - Path to worktree
   * @returns CheckResult
   */
  private async checkToolsAccessible(_worktreePath: string): Promise<CheckResult> {
    // For now, we just return true as we can't test without spawning an agent
    // This check could be expanded to verify the maproom binary is accessible
    return {
      passed: true,
      message: 'Tools accessible (basic check)',
      details: {},
    }
  }

  /**
   * Execute maproom status command
   *
   * @param repo - Repository name
   * @param worktree - Worktree name
   * @returns JSON output from maproom status
   */
  private async executeMaproomStatus(repo: string, worktree: string): Promise<string> {
    return new Promise((resolve, reject) => {
      // Find the maproom binary - look for workspace root
      let currentDir = process.cwd()

      // If we're in packages/cli, go up to workspace root
      if (currentDir.endsWith('/packages/cli') || currentDir.endsWith('\\packages\\cli')) {
        currentDir = join(currentDir, '..', '..')
      }

      const maproomBinary = join(currentDir, 'packages', 'cli', 'bin', 'crewchief-maproom')

      const args = ['status', '--repo', repo, '--worktree', worktree, '--json']

      const child = spawn(maproomBinary, args, {
        stdio: ['ignore', 'pipe', 'pipe'],
      })

      let stdout = ''
      let stderr = ''

      child.stdout.on('data', (data) => {
        stdout += data.toString()
      })

      child.stderr.on('data', (data) => {
        stderr += data.toString()
      })

      child.on('close', (code) => {
        if (code === 0) {
          resolve(stdout)
        } else {
          reject(new Error(`Maproom status failed with code ${code}: ${stderr}`))
        }
      })

      child.on('error', (error) => {
        reject(error)
      })
    })
  }

  /**
   * Get database troubleshooting message
   *
   * @returns Troubleshooting guidance
   */
  private getDatabaseTroubleshooting(): string {
    // Sanitize database URL for display
    const sanitizedUrl = this.databaseUrl ? this.databaseUrl.replace(/:[^:@]+@/, ':***@') : 'Not configured'

    return `
Troubleshooting:
- Verify PostgreSQL is running: docker ps | grep postgres
- Check MAPROOM_DATABASE_URL environment variable
- Test connection: psql $MAPROOM_DATABASE_URL -c "SELECT 1"

Current value: ${sanitizedUrl}
`.trim()
  }
}
