import path from 'node:path'
import { Command } from 'commander'
import { runDefaultChecks } from '../evaluation/checks'
import { RunManager } from '../orchestrator/runManager'
import { logger } from '../utils/logger'

export function registerEvalCommands(program: Command): void {
  const ev = new Command('eval').description('Evaluate run outputs')

  ev.command('run')
    .argument('<runId>')
    .description('Run default evaluation checks against a run')
    .action(async (runId: string) => {
      const rm = new RunManager()
      const run = rm.getRun(runId)
      if (!run) {
        logger.warn(`Run not found: ${runId}`)
        return
      }
      const runDir = rm.getRunDir(runId)
      const summary = await runDefaultChecks(run.worktreePath, runDir)
      logger.info(`score: ${summary.score.toFixed(2)}`)
      for (const r of summary.results) {
        logger.info(`${r.passed ? 'PASS' : 'FAIL'} ${r.name}${r.details ? ` - ${r.details}` : ''}`)
      }
    })

  program.addCommand(ev)
}
