import { randomUUID } from 'node:crypto'
import { Command } from 'commander'
import { RunManager } from '../orchestrator/runManager'
import { Scheduler } from '../orchestrator/scheduler'
import { Task } from '../orchestrator/task.types'
import { TerminalFactory } from '../terminal/factory'
import { logger } from '../utils/logger'

export function registerTaskCommands(program: Command): void {
  const taskCmd = new Command('task').description('Task orchestration')

  taskCmd
    .command('assign')
    .argument('<platform>')
    .argument('<description>')
    .description('Assign a simple task to a platform')
    .action(async (platform: string, description: string) => {
      const task: Task = {
        id: randomUUID(),
        description,
        requirements: [],
        acceptanceCriteria: [],
      }
      const terminal = TerminalFactory.autoDetect()
      const runManager = new RunManager()
      const scheduler = new Scheduler(terminal, runManager)
      const runId = await scheduler.spawnAgent(task.description, platform, {
        useWorktree: false,
      })
      logger.success(`Assigned task ${task.id} to ${platform} run=${runId}`)
    })

  program.addCommand(taskCmd)
}
