/**
 * Tmux adapter for the terminal interface
 */

import type { ITerminalService, TerminalLayout } from './terminal.interface.js'
import { TmuxService } from '../tmux/tmux.service.js'

export class TmuxAdapter implements ITerminalService {
  private tmux: TmuxService

  constructor(sessionName: string) {
    this.tmux = new TmuxService(sessionName)
  }

  ensureBackend(): void {
    this.tmux.ensureTmux()
  }

  startSession(): void {
    this.tmux.startSession()
  }

  isInsideTerminal(): boolean {
    return this.tmux.isInsideTmux()
  }

  attach(): void {
    this.tmux.attach()
  }

  createPane(layout?: TerminalLayout): string {
    return this.tmux.createPane(layout)
  }

  hasSession(): boolean {
    return this.tmux.hasSession()
  }

  ensureSession(): void {
    this.tmux.ensureSession()
  }

  sendKeys(paneId: string, command: string): void {
    this.tmux.sendKeys(paneId, command)
  }

  sendLine(paneId: string, text: string): void {
    this.tmux.sendLine(paneId, text)
  }

  closePane(paneId: string): void {
    this.tmux.closePane(paneId)
  }

  captureOutput(paneId: string): string {
    return this.tmux.captureOutput(paneId)
  }

  createWindow(): string {
    return this.tmux.createWindow()
  }

  createWindowWithCommand(command: string, cwd?: string): string {
    return this.tmux.createWindowWithCommand(command, cwd)
  }

  createWindowWithCwd(cwd?: string): { windowId: string; paneId: string } {
    return this.tmux.createWindowWithCwd(cwd)
  }

  createNamedWindow(
    name: string,
    cwd?: string,
  ): { windowId: string; paneId: string; target: string } {
    return this.tmux.createNamedWindow(name, cwd)
  }

  getAnyPaneId(): string {
    return this.tmux.getAnyPaneId()
  }

  async waitForReady(paneId: string, timeoutMs?: number): Promise<boolean> {
    return this.tmux.waitForReady(paneId, timeoutMs)
  }
}
