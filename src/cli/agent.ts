import { Command } from 'commander';
import { getAgentType } from '../agents/registry';
import { logger } from '../utils/logger';
import { RunManager } from '../orchestrator/runManager';
import { WorktreeService } from '../git/worktrees';
import { loadConfig } from '../config/loader';
import { TmuxService } from '../tmux/tmux.service';
import { randomUUID } from 'node:crypto';

export function registerAgentCommands(program: Command): void {
  const agent = new Command('agent').description('Agent lifecycle');

  agent
    .command('spawn')
    .argument('<type>')
    .argument('<task>')
    .description('Spawn an agent of <type> to work on <task> (mock)')
    .action(async (typeId: string, task: string) => {
      const type = getAgentType(typeId);
      if (!type) {
        logger.error(`Unknown agent type: ${typeId}`);
        process.exitCode = 1;
        return;
      }
      const config = await loadConfig();
      // Create unique branch/worktree name
      const short = randomUUID().slice(0, 8);
      const branchName = `crewchief-${typeId}-${short}`;
      const wt = new WorktreeService();
      const worktreePath = await wt.createWorktree(branchName, config.repository.mainBranch, config.repository.worktreeBasePath);

      const tmux = new TmuxService(config.tmux.sessionName);
      tmux.ensureSession();
      const paneId = tmux.createPane('vertical');

      const rm = new RunManager();
      const run = rm.createRun(typeId, task, paneId, worktreePath);

      // Start the agent command in the pane within the new worktree path
      const startCmd = `cd ${JSON.stringify(worktreePath)} && ${type.executionCommand}`;
      tmux.sendKeys(paneId, startCmd);

      // Pipe pane output to a run file for later parsing
      tmux.pipePaneToFile(paneId, `${rm.getRunDir(run.id)}/pane.log`, true);

      logger.success(`Spawned ${typeId} in ${worktreePath} [pane=${paneId}] [run=${run.id}]`);
    });

  agent
    .command('message')
    .argument('<agentId>')
    .argument('<message>')
    .description('Send a message to an agent (mock)')
    .action(async (agentId: string, message: string) => {
      const rm = new RunManager();
      const run = rm.getRunByAgentType(agentId);
      if (!run) {
        logger.warn(`Agent ${agentId} not running`);
        return;
      }
      const config = await loadConfig();
      const tmux = new TmuxService(config.tmux.sessionName);
      tmux.sendKeys(run.paneId, message);
      rm.appendLog(run.id, 'messages.log', `[in] ${message}`);
      logger.info(`[${agentId}] <= ${message} [pane=${run.paneId}] [run=${run.id}]`);
    });

  agent
    .command('close')
    .argument('<agentId>')
    .description('Close an agent and optionally merge work (mock)')
    .action(async (agentId: string) => {
      const rm = new RunManager();
      const run = rm.getRunByAgentType(agentId);
      if (!run) {
        logger.warn(`Agent ${agentId} not running`);
        return;
      }
      const config = await loadConfig();
      const tmux = new TmuxService(config.tmux.sessionName);
      tmux.closePane(run.paneId);
      rm.updateRun(run.id, { status: 'closed' });
      logger.success(`Closed agent ${agentId} [pane=${run.paneId}] [run=${run.id}]`);
    });

  program.addCommand(agent);
}


