import { Command } from 'commander';
import { loadConfig } from '../config/loader';
import { TmuxService } from '../tmux/tmux.service';
import { logger } from '../utils/logger';

export function registerSessionCommands(program: Command): void {
  const session = new Command('session').description('Tmux session operations');

  session
    .command('start')
    .description('Start tmux session with orchestrator pane')
    .action(async () => {
      try {
        const config = await loadConfig();
        const tmux = new TmuxService(config.tmux.sessionName);
        tmux.startSession();
        logger.success(`Started tmux session '${config.tmux.sessionName}'`);
      } catch (err) {
        logger.error('Failed to start tmux session:', err);
        process.exitCode = 1;
      }
    });

  program.addCommand(session);
}


