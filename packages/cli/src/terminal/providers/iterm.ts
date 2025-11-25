import { ITermService } from '../../iterm/iterm.service'
import { logger } from '../../utils/logger'
import { TerminalProvider, WindowOptions, SplitDirection } from '../interface'

export class ITermProvider implements TerminalProvider {
  readonly id = 'iterm'
  private service: ITermService

  constructor() {
    this.service = new ITermService('crewchief')
  }

  async initialize(): Promise<void> {
    if (process.env.TERM_PROGRAM !== 'iTerm.app') {
      throw new Error('ITermProvider requires running in iTerm.app')
    }
    await this.service.startBridge()
  }

  async dispose(): Promise<void> {
    this.service.stopBridge()
  }

  async createWindow(options?: WindowOptions): Promise<string> {
    if (options?.title && options?.workingDirectory) {
      const result = await this.service.createNamedWindow(options.title, options.workingDirectory)
      return result.windowId
    } else if (options?.workingDirectory) {
      const result = await this.service.createWindowWithCwd(options.workingDirectory)
      return result.windowId
    } else {
      return await this.service.createWindow()
    }
  }

  async createTab(_windowId: string): Promise<string> {
    // iTermService fallback: creating a window is safest without complex RPC
    logger.warn('createTab falling back to createWindow in current iTerm implementation')
    return await this.service.createWindow()
  }

  async splitPane(targetId: string, direction: SplitDirection): Promise<string> {
    await this.service.focusSession(targetId)
    return await this.service.createPane(direction)
  }

  async runCommand(paneId: string, command: string): Promise<void> {
    await this.service.sendLine(paneId, command)
  }

  async focus(paneId: string): Promise<void> {
    await this.service.focusSession(paneId)
  }
}
