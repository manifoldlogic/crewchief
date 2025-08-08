import { randomUUID } from 'node:crypto';
import { Task, TaskAssignment } from './task.types';
import { getAgentType } from '../agents/registry';
import { RunManager } from './runManager';
import { WorktreeService } from '../git/worktrees';
import { TmuxService } from '../tmux/tmux.service';
import { loadConfig } from '../config/loader';

export class Scheduler {
  async assignSingleAgent(task: Task, agentTypeId: string): Promise<TaskAssignment> {
    const config = await loadConfig();
    const wt = new WorktreeService();
    const short = randomUUID().slice(0, 8);
    const branchName = `task-${short}-${agentTypeId}`;
    const worktreePath = await wt.createWorktree(branchName, config.repository.mainBranch, config.repository.worktreeBasePath);

    const tmux = new TmuxService(config.tmux.sessionName);
    tmux.ensureSession();
    const paneId = tmux.createPane('vertical');

    const type = getAgentType(agentTypeId);
    if (!type) throw new Error(`Unknown agent type: ${agentTypeId}`);

    const rm = new RunManager();
    const run = rm.createRun(agentTypeId, task.description, paneId, worktreePath, branchName);

    // start agent in pane (delegate to agent CLI protocol for now)
    const exec = type.executionCommand.includes('scripts/mock-agent.js')
      ? `node ${JSON.stringify(process.cwd() + '/scripts/mock-agent.js')}`
      : type.executionCommand;
    tmux.sendKeys(paneId, `cd ${JSON.stringify(worktreePath)} && ${exec}`);
    tmux.pipePaneToFile(paneId, `${rm.getRunDir(run.id)}/pane.log`, true);

    const assignment: TaskAssignment = {
      taskId: task.id,
      agentId: agentTypeId,
      worktreeId: worktreePath,
      startTime: new Date().toISOString(),
      status: 'assigned',
      runId: run.id
    };
    return assignment;
  }
}


