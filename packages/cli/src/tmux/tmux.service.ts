import { spawnSync } from 'node:child_process'

export type PaneLayout = 'horizontal' | 'vertical'

export class TmuxService {
  private sessionName: string

  constructor(sessionName: string) {
    this.sessionName = sessionName
  }

  ensureTmux(): void {
    const res = spawnSync('tmux', ['-V'])
    if (res.status !== 0) {
      throw new Error('tmux not found. Please install tmux.')
    }
  }

  startSession(): void {
    this.ensureTmux()
    spawnSync('tmux', ['new-session', '-d', '-s', this.sessionName])
  }

  isInsideTmux(): boolean {
    return Boolean(process.env.TMUX)
  }

  attach(): void {
    this.ensureTmux()
    const args = this.isInsideTmux()
      ? ['switch-client', '-t', this.sessionName]
      : ['attach-session', '-t', this.sessionName]
    spawnSync('tmux', args, { stdio: 'inherit' })
  }

  createPane(layout: PaneLayout = 'vertical'): string {
    const flag = layout === 'vertical' ? '-v' : '-h'
    const res = spawnSync('tmux', ['split-window', flag, '-P', '-F', '#{pane_id}', '-t', `${this.sessionName}:`])
    const out = res.stdout.toString().trim()
    if (res.status === 0 && out) return out
    // Fallback to the first available pane in the session
    return this.getAnyPaneId()
  }

  hasSession(): boolean {
    const res = spawnSync('tmux', ['has-session', '-t', this.sessionName])
    return res.status === 0
  }

  ensureSession(): void {
    if (!this.hasSession()) this.startSession()
  }

  sendKeys(paneId: string, command: string): void {
    this.ensureTmux()
    // Clear line and send full command then Enter, to avoid partial typing issues
    spawnSync('tmux', ['send-keys', '-t', paneId, 'C-c'])
    spawnSync('tmux', ['send-keys', '-t', paneId, 'C-u'])
    spawnSync('tmux', ['send-keys', '-t', paneId, '-l', command])
    spawnSync('tmux', ['send-keys', '-t', paneId, 'C-m'])
  }

  sendLine(paneId: string, text: string): void {
    this.ensureTmux()
    spawnSync('tmux', ['send-keys', '-t', paneId, '-l', text])
    spawnSync('tmux', ['send-keys', '-t', paneId, 'C-m'])
  }

  closePane(paneId: string): void {
    this.ensureTmux()
    spawnSync('tmux', ['kill-pane', '-t', paneId])
  }

  captureOutput(paneId: string): string {
    this.ensureTmux()
    const res = spawnSync('tmux', ['capture-pane', '-p', '-t', paneId])
    return res.stdout.toString()
  }

  pipePaneToFile(paneId: string, filePath: string, append = true): void {
    this.ensureTmux()
    const redirect = append ? `cat >> ${filePath}` : `cat > ${filePath}`
    spawnSync('tmux', ['pipe-pane', append ? '-o' : '', '-t', paneId, redirect].filter(Boolean as any))
  }

  createWindow(): string {
    this.ensureTmux()
    const res = spawnSync('tmux', ['new-window', '-d', '-t', this.sessionName, '-P', '-F', '#{pane_id}'])
    const out = res.stdout.toString().trim()
    if (res.status === 0 && out) return out
    return this.getAnyPaneId()
  }

  createWindowWithCommand(command: string, cwd?: string): string {
    this.ensureTmux()
    const args = ['new-window', '-d', '-t', this.sessionName, '-P', '-F', '#{pane_id}']
    if (cwd) {
      args.push('-c', cwd)
    }
    args.push(command)
    const res = spawnSync('tmux', args)
    const out = res.stdout.toString().trim()
    if (res.status === 0 && out) return out
    return this.getAnyPaneId()
  }

  createWindowWithCwd(cwd?: string): { windowId: string; paneId: string } {
    this.ensureTmux()
    const args = ['new-window', '-d', '-t', this.sessionName, '-P', '-F', '#{window_id}']
    if (cwd) {
      args.push('-c', cwd)
    }
    const res = spawnSync('tmux', args)
    const windowId = res.stdout.toString().trim()
    if (res.status !== 0 || !windowId) {
      // Fallback to current window and first pane
      const paneId = this.getAnyPaneId()
      const wid = this.getWindowIdForPane(paneId) ?? ''
      return { windowId: wid, paneId }
    }
    const paneList = spawnSync('tmux', ['list-panes', '-t', windowId, '-F', '#{pane_id}'])
    const paneId = (paneList.stdout.toString().trim().split('\n')[0] ?? '').trim()
    return { windowId, paneId: paneId || this.getAnyPaneId() }
  }

  createNamedWindow(name: string, cwd?: string): { windowId: string; paneId: string; target: string } {
    this.ensureTmux()
    const args = ['new-window', '-d', '-t', this.sessionName, '-n', name, '-P', '-F', '#{window_id}']
    if (cwd) {
      args.push('-c', cwd)
    }
    const res = spawnSync('tmux', args)
    const windowId = res.stdout.toString().trim()
    const target = `${this.sessionName}:${name}`
    if (res.status !== 0 || !windowId) {
      const paneId = this.getAnyPaneId()
      const wid = this.getWindowIdForPane(paneId) ?? ''
      return { windowId: wid, paneId, target }
    }
    const paneList = spawnSync('tmux', ['list-panes', '-t', windowId, '-F', '#{pane_id}'])
    const paneId = (paneList.stdout.toString().trim().split('\n')[0] ?? '').trim()
    return { windowId, paneId: paneId || this.getAnyPaneId(), target }
  }

  getAnyPaneId(): string {
    this.ensureTmux()
    const res = spawnSync('tmux', ['list-panes', '-t', this.sessionName, '-F', '#{pane_id}'])
    const out = res.stdout.toString().trim().split('\n')[0] ?? ''
    return out || '%0'
  }

  getWindowIdForPane(paneId: string): string | undefined {
    this.ensureTmux()
    const res = spawnSync('tmux', ['display-message', '-p', '-t', paneId, '#{window_id}'])
    const out = res.stdout.toString().trim()
    return res.status === 0 && out ? out : undefined
  }

  selectWindowById(windowId: string): void {
    this.ensureTmux()
    spawnSync('tmux', ['select-window', '-t', windowId])
  }

  async waitForReady(paneId: string, timeoutMs = 5000): Promise<boolean> {
    const marker = `CC_READY_${Math.random().toString(36).slice(2, 8)}`
    const deadline = Date.now() + timeoutMs
    while (Date.now() < deadline) {
      this.sendLine(paneId, `echo ${marker}`)
      const out = this.captureOutput(paneId)
      if (out.includes(marker)) return true
      await new Promise((r) => setTimeout(r, 200))
    }
    return false
  }
}
