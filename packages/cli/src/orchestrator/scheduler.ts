import { safeClosePane, safeRemoveWorktree } from './rollback'
import { RunManager } from './runManager'
import { Task, TaskAssignment } from './task.types'
import { getAgentType } from '../agents/registry'
import { loadConfig } from '../config/loader'
import { WorktreeService, buildDeterministicBranchName } from '../git/worktrees'
import { TmuxService } from '../tmux/tmux.service'

export class Scheduler {
  async assignSingleAgent(task: Task, agentTypeId: string): Promise<TaskAssignment> {
    const config = await loadConfig()
    const wt = new WorktreeService()
    const branchName = buildDeterministicBranchName({ agentTypeId, taskDescription: task.description })
    let worktreePath = ''
    let paneId = ''
    try {
      worktreePath = await wt.createWorktree(
        branchName,
        config.repository.mainBranch,
        config.repository.worktreeBasePath,
      )

      const tmux = new TmuxService(config.tmux.sessionName)
      tmux.ensureSession()
      paneId = tmux.createPane('vertical')

      const type = getAgentType(agentTypeId)
      if (!type) throw new Error(`Unknown agent type: ${agentTypeId}`)

      const rm = new RunManager()
      const run = rm.createRun(agentTypeId, task.description, paneId, worktreePath, branchName)

      // start agent in pane (delegate to agent CLI protocol for now)
      const exec = type.executionCommand.includes('scripts/mock-agent.js')
        ? `node ${JSON.stringify(process.cwd() + '/scripts/mock-agent.js')}`
        : type.executionCommand
      tmux.sendKeys(paneId, `cd ${JSON.stringify(worktreePath)} && ${exec}`)
      tmux.pipePaneToFile(paneId, `${rm.getRunDir(run.id)}/pane.log`, true)

      const assignment: TaskAssignment = {
        taskId: task.id,
        agentId: agentTypeId,
        worktreeId: worktreePath,
        startTime: new Date().toISOString(),
        status: 'assigned',
        runId: run.id,
      }
      return assignment
    } catch (err) {
      // Best-effort cleanup
      if (paneId) await safeClosePane(new TmuxService(config.tmux.sessionName), paneId)
      if (worktreePath) await safeRemoveWorktree(wt, worktreePath)
      throw err
    }
  }
}
