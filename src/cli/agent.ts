import { Command } from 'commander';
import { getAgentType } from '../agents/registry';
import { logger } from '../utils/logger';

// For M2 groundwork, we simulate agent runs in-memory
const runs = new Map<string, { type: string; task: string; status: string }>();

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
      runs.set(typeId, { type: typeId, task, status: 'running' });
      logger.success(`Spawned ${typeId} (mock) for task: ${task}`);
    });

  agent
    .command('message')
    .argument('<agentId>')
    .argument('<message>')
    .description('Send a message to an agent (mock)')
    .action(async (agentId: string, message: string) => {
      if (!runs.has(agentId)) {
        logger.warn(`Agent ${agentId} not running`);
        return;
      }
      logger.info(`[${agentId}] <= ${message}`);
    });

  agent
    .command('close')
    .argument('<agentId>')
    .description('Close an agent and optionally merge work (mock)')
    .action(async (agentId: string) => {
      const run = runs.get(agentId);
      if (!run) {
        logger.warn(`Agent ${agentId} not running`);
        return;
      }
      run.status = 'closed';
      logger.success(`Closed agent ${agentId}`);
    });

  program.addCommand(agent);
}


