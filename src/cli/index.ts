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
import { startOrchestratorEventBridge } from '../orchestrator/events';
import { registerSetupCommand } from './setup';

const program = new Command();

program
  .name('crewchief')
  .description('CrewChief: Multi-agent orchestration for AI agents via git worktrees and tmux')
  .version('0.1.0');

// Start event bridge for orchestrator messages
startOrchestratorEventBridge();

registerInitCommand(program);
registerSessionCommands(program);
registerWorktreeCommands(program);
registerAgentCommands(program);
registerRunsCommands(program);
registerEvalCommands(program);
registerTaskCommands(program);
registerMergeCommands(program);
registerCompetitionCommands(program);
registerSetupCommand(program);

program.parseAsync(process.argv);

