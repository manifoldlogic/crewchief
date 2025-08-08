#!/usr/bin/env node
import { Command } from 'commander';
import chalk from 'chalk';
import { registerInitCommand } from './init';
import { registerWorktreeCommands } from './worktree';
import { registerSessionCommands } from './session';
import { registerAgentCommands } from './agent';
import { registerRunsCommands } from './runs';
import { registerEvalCommands } from './eval';
import { registerTaskCommands } from './task';
import { registerMergeCommands } from './merge';
import { registerCompetitionCommands } from './competition';

const program = new Command();

program
  .name('crewchief')
  .description('CrewChief: Multi-agent orchestration for AI agents via git worktrees and tmux')
  .version('0.1.0');

registerInitCommand(program);
registerSessionCommands(program);
registerWorktreeCommands(program);
registerAgentCommands(program);
registerRunsCommands(program);
registerEvalCommands(program);
registerTaskCommands(program);
registerMergeCommands(program);
registerCompetitionCommands(program);


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

