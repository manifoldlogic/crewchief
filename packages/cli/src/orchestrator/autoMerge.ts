import { RunManager } from './runManager'
import { loadConfig } from '../config/loader'
import { runDefaultChecks } from '../evaluation/checks'
import { GitMergeService } from '../git/merge'

export async function evaluateAndMaybeMerge(
  runId: string,
): Promise<{ merged: boolean; score: number; reason?: string }> {
  const rm = new RunManager()
  const run = rm.getRun(runId)
  if (!run) throw new Error('Run not found')
  const runDir = rm.getRunDir(runId)
  const summary = await runDefaultChecks(run.worktreePath, runDir)

  const config = await loadConfig()
  const threshold = config.evaluation.autoMergeThreshold ?? 0.95
  if (summary.score >= threshold && run.branchName) {
    const merge = new GitMergeService()
    const result = await merge.merge({
      sourceBranch: run.branchName,
      targetBranch: config.repository.mainBranch,
      strategy: 'squash',
    })
    return { merged: result.success, score: summary.score, reason: result.message }
  }
  return { merged: false, score: summary.score, reason: 'below threshold' }
}
