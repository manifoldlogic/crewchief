/**
 * Git utility functions for MCP server operations
 *
 * Provides helpers for:
 * - Checking if a commit is currently checked out
 * - Retrieving file contents from git history
 * - Validating git operations
 */

import { execa } from 'execa'

/**
 * Execute a git command and return stdout
 * @param args - Git command arguments (without 'git' prefix)
 * @param cwd - Working directory for git command
 * @returns stdout from git command
 * @throws Error if git command fails
 */
export async function execGit(args: string[], cwd?: string): Promise<string> {
  try {
    const { stdout } = await execa('git', args, { cwd })
    return stdout
  } catch (error: any) {
    throw new Error(`Git command failed: ${error.message}`)
  }
}

/**
 * Check if a specific commit is currently checked out
 * @param commit - Commit SHA to check (optional)
 * @param cwd - Working directory for git operations
 * @returns true if commit is checked out, false otherwise
 */
export async function isCommitCheckedOut(commit: string | undefined, cwd?: string): Promise<boolean> {
  if (!commit) {
    return true // No commit specified means current HEAD
  }

  try {
    const currentCommit = await execGit(['rev-parse', 'HEAD'], cwd)
    return currentCommit.trim() === commit.trim()
  } catch (error) {
    return false
  }
}

/**
 * Retrieve file contents from git history
 * @param commit - Commit SHA to retrieve file from
 * @param relpath - Relative path to file within repository
 * @param cwd - Working directory for git operations
 * @returns File contents as string
 * @throws Error if file doesn't exist in commit or git fails
 */
export async function getFileFromGit(commit: string, relpath: string, cwd?: string): Promise<string> {
  try {
    const content = await execGit(['show', `${commit}:${relpath}`], cwd)
    return content
  } catch (error: any) {
    if (error.message.includes('does not exist') || error.message.includes('Path') || error.message.includes('exists on disk')) {
      throw new Error(`File '${relpath}' not found in commit ${commit}`)
    }
    throw new Error(`Failed to retrieve file from git: ${error.message}`)
  }
}

/**
 * Get the repository root path
 * @param cwd - Working directory to start search from
 * @returns Absolute path to repository root
 * @throws Error if not in a git repository
 */
export async function getRepoRoot(cwd?: string): Promise<string> {
  try {
    const root = await execGit(['rev-parse', '--show-toplevel'], cwd)
    return root.trim()
  } catch (error: any) {
    throw new Error(`Not in a git repository: ${error.message}`)
  }
}
