/**
 * Factory for creating terminal service instances
 */

import { platform } from 'node:os'
import { ITermAdapter } from './iterm.adapter.js'
import type { ITerminalService, IAgentTerminalService } from './terminal.interface.js'

export type TerminalBackend = 'iterm' | 'auto'

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
        throw new Error('iTerm2 is required. Please install iTerm2: https://iterm2.com/downloads.html')
      }
    }

    // Create appropriate adapter
    switch (selectedBackend) {
      case 'iterm':
        if (platform() !== 'darwin') {
          throw new Error('iTerm2 backend requires macOS')
        }
        return new ITermAdapter(sessionName)

      default:
        throw new Error('Only iTerm2 backend is supported')
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
    throw new Error('iTerm2 is required but not available')
  }
}
