import { TerminalProvider, WindowOptions, SplitDirection } from '../interface'

export class MockProvider implements TerminalProvider {
  readonly id = 'mock'

  // Public state for assertions
  public executedCommands: { paneId: string; command: string }[] = []
  public windows: string[] = []
  public panes: Record<string, { parent: string }> = {}

  private windowCounter = 0
  private paneCounter = 0

  async initialize(): Promise<void> {
    // No-op
  }

  async dispose(): Promise<void> {
    this.executedCommands = []
    this.windows = []
    this.panes = {}
    this.windowCounter = 0
    this.paneCounter = 0
  }

  async createWindow(_options?: WindowOptions): Promise<string> {
    this.windowCounter++
    const windowId = `mock-window-${this.windowCounter}`
    this.windows.push(windowId)

    // A window creates an initial pane
    this.paneCounter++
    const paneId = `mock-pane-${this.paneCounter}`
    this.panes[paneId] = { parent: windowId }

    return windowId
  }

  async createTab(windowId: string): Promise<string> {
    if (!this.windows.includes(windowId)) {
      throw new Error(`Window ${windowId} does not exist`)
    }

    this.paneCounter++
    const paneId = `mock-pane-${this.paneCounter}`
    this.panes[paneId] = { parent: windowId }

    return paneId
  }

  async splitPane(targetId: string, _direction: SplitDirection): Promise<string> {
    if (!this.panes[targetId]) {
      throw new Error(`Pane ${targetId} does not exist`)
    }

    this.paneCounter++
    const paneId = `mock-pane-${this.paneCounter}`
    // Inherit parent window
    this.panes[paneId] = { parent: this.panes[targetId].parent }

    return paneId
  }

  async runCommand(paneId: string, command: string): Promise<void> {
    if (!this.panes[paneId]) {
      throw new Error(`Pane ${paneId} does not exist`)
    }

    this.executedCommands.push({ paneId, command })
  }

  async focus(paneId: string): Promise<void> {
    if (!this.panes[paneId]) {
      throw new Error(`Pane ${paneId} does not exist`)
    }
    // No-op for mock
  }
}
