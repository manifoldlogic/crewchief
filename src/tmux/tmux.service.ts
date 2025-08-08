import { spawnSync } from 'node:child_process';

export type PaneLayout = 'horizontal' | 'vertical';

export class TmuxService {
  private sessionName: string;

  constructor(sessionName: string) {
    this.sessionName = sessionName;
  }

  ensureTmux(): void {
    const res = spawnSync('tmux', ['-V']);
    if (res.status !== 0) {
      throw new Error('tmux not found. Please install tmux.');
    }
  }

  startSession(): void {
    this.ensureTmux();
    spawnSync('tmux', ['new-session', '-d', '-s', this.sessionName]);
  }

  createPane(layout: PaneLayout = 'vertical'): string {
    const flag = layout === 'vertical' ? '-v' : '-h';
    const res = spawnSync('tmux', ['split-window', flag, '-P', '-F', '#{pane_id}', '-t', this.sessionName]);
    return res.stdout.toString().trim();
  }
}


