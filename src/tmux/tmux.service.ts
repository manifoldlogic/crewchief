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

  hasSession(): boolean {
    const res = spawnSync('tmux', ['has-session', '-t', this.sessionName]);
    return res.status === 0;
  }

  ensureSession(): void {
    if (!this.hasSession()) this.startSession();
  }

  sendKeys(paneId: string, command: string): void {
    this.ensureTmux();
    spawnSync('tmux', ['send-keys', '-t', paneId, command, 'C-m']);
  }

  closePane(paneId: string): void {
    this.ensureTmux();
    spawnSync('tmux', ['kill-pane', '-t', paneId]);
  }

  captureOutput(paneId: string): string {
    this.ensureTmux();
    const res = spawnSync('tmux', ['capture-pane', '-p', '-t', paneId]);
    return res.stdout.toString();
  }

  pipePaneToFile(paneId: string, filePath: string, append = true): void {
    this.ensureTmux();
    const redirect = append ? `cat >> ${filePath}` : `cat > ${filePath}`;
    spawnSync('tmux', ['pipe-pane', append ? '-o' : '', '-t', paneId, redirect].filter(Boolean as any));
  }
}


