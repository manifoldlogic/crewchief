import { TerminalProvider } from './interface'
import { HeadlessProvider } from './providers/headless'
import { ITermProvider } from './providers/iterm'
import { MockProvider } from './providers/mock'
import { TmuxProvider } from './providers/tmux'

export class TerminalFactory {
  /**
   * Auto-detects the best terminal provider for the current environment.
   *
   * Detection order:
   * 1. --headless flag → HeadlessProvider
   * 2. TMUX env var → TmuxProvider
   * 3. TERM_PROGRAM === 'iTerm.app' → ITermProvider
   * 4. Fallback → HeadlessProvider
   */
  static autoDetect(): TerminalProvider {
    // Check for --headless flag first (highest priority)
    if (process.argv.includes('--headless')) {
      return new HeadlessProvider()
    }

    // Check for tmux environment
    if (process.env.TMUX) {
      return new TmuxProvider()
    }

    // Existing iTerm detection
    if (process.env.TERM_PROGRAM === 'iTerm.app') {
      return new ITermProvider()
    }

    // Default fallback
    return new HeadlessProvider()
  }

  /**
   * Returns a specific provider by ID.
   */
  static getProvider(id: 'iterm' | 'headless' | 'mock' | 'tmux'): TerminalProvider {
    switch (id) {
      case 'tmux':
        return new TmuxProvider()
      case 'iterm':
        return new ITermProvider()
      case 'headless':
        return new HeadlessProvider()
      case 'mock':
        return new MockProvider()
      default:
        throw new Error(`Unknown provider: ${id}`)
    }
  }
}
