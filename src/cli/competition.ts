import { Command } from 'commander';
import { CompetitionManager } from '../orchestrator/competition';
import { logger } from '../utils/logger';

export function registerCompetitionCommands(program: Command): void {
  const comp = new Command('competition').description('Run competition mode across agents');

  comp
    .command('start')
    .argument('<description>')
    .argument('<agentIds...>')
    .description('Create a new competition')
    .action(async (description: string, agentIds: string[]) => {
      const cm = new CompetitionManager();
      const c = cm.start(description, agentIds);
      logger.success(`Competition ${c.id} created with agents: ${agentIds.join(', ')}`);
    });

  comp
    .command('assign')
    .argument('<competitionId>')
    .description('Assign task to all competition agents')
    .action(async (competitionId: string) => {
      const cm = new CompetitionManager();
      const c = await cm.assign(competitionId);
      logger.success(`Competition ${c.id} assigned: ${c.participants.map((p) => p.runId).join(', ')}`);
    });

  comp
    .command('evaluate')
    .argument('<competitionId>')
    .description('Evaluate competition runs and pick winner')
    .action(async (competitionId: string) => {
      const cm = new CompetitionManager();
      const c = await cm.evaluate(competitionId);
      logger.success(`Competition ${c.id} winner: ${c.winner ?? 'n/a'}`);
    });

  program.addCommand(comp);
}


