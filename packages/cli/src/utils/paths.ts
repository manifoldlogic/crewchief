import os from 'node:os'
import path from 'node:path'
import { simpleGit } from 'simple-git'

/**
 * Expands tilde (~) in a path to the user's home directory.
 * Only expands a leading ~ followed by / or end of string.
 * Returns the original string if no expansion is needed.
 *
 * @param pathStr - The path string to expand
 * @returns The expanded path
 *
 * @example
 * expandTilde('~') // '/home/user'
 * expandTilde('~/foo') // '/home/user/foo'
 * expandTilde('/abs/path') // '/abs/path'
 */
export function expandTilde(pathStr: string): string {
  if (pathStr === '~') {
    return os.homedir()
  }
  if (pathStr.startsWith('~/')) {
    return path.join(os.homedir(), pathStr.slice(2))
  }
  return pathStr
}

/**
 * Extracts the repository name from git remote URL or falls back to directory basename.
 * Uses simple-git with a 5-second timeout.
 * Sanitizes special characters (/, \, :, *, ?, ", <, >, |) by replacing with hyphens.
 * Limits result to 255 characters.
 *
 * @param cwd - Current working directory (defaults to process.cwd())
 * @returns The repository name
 *
 * @example
 * // In a repo with remote git@github.com:org/myrepo.git
 * await getRepositoryName() // 'myrepo'
 *
 * // In a repo with remote https://github.com/org/myrepo.git
 * await getRepositoryName() // 'myrepo'
 *
 * // In a non-git directory or on git error
 * await getRepositoryName('/path/to/mydir') // 'mydir'
 */
export async function getRepositoryName(cwd?: string): Promise<string> {
  const workingDir = cwd ?? process.cwd()

  try {
    const git = simpleGit({
      baseDir: workingDir,
      timeout: { block: 5000 },
    })

    // Get remote origin URL
    const output = await git.raw(['config', '--get', 'remote.origin.url'])
    const url = output.trim()

    // Parse repository name from URL
    // Matches: git@github.com:org/repo.git → "repo"
    //          https://github.com/org/repo.git → "repo"
    //          https://github.com/org/repo → "repo"
    const regex = /[/:]([^/:]+?)(\.git)?$/
    const match = url.match(regex)

    let repoName: string
    if (match && match[1]) {
      repoName = match[1]
    } else {
      // Fallback to directory basename
      repoName = path.basename(workingDir)
    }

    // Sanitize: Replace dangerous characters with hyphens
    const sanitized = repoName.replace(/[/\\:*?"<>|]/g, '-')

    // Limit to 255 characters
    return sanitized.slice(0, 255)
  } catch {
    // Git command failed (timeout, not a git repo, etc.) - fall back to directory basename
    const repoName = path.basename(workingDir)
    const sanitized = repoName.replace(/[/\\:*?"<>|]/g, '-')
    return sanitized.slice(0, 255)
  }
}

/**
 * Replaces all occurrences of <repo-name> placeholder with the actual repository name.
 * Calls getRepositoryName() to determine the repository name.
 *
 * @param pathStr - The path string containing placeholders
 * @param cwd - Current working directory (defaults to process.cwd())
 * @returns The path with placeholders replaced
 *
 * @example
 * // In a repo named "myrepo"
 * await expandRepoPlaceholder('~/.crewchief/<repo-name>') // '~/.crewchief/myrepo'
 * await expandRepoPlaceholder('/data/<repo-name>/<repo-name>-backup') // '/data/myrepo/myrepo-backup'
 */
export async function expandRepoPlaceholder(pathStr: string, cwd?: string): Promise<string> {
  // Only call getRepositoryName if the placeholder exists
  if (!pathStr.includes('<repo-name>')) {
    return pathStr
  }

  const repoName = await getRepositoryName(cwd)
  return pathStr.replace(/<repo-name>/g, repoName)
}

/**
 * System directories that should be rejected for safety.
 */
const SYSTEM_DIRECTORIES = ['/', '/etc', '/usr', '/System', 'C:\\Windows']

/**
 * Checks if a path is a system directory or subdirectory of one.
 *
 * @param pathStr - The absolute path to check
 * @returns true if the path is a system directory
 */
function isSystemDirectory(pathStr: string): boolean {
  const normalized = path.normalize(pathStr)

  for (const sysDir of SYSTEM_DIRECTORIES) {
    const normalizedSysDir = path.normalize(sysDir)

    // Exact match
    if (normalized === normalizedSysDir) {
      return true
    }

    // Subdirectory check (must be followed by path separator)
    if (normalized.startsWith(normalizedSysDir + path.sep)) {
      return true
    }
  }

  return false
}

/**
 * Performs full path expansion by chaining tilde expansion, placeholder replacement,
 * absolute path resolution, and system directory validation.
 *
 * @param pathStr - The path string to expand
 * @param cwd - Current working directory (defaults to process.cwd())
 * @returns The fully expanded absolute path
 * @throws Error if the resolved path is a system directory
 *
 * @example
 * // In a repo named "myrepo"
 * await expandWorktreePath('~/.crewchief/<repo-name>') // '/home/user/.crewchief/myrepo'
 * await expandWorktreePath('~/projects/<repo-name>') // '/home/user/projects/myrepo'
 *
 * // System directory rejection
 * await expandWorktreePath('/etc/worktrees') // throws Error
 */
export async function expandWorktreePath(pathStr: string, cwd?: string): Promise<string> {
  // Step 1: Expand tilde
  let expanded = expandTilde(pathStr)

  // Step 2: Replace <repo-name> placeholder
  expanded = await expandRepoPlaceholder(expanded, cwd)

  // Step 3: Make absolute
  expanded = path.resolve(expanded)

  // Step 4: Validate not a system directory
  if (isSystemDirectory(expanded)) {
    throw new Error(
      `Rejected system directory: "${expanded}"\n` +
        'Reason: Cannot create worktrees in system directories for safety\n' +
        'Example valid path: "~/.crewchief/worktrees/<repo-name>"',
    )
  }

  return expanded
}
