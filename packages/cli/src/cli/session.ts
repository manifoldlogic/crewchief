import { Command } from 'commander';
import { loadConfig } from '../config/loader';
import { TmuxService } from '../tmux/tmux.service';
import { logger } from '../utils/logger';
import { runSetupWizard } from './setup';

// Session subcommand removed per simplified UX; keep file for potential future ops
export function registerSessionCommands(_program: Command): void {}


