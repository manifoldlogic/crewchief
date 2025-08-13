import fs from 'node:fs'
import path from 'node:path'
import { Command } from 'commander'
import { RunManager } from '../orchestrator/runManager'
import { tailFileSync } from '../utils/log'
import { logger } from '../utils/logger'

export function registerRunsCommands(program: Command): void {
  const runs = new Command('runs').description('Inspect agent runs')

  runs
    .command('list')
    .description('List persisted runs')
    .action(async () => {
      const rm = new RunManager()
      const list = rm.listRuns()
      if (list.length === 0) {
        logger.info('No runs')
        return
      }
      for (const run of list) {
        logger.info(`${run.id} ${run.agentTypeId} [${run.status}] ${run.worktreePath} pane=${run.paneId}`)
      }
    })

  runs
    .command('events')
    .argument('<runId>')
    .description('Show parsed JSONL events for a run')
    .action(async (runId: string) => {
      const rm = new RunManager()
      const run = rm.getRun(runId)
      if (!run) {
        logger.warn(`Run not found: ${runId}`)
        return
      }
      const eventsPath = path.join(rm.getRunDir(runId), 'events.log')
      if (!fs.existsSync(eventsPath)) {
        logger.warn('No events.log found yet')
        return
      }
      const content = fs.readFileSync(eventsPath, 'utf8')
      process.stdout.write(content)
    })

  runs
    .command('logs')
    .argument('<runId>')
    .option('--tail <n>', 'Number of lines from the end', '200')
    .description('Show a summary of run logs (pane/events/orchestrator)')
    .action(async (runId: string, options: { tail: string }) => {
      const rm = new RunManager()
      const run = rm.getRun(runId)
      if (!run) {
        logger.warn(`Run not found: ${runId}`)
        return
      }
      const n = Number(options.tail ?? '200')
      const dir = rm.getRunDir(runId)
      const pane = tailFileSync(path.join(dir, 'pane.log'), n)
      const events = tailFileSync(path.join(dir, 'events.log'), n)
      const orch = tailFileSync(path.join(dir, 'orchestrator.log'), n)
      logger.info('--- pane.log ---')
      process.stdout.write(pane + '\n')
      logger.info('--- events.log ---')
      process.stdout.write(events + '\n')
      logger.info('--- orchestrator.log ---')
      process.stdout.write(orch + '\n')
    })

  program.addCommand(runs)
}
