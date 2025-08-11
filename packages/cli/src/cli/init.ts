import path from 'node:path';
import { Command } from 'commander';
import { loadConfig } from '../config/loader';
import { WorktreeService } from '../git/worktrees';
import { logger } from '../utils/logger';

export function registerInitCommand(program: Command): void {
  program
    .command('init')
    .description('Initialize repository for multi-agent work')
    .action(async () => {
      try {
        const config = await loadConfig();
        const wt = new WorktreeService();
        const basePath = config.repository.worktreeBasePath;
        await wt.initRepository(basePath);
        logger.success(`Initialized worktree storage at ${path.resolve(basePath)}`);
      } catch (err) {
        logger.error('Initialization failed:', err);
        process.exitCode = 1;
      }
    });
}


