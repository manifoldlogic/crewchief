import { randomUUID } from 'node:crypto'
import { Command } from 'commander'
import { Scheduler } from '../orchestrator/scheduler'
import { Task } from '../orchestrator/task.types'
import { logger } from '../utils/logger'

export function registerTaskCommands(program: Command): void {
  const taskCmd = new Command('task').description('Task orchestration')

  taskCmd
    .command('assign')
    .argument('<agentTypeId>')
    .argument('<description>')
    .description('Assign a simple task to a single agent type')
    .action(async (agentTypeId: string, description: string) => {
      const task: Task = {
        id: randomUUID(),
        description,
        requirements: [],
        acceptanceCriteria: [],
      }
      const scheduler = new Scheduler()
      const assignment = await scheduler.assignSingleAgent(task, agentTypeId)
      logger.success(`Assigned task ${task.id} to ${agentTypeId} run=${assignment.runId}`)
    })

  program.addCommand(taskCmd)
}
