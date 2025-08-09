import { Command } from 'commander';
import { GitMergeService, MergeStrategyType } from '../git/merge';
import { logger } from '../utils/logger';
import { evaluateAndMaybeMerge } from '../orchestrator/autoMerge';

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

  // Place the 'auto' subcommand under the 'merge' group to match README usage: `crewchief merge auto <runId>`
  merge
    .command('auto')
    .description('Evaluate run and auto-merge if passing')
    .argument('<runId>')
    .action(async (runId: string) => {
      try {
        const { merged, score, reason } = await evaluateAndMaybeMerge(runId);
        if (merged) logger.success(`Auto-merged (score=${score.toFixed(2)})`);
        else {
          logger.warn(`Not merged (score=${score.toFixed(2)}): ${reason ?? ''}`);
        }
      } catch (err) {
        logger.error('Auto-merge error:', err);
        process.exitCode = 1;
      }
    });
  program.addCommand(merge);
}


