import { Command } from 'commander';
import path from 'node:path';
import { loadConfig } from '../config/loader';
import { WorktreeService } from '../git/worktrees';
import { RunManager } from '../orchestrator/runManager';
import { logger } from '../utils/logger';
import { spawn } from 'node:child_process';

export function registerWorktreeCommands(program: Command): void {
  const worktree = new Command('worktree').description('Worktree operations');

  worktree
    .command('create')
    .argument('<name>')
    .option('--branch <base>', 'Base branch to create the worktree from')
    .option('--base-path <dir>', 'Base directory for storing worktrees')
    .description('Create a git worktree from a base branch (general-purpose)')
    .action(async (name: string, opts: { branch?: string; basePath?: string }) => {
      try {
        const config = await loadConfig();
        const baseBranch = opts.branch ?? config.repository.mainBranch;
        const basePath = opts.basePath ?? config.repository.worktreeBasePath;
        const wt = new WorktreeService();
        await wt.initRepository(basePath);
        const createdPath = await wt.createWorktree(name, baseBranch, basePath);
        logger.success(`Created worktree at ${createdPath} [${baseBranch}]`);
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
        const rm = new RunManager();
        const runs = rm.listRuns();
        if (list.length === 0) {
          logger.info('No worktrees found');
          return;
        }
        for (const item of list) {
          const run = runs.find((r) => r.worktreePath === item.path && r.status === 'running');
          const suffix = [
            item.branch ? `[${item.branch}]` : undefined,
            run ? `agent=${run.agentTypeId}` : undefined,
            run ? `status=${run.status}` : undefined
          ]
            .filter(Boolean)
            .join(' ');
          logger.info(`${item.path}${suffix ? ' ' + suffix : ''}`);
        }
      } catch (err) {
        logger.error('Failed to list worktrees:', err);
        process.exitCode = 1;
      }
    });

  worktree
    .command('clean')
    .option('--stale', 'Remove stale worktrees only')
    .option('--all', 'Remove all non-current worktrees')
    .description('Prune worktree metadata and entries')
    .action(async (opts: { stale?: boolean; all?: boolean }) => {
      try {
        const wt = new WorktreeService();
        if (opts.all) {
          await wt.pruneWorktrees({ mode: 'all' });
          logger.success('Removed all non-current worktrees');
        } else if (opts.stale) {
          await wt.pruneWorktrees({ mode: 'stale' });
          logger.success('Removed stale worktrees');
        } else {
          await wt.pruneWorktrees();
          logger.success('Pruned worktrees');
        }
      } catch (err) {
        logger.error('Failed to prune worktrees:', err);
        process.exitCode = 1;
      }
    });

  worktree
    .command('cd')
    .argument('<selector>')
    .option('-p, --print', 'Print the absolute path instead of starting a subshell')
    .description(
      'Resolve a worktree by branch/name/path and start a subshell there by default. Use --print to output the absolute path for command substitution.'
    )
    .action(async (selector: string, opts: { print?: boolean }) => {
      try {
        const wt = new WorktreeService();
        const list = await wt.listWorktrees();
        const matches = list.filter((item) => {
          const sel = selector.trim();
          const byBranch = item.branch && item.branch === sel;
          const byBaseName = path.basename(item.path) === sel;
          let byPath = false;
          try {
            const resolvedSel = path.resolve(sel);
            byPath = path.resolve(item.path) === resolvedSel || path.resolve(item.path).includes(resolvedSel);
          } catch {}
          return Boolean(byBranch || byBaseName || byPath);
        });
        if (matches.length === 0) {
          logger.error(`No matching worktree for '${selector}'. Try using branch name or worktree directory name.`);
          process.exitCode = 1;
          return;
        }
        if (matches.length > 1) {
          logger.error(`Ambiguous selector '${selector}'. Candidates:`);
          for (const m of matches) logger.info(`${m.path}${m.branch ? ` [${m.branch}]` : ''}`);
          process.exitCode = 1;
          return;
        }
        const targetPath = path.resolve(matches[0].path);
        if (opts.print) {
          process.stdout.write(targetPath + '\n');
          return;
        }
        const shell = process.env.SHELL || '/bin/bash';
        logger.info(`Starting subshell in ${targetPath}. Exit to return.`);
        const child = spawn(shell, { stdio: 'inherit', cwd: targetPath, env: process.env });
        child.on('exit', (code) => process.exit(code ?? 0));
      } catch (err) {
        logger.error('Failed to resolve worktree:', err);
        process.exitCode = 1;
      }
    });

  program.addCommand(worktree);
}


