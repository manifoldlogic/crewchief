import { Command } from 'commander';
import fs from 'node:fs';
import path from 'node:path';
import { RunManager } from '../orchestrator/runManager';
import { logger } from '../utils/logger';

export function registerRunsCommands(program: Command): void {
  const runs = new Command('runs').description('Inspect agent runs');

  runs
    .command('list')
    .description('List persisted runs')
    .action(async () => {
      const rm = new RunManager();
      const list = rm.listRuns();
      if (list.length === 0) {
        logger.info('No runs');
        return;
      }
      for (const run of list) {
        logger.info(`${run.id} ${run.agentTypeId} [${run.status}] ${run.worktreePath} pane=${run.paneId}`);
      }
    });

  runs
    .command('events')
    .argument('<runId>')
    .description('Show parsed JSONL events for a run')
    .action(async (runId: string) => {
      const rm = new RunManager();
      const run = rm.getRun(runId);
      if (!run) {
        logger.warn(`Run not found: ${runId}`);
        return;
      }
      const eventsPath = path.join(rm.getRunDir(runId), 'events.log');
      if (!fs.existsSync(eventsPath)) {
        logger.warn('No events.log found yet');
        return;
      }
      const content = fs.readFileSync(eventsPath, 'utf8');
      process.stdout.write(content);
    });

  program.addCommand(runs);
}


