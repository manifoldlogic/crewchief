import simpleGit, { SimpleGit } from 'simple-git'

export type MergeStrategyType = 'squash' | 'ff' | 'cherry-pick'

export interface MergeOptions {
  sourceBranch: string
  targetBranch: string
  strategy: MergeStrategyType
  commitMessage?: string
}

export interface MergeResult {
  success: boolean
  message?: string
}

export interface WorktreeMergeOptions {
  worktreePath: string
  sourceBranch: string
  targetBranch: string
  strategy: MergeStrategyType
  commitMessage?: string
  ignoredFilesCopied?: string[]
}

export interface ChangesStats {
  commitCount: number
  filesChanged: number
  insertions: number
  deletions: number
}

export class GitMergeService {
  private git: SimpleGit

  constructor(cwd: string = process.cwd()) {
    this.git = simpleGit({ baseDir: cwd })
  }

  async ensureClean(): Promise<void> {
    const status = await this.git.status()
    if (!status.isClean()) {
      throw new Error('Working tree not clean. Commit or stash changes before merging.')
    }
  }

  async checkout(branch: string): Promise<void> {
    await this.git.checkout(branch)
  }

  async merge(options: MergeOptions): Promise<MergeResult> {
    await this.ensureClean()
    await this.git.fetch()
    await this.checkout(options.targetBranch)
    try {
      if (options.strategy === 'squash') {
        await this.git.raw(['merge', '--squash', options.sourceBranch])
        const msg = options.commitMessage ?? `squash-merge ${options.sourceBranch} -> ${options.targetBranch}`
        await this.git.commit(msg)
        return { success: true, message: 'Squash merge completed' }
      }
      if (options.strategy === 'ff') {
        await this.git.merge([options.sourceBranch])
        return { success: true, message: 'Merge completed' }
      }
      if (options.strategy === 'cherry-pick') {
        const log = await this.git.log({ from: options.sourceBranch, to: options.sourceBranch })
        if (log.all.length === 0) return { success: false, message: 'No commits to cherry-pick' }
        for (const entry of log.all) {
          await this.git.raw(['cherry-pick', entry.hash])
        }
        return { success: true, message: 'Cherry-pick completed' }
      }
      return { success: false, message: `Unknown strategy ${options.strategy}` }
    } catch (err: any) {
      return { success: false, message: String(err?.message ?? err) }
    }
  }

  async getChangesStats(sourceBranch: string, targetBranch: string): Promise<ChangesStats> {
    // Get commit count
    const mergeBase = await this.git.raw(['merge-base', targetBranch, sourceBranch]).catch(() => targetBranch)
    const log = await this.git.log({ from: mergeBase.trim(), to: sourceBranch })
    const commitCount = log.all.length

    // Get diff stats
    const diffStat = await this.git.raw(['diff', '--stat', `${targetBranch}...${sourceBranch}`])
    const lines = diffStat.split('\n').filter((line) => line.trim())
    const lastLine = lines[lines.length - 1] || ''

    // Parse the summary line (e.g., "5 files changed, 123 insertions(+), 45 deletions(-)")
    const filesMatch = lastLine.match(/(\d+) files? changed/)
    const insertMatch = lastLine.match(/(\d+) insertions?\(\+\)/)
    const deleteMatch = lastLine.match(/(\d+) deletions?\(-\)/)

    return {
      commitCount,
      filesChanged: filesMatch ? parseInt(filesMatch[1]) : 0,
      insertions: insertMatch ? parseInt(insertMatch[1]) : 0,
      deletions: deleteMatch ? parseInt(deleteMatch[1]) : 0,
    }
  }

  async hasCommitsToMerge(sourceBranch: string, targetBranch: string): Promise<boolean> {
    try {
      const mergeBase = await this.git.raw(['merge-base', targetBranch, sourceBranch]).catch(() => targetBranch)
      const log = await this.git.log({ from: mergeBase.trim(), to: sourceBranch })
      return log.all.length > 0
    } catch {
      return false
    }
  }

  async generateMergeCommitMessage(options: WorktreeMergeOptions): Promise<string> {
    const stats = await this.getChangesStats(options.sourceBranch, options.targetBranch)

    let message = `Merge worktree '${options.sourceBranch}'\n\n`
    message += 'Changes from worktree:\n'
    message += `- ${stats.commitCount} commit${stats.commitCount !== 1 ? 's' : ''} merged\n`
    message += `- ${stats.filesChanged} file${stats.filesChanged !== 1 ? 's' : ''} changed, `
    message += `${stats.insertions} insertion${stats.insertions !== 1 ? 's' : ''}(+), `
    message += `${stats.deletions} deletion${stats.deletions !== 1 ? 's' : ''}(-)\n`

    if (options.ignoredFilesCopied && options.ignoredFilesCopied.length > 0) {
      message += '\nIgnored files updated:\n'
      for (const file of options.ignoredFilesCopied) {
        message += `- ${file}\n`
      }
    }

    message += `\nSource branch: ${options.targetBranch}\n`
    message += `Worktree branch: ${options.sourceBranch}`

    return message
  }

  async deleteBranch(branch: string, force: boolean = false): Promise<void> {
    const flag = force ? '-D' : '-d'
    await this.git.raw(['branch', flag, branch])
  }

  async isCurrentBranch(branch: string): Promise<boolean> {
    const current = await this.git.revparse(['--abbrev-ref', 'HEAD'])
    return current.trim() === branch
  }
}
