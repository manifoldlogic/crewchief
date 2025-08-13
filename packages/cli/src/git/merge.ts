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
}
