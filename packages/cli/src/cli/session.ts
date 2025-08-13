import { Command } from 'commander'
import { runSetupWizard } from './setup'
import { loadConfig } from '../config/loader'
import { TmuxService } from '../tmux/tmux.service'
import { logger } from '../utils/logger'

// Session subcommand removed per simplified UX; keep file for potential future ops
export function registerSessionCommands(_program: Command): void {}
