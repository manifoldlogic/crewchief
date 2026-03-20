/**
 * Tool description variant injection via worktree source code modification
 */

import { execSync } from 'child_process'
import { readFileSync, writeFileSync } from 'fs'
import { join, resolve, dirname } from 'path'
import { fileURLToPath } from 'url'
import type { Variant } from './types.js'

// ESM equivalent of __dirname
const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

/**
 * Creates a git worktree with a modified tool description variant
 *
 * This function:
 * 1. Creates a new worktree of the crewchief repo
 * 2. Modifies the search tool description in packages/maproom-mcp/src/index.ts
 * 3. Returns the worktree path and cleanup function
 *
 * @param variant - The tool description variant to inject
 * @param basePath - Base path for worktree creation (default: cwd)
 * @returns Worktree path and cleanup function
 *
 * @example
 * ```typescript
 * const variant = {
 *   id: 'variant-a-detailed',
 *   name: 'Detailed',
 *   description: 'Enhanced search description...'
 * }
 *
 * const { path, cleanup } = await createVariantWorktree(variant)
 * try {
 *   // Spawn agent in this worktree
 *   await spawnAgent({ workingDirectory: path })
 * } finally {
 *   await cleanup()
 * }
 * ```
 */
export async function createVariantWorktree(
  variant: Variant,
  basePath: string = process.cwd(),
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  // 1. Create worktree using CLI command (decoupled from internal implementation)
  const branchName = `variant-${variant.id}-${Date.now()}`
  let currentBranch: string
  try {
    currentBranch = execSync('git rev-parse --abbrev-ref HEAD', { encoding: 'utf-8' }).trim()
  } catch {
    throw new Error('createVariantWorktree must be called from within a git repository')
  }

  // Determine the CLI path - use the built CLI from the same package
  const cliPath = resolve(__dirname, '../../dist/cli/index.js')

  // Execute CLI command to create worktree
  // Note: worktree create now prints path to stdout by default (as the last line)
  const createCommand = `node "${cliPath}" worktree create "${branchName}" --branch "${currentBranch}" --no-copy-ignored`

  let worktreePath: string
  try {
    const output = execSync(createCommand, {
      cwd: basePath,
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
    })

    // The CLI prints the path as the last line of stdout
    // Other info messages may appear before it
    const lines = output.trim().split('\n')
    worktreePath = lines[lines.length - 1].trim()
    if (!worktreePath || !worktreePath.startsWith('/')) {
      throw new Error(`No valid path returned from CLI output: ${output}`)
    }
  } catch (error) {
    throw new Error(`Failed to create worktree via CLI: ${error}`)
  }

  try {
    // 2. Modify tool description in worktree's MCP server source
    // NOTE: Tool description is in packages/maproom-mcp/src/index.ts at line ~118
    const toolFilePath = join(worktreePath, 'packages/maproom-mcp/src/index.ts')

    // Read current file
    let content = readFileSync(toolFilePath, 'utf-8')

    // Replace description in the toolSchemas array
    // The pattern matches: description: `...template literal...`
    // Note: The description uses backticks (template literals)
    // Pattern explanation: matches content between backticks, handling nested backticks via \`
    const descriptionPattern = /(name:\s*'search'\s*,\s*description:\s*)`(?:[^`\\]|\\.)*`/s

    if (!descriptionPattern.test(content)) {
      throw new Error(
        `Could not find search tool description in ${toolFilePath}. ` +
          'Pattern may have changed - please verify tool definition format.',
      )
    }

    // Escape special characters in the variant description for JavaScript template literal
    const escapedDescription = variant.description
      .replace(/\\/g, '\\\\') // Escape backslashes first
      .replace(/`/g, '\\`') // Escape backticks
      .replace(/\$/g, '\\$') // Escape dollar signs (template expressions)

    content = content.replace(descriptionPattern, `$1\`${escapedDescription}\``)

    // Write back modified content
    writeFileSync(toolFilePath, content, 'utf-8')

    // 3. Return worktree info with cleanup function
    return {
      path: worktreePath,
      cleanup: async () => {
        // Use CLI command to remove worktree (decoupled from internal implementation)
        const cleanCommand = `node "${cliPath}" worktree clean "${branchName}"`
        try {
          execSync(cleanCommand, {
            cwd: basePath,
            encoding: 'utf-8',
            stdio: ['pipe', 'pipe', 'pipe'],
          })
        } catch (error) {
          throw new Error(`Failed to cleanup worktree via CLI: ${error}`)
        }
      },
    }
  } catch (error) {
    // Cleanup worktree on error using CLI command
    try {
      const cleanCommand = `node "${cliPath}" worktree clean "${branchName}"`
      execSync(cleanCommand, {
        cwd: basePath,
        encoding: 'utf-8',
        stdio: ['pipe', 'pipe', 'pipe'],
      })
    } catch (cleanupError) {
      // Log but don't throw - original error is more important
      console.error('Failed to cleanup worktree after error:', cleanupError)
    }
    throw error
  }
}
