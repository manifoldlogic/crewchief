/**
 * Git utilities for extracting repository and branch information
 */

import { execFile } from 'node:child_process'
import { promisify } from 'node:util'

const execFileAsync = promisify(execFile)

/**
 * Get repository name from git remote origin URL
 *
 * Extracts owner/repo from URLs like:
 * - https://github.com/owner/repo.git
 * - git@github.com:owner/repo.git
 *
 * @param workspaceRoot - Workspace root directory
 * @returns Repository name in format "owner/repo", or "unknown" if not found
 */
export async function getRepoName(workspaceRoot: string): Promise<string> {
  try {
    const { stdout } = await execFileAsync('git', ['remote', 'get-url', 'origin'], {
      cwd: workspaceRoot,
    })

    const url = stdout.trim()

    // Extract owner/repo from GitHub URL
    // Handles both HTTPS and SSH formats
    const match = url.match(/[:/]([^/]+\/[^/]+?)(?:\.git)?$/)
    if (match) {
      return match[1]
    }

    return 'unknown'
  } catch (error) {
    // No git remote or not a git repo
    return 'unknown'
  }
}

/**
 * Get current git branch name
 *
 * @param workspaceRoot - Workspace root directory
 * @returns Branch name, or "main" if not found
 */
export async function getBranchName(workspaceRoot: string): Promise<string> {
  try {
    const { stdout } = await execFileAsync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], {
      cwd: workspaceRoot,
    })

    return stdout.trim()
  } catch (error) {
    // Not a git repo or detached HEAD
    return 'main'
  }
}
