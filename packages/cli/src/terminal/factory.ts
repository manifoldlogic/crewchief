import { TerminalProvider } from './interface'
import { HeadlessProvider } from './providers/headless'
import { ITermProvider } from './providers/iterm'
import { MockProvider } from './providers/mock'

export class TerminalFactory {
  /**
   * Auto-detects the best terminal provider for the current environment.
   */
  static autoDetect(): TerminalProvider {
    if (process.argv.includes('--headless')) {
      return new HeadlessProvider()
    }

    if (process.env.TERM_PROGRAM === 'iTerm.app') {
      return new ITermProvider()
    }

    return new HeadlessProvider()
  }

  /**
   * Returns a specific provider by ID.
   */
  static getProvider(id: 'iterm' | 'headless' | 'mock'): TerminalProvider {
    switch (id) {
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
