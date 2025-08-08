import { Command } from 'commander';
import { getAgentType } from '../agents/registry';
import { logger } from '../utils/logger';
import { RunManager } from '../orchestrator/runManager';

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
      const rm = new RunManager();
      const run = rm.createRun(typeId, task, 'pane-unknown', process.cwd());
      logger.success(`Spawned ${typeId} (mock) for task: ${task} [run=${run.id}]`);
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
      rm.appendLog(run.id, 'messages.log', `[in] ${message}`);
      logger.info(`[${agentId}] <= ${message} [run=${run.id}]`);
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
      rm.updateRun(run.id, { status: 'closed' });
      logger.success(`Closed agent ${agentId} [run=${run.id}]`);
    });

  program.addCommand(agent);
}


