/**
 * Factory for creating terminal service instances
 */

import { platform } from 'node:os'
import { TmuxAdapter } from './tmux.adapter.js'
import { ITermAdapter } from './iterm.adapter.js'
import type { ITerminalService, IAgentTerminalService } from './terminal.interface.js'

export type TerminalBackend = 'tmux' | 'iterm' | 'auto'

export interface TerminalConfig {
  backend?: TerminalBackend
  sessionName?: string
  bridgePort?: number
}

export class TerminalFactory {
  /**
   * Create a terminal service instance based on configuration
   */
  static create(config?: TerminalConfig): ITerminalService | IAgentTerminalService {
    const backend = config?.backend ?? 'auto'
    const sessionName = config?.sessionName ?? 'crewchief'

    // Determine which backend to use
    let selectedBackend: TerminalBackend = backend

    if (backend === 'auto') {
      // Auto-detect best backend
      if (platform() === 'darwin' && process.env.TERM_PROGRAM === 'iTerm.app') {
        selectedBackend = 'iterm'
      } else {
        selectedBackend = 'tmux'
      }
    }

    // Create appropriate adapter
    switch (selectedBackend) {
      case 'iterm':
        if (platform() !== 'darwin') {
          console.warn('iTerm2 backend requested but not on macOS, falling back to tmux')
          return new TmuxAdapter(sessionName)
        }
        return new ITermAdapter(sessionName)

      case 'tmux':
      default:
        return new TmuxAdapter(sessionName)
    }
  }

  /**
   * Check if iTerm2 backend is available
   */
  static isITermAvailable(): boolean {
    return platform() === 'darwin' && process.env.TERM_PROGRAM === 'iTerm.app'
  }

  /**
   * Get the recommended backend for the current environment
   */
  static getRecommendedBackend(): TerminalBackend {
    if (this.isITermAvailable()) {
      return 'iterm'
    }
    return 'tmux'
  }
}