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

  comp
    .command('finalize')
    .argument('<competitionId>')
    .description('Evaluate and attempt auto-merge winner based on score threshold')
    .action(async (competitionId: string) => {
      const cm = new CompetitionManager();
      const result = await cm.finalize(competitionId);
      if (result.merged) {
        logger.success(`Competition ${result.competition.id} winner merged (score=${result.score?.toFixed(2)})`);
      } else {
        logger.warn(`Competition ${result.competition.id} not merged: ${result.reason ?? ''}`);
      }
    });

  program.addCommand(comp);
}


