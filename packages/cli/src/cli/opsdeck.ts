import { Command } from 'commander';
import { TmuxService } from '../tmux/tmux.service';
import { loadConfig } from '../config/loader';
import { logger } from '../utils/logger';
import { spawnSync } from 'node:child_process';

function hasOpsdeckBinary(): boolean {
  const res = spawnSync('crewchief-opsdeck', ['--version']);
  return res.status === 0;
}

export function registerOpsdeckCommand(program: Command): void {
  const cmd = new Command('opsdeck').description('Launch the Ops Deck dashboard (requires crewchief-opsdeck)');

  cmd
    .option('--mode <mode>', 'opsdeck|roster', 'opsdeck')
    .option('--fps <n>', 'frames per second', '2')
    .action(async (opts: { mode: 'opsdeck' | 'roster'; fps: string }) => {
      try {
        if (!hasOpsdeckBinary()) {
          logger.error('Missing `crewchief-opsdeck` binary on PATH. Please install from releases.');
          logger.info('Example: download the binary and add it to your PATH. Homebrew tap coming later.');
          return;
        }
        const config = await loadConfig();
        const tmux = new TmuxService(config.tmux.sessionName);
        tmux.ensureSession();
        const paneId = tmux.createPane('vertical');
        const fpsArg = parseInt(opts.fps, 10) || 2;
        const startCmd = `crewchief-opsdeck ui --mode ${opts.mode} --fps ${fpsArg}`;
        tmux.sendKeys(paneId, startCmd);
        logger.success(`Ops Deck started in pane ${paneId} (mode=${opts.mode}, fps=${fpsArg})`);
      } catch (err) {
        logger.error('Failed to launch Ops Deck:', err);
        process.exitCode = 1;
      }
    });

  program.addCommand(cmd);
}


