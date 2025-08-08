import { Command } from 'commander';
import { GitMergeService, MergeStrategyType } from '../git/merge';
import { logger } from '../utils/logger';

export function registerMergeCommands(program: Command): void {
  const merge = new Command('merge').description('Merge run branches into target');

  merge
    .command('run')
    .argument('<sourceBranch>')
    .option('--target <branch>', 'Target branch', 'main')
    .option('--strategy <type>', 'ff|squash|cherry-pick', 'squash')
    .description('Merge a run branch into target branch')
    .action(async (sourceBranch: string, options: { target: string; strategy: MergeStrategyType }) => {
      try {
        const svc = new GitMergeService();
        const res = await svc.merge({ sourceBranch, targetBranch: options.target, strategy: options.strategy });
        if (res.success) logger.success(res.message ?? 'Merged');
        else {
          logger.error('Merge failed:', res.message ?? 'unknown');
          process.exitCode = 1;
        }
      } catch (err) {
        logger.error('Merge error:', err);
        process.exitCode = 1;
      }
    });

  program.addCommand(merge);
}


