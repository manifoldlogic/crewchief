import { Command } from 'commander';
import path from 'node:path';
import { loadConfig } from '../config/loader';
import { WorktreeService } from '../git/worktrees';
import { logger } from '../utils/logger';

export function registerWorktreeCommands(program: Command): void {
  const worktree = new Command('worktree').description('Worktree operations');

  worktree
    .command('create')
    .argument('<name>')
    .option('--branch <branch>', 'Base branch')
    .description('Create named worktree from branch')
    .action(async (name: string, options: { branch?: string }) => {
      try {
        const config = await loadConfig();
        const baseBranch = options.branch ?? config.repository.mainBranch;
        const wt = new WorktreeService();
        const wtPath = await wt.createWorktree(name, baseBranch, config.repository.worktreeBasePath);
        logger.success(`Created worktree '${name}' at ${path.resolve(wtPath)} from ${baseBranch}`);
      } catch (err) {
        logger.error('Failed to create worktree:', err);
        process.exitCode = 1;
      }
    });

  worktree
    .command('list')
    .description('List active worktrees')
    .action(async () => {
      try {
        const wt = new WorktreeService();
        const list = await wt.listWorktrees();
        if (list.length === 0) {
          logger.info('No worktrees found');
          return;
        }
        for (const item of list) {
          logger.info(`${item.path}${item.branch ? ` [${item.branch}]` : ''}`);
        }
      } catch (err) {
        logger.error('Failed to list worktrees:', err);
        process.exitCode = 1;
      }
    });

  worktree
    .command('clean')
    .description('Prune stale worktree metadata and entries')
    .action(async () => {
      try {
        const wt = new WorktreeService();
        await wt.pruneWorktrees();
        logger.success('Pruned worktrees');
      } catch (err) {
        logger.error('Failed to prune worktrees:', err);
        process.exitCode = 1;
      }
    });

  program.addCommand(worktree);
}


