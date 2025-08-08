#!/usr/bin/env node
import { Command } from 'commander';
import chalk from 'chalk';
import { registerInitCommand } from './init';

const program = new Command();

program
  .name('crewchief')
  .description('CrewChief: Multi-agent orchestration for AI agents via git worktrees and tmux')
  .version('0.1.0');

registerInitCommand(program);

program
  .command('session')
  .description('Tmux session operations')
  .command('start')
  .description('Start tmux session with orchestrator pane')
  .action(async () => {
    console.log(chalk.green('[session:start]'), 'Tmux session start stub');
  });

const worktree = new Command('worktree').description('Worktree operations');
worktree
  .command('create')
  .argument('<name>')
  .option('--branch <branch>', 'Base branch', 'main')
  .description('Create named worktree from branch')
  .action(async (name: string, options: { branch: string }) => {
    console.log(
      chalk.green('[worktree:create]'),
      `Creating worktree ${name} from ${options.branch} (stub)`
    );
  });

worktree
  .command('list')
  .description('List active worktrees')
  .action(async () => {
    console.log(chalk.green('[worktree:list]'), 'Listing worktrees (stub)');
  });

worktree
  .command('clean')
  .description('Clean completed/abandoned worktrees')
  .action(async () => {
    console.log(chalk.green('[worktree:clean]'), 'Cleaning worktrees (stub)');
  });

program.addCommand(worktree);

const agent = new Command('agent').description('Agent lifecycle');
agent
  .command('spawn')
  .argument('<type>')
  .argument('<task>')
  .description('Spawn an agent of <type> to work on <task>')
  .action(async (type: string, task: string) => {
    console.log(chalk.green('[agent:spawn]'), `Spawning ${type} on task: ${task} (stub)`);
  });

agent
  .command('message')
  .argument('<agentId>')
  .argument('<message>')
  .description('Send a message to an agent')
  .action(async (agentId: string, message: string) => {
    console.log(chalk.green('[agent:message]'), `-> ${agentId}: ${message} (stub)`);
  });

agent
  .command('close')
  .argument('<agentId>')
  .description('Close an agent and optionally merge work')
  .action(async (agentId: string) => {
    console.log(chalk.green('[agent:close]'), `Closing ${agentId} (stub)`);
  });

program.addCommand(agent);

program.parseAsync(process.argv);

