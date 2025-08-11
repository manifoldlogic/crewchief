import { Command } from 'commander';
import { getAgentType } from '../agents/registry';
import { logger } from '../utils/logger';
import { RunManager } from '../orchestrator/runManager';
import { WorktreeService, buildDeterministicBranchName } from '../git/worktrees';
import { loadConfig } from '../config/loader';
import { TmuxService } from '../tmux/tmux.service';
import { randomUUID } from 'node:crypto';
import { LogFollower } from '../bus/logFollower';
import { messageBus } from '../bus/index';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

export function registerAgentCommands(program: Command): void {
  const agent = new Command('agent').description('Agent lifecycle');

  agent
    .command('spawn')
    .argument('<type>')
    .argument('[task]')
    .option('--count <n>', 'Number of agents to spawn', '1')
    .option('--branch <base>', 'Base branch name to derive worktrees from')
    .option('--env <kv...>', 'Environment variables KEY=VAL to pass to the agent')
    .description('Spawn one or more agents of <type> (optionally provide a task)')
    .action(async (typeId: string, task: string | undefined, options: { count: string; branch?: string; env?: string[] }) => {
      const type = getAgentType(typeId);
      if (!type) {
        logger.error(`Unknown agent type: ${typeId}`);
        process.exitCode = 1;
        return;
      }
      const config = await loadConfig();
      const count = Math.max(1, parseInt(options.count, 10));
      const baseBranch = options.branch ?? config.repository.mainBranch;
      const envVars: Record<string, string> = {};
      for (const kv of options.env ?? []) {
        const [k, ...rest] = kv.split('=');
        if (k && rest.length) envVars[k] = rest.join('=');
      }

      const tmux = new TmuxService(config.tmux.sessionName);
      tmux.ensureSession();
      const rm = new RunManager();
      const wt = new WorktreeService();

      for (let i = 0; i < count; i++) {
        const branchName = buildDeterministicBranchName({ agentTypeId: typeId, taskDescription: task });
        const worktreePath = await wt.createWorktree(branchName, baseBranch, config.repository.worktreeBasePath);

        // Determine execution command (support mock-agent replacement)
        let execCmd = type.executionCommand;
        if (execCmd.includes('scripts/mock-agent.js')) {
          const here = path.dirname(fileURLToPath(import.meta.url));
          const pkgDir = path.resolve(here, '..');
          const cand = path.join(pkgDir, 'scripts', 'mock-agent.js');
          const fallback = path.join(process.cwd(), 'scripts', 'mock-agent.js');
          const abs = fs.existsSync(cand) ? cand : fallback;
          // Preserve uniqueness even with deterministic branch naming by using branchName in mock id
          execCmd = `MOCK_AGENT_ID=${typeId}-${branchName.slice(-8)} node ${JSON.stringify(abs)}`;
        }

        // Merge in provided env vars
        const envArgs = Object.entries(envVars)
          .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
          .join(' ');
        const fullCmd = `${envArgs} ${execCmd}`.trim();

        // Launch command directly in a new tmux window; embed cd into the command
        const cmd = `cd ${JSON.stringify(worktreePath)} && ${fullCmd}`;
        const paneId = tmux.createWindowWithCommand(cmd);
        const run = rm.createRun(typeId, task ?? '', paneId, worktreePath, branchName);

        const logPath = `${rm.getRunDir(run.id)}/pane.log`;
        tmux.pipePaneToFile(paneId, logPath, true);
        const follower = new LogFollower(logPath);
        follower.start((env) => {
          rm.appendLog(run.id, 'events.log', JSON.stringify(env));
          messageBus.send({
            type: 'status',
            from: `${typeId}`,
            to: 'orchestrator',
            payload: env,
            timestamp: new Date(),
            worktreeContext: { branch: run.branchName ?? '', modifiedFiles: [], lastCommit: '' }
          });
        });

        logger.success(`Spawned ${typeId} in ${worktreePath} [pane=${paneId}] [run=${run.id}]`);
      }
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


