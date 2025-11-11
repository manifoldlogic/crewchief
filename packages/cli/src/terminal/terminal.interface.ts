/**
 * Common interface for terminal orchestration backends (tmux, iTerm2, etc.)
 */

export type TerminalLayout = 'horizontal' | 'vertical'

export interface TerminalSessionInfo {
  sessionId: string
  windowId?: string
  name?: string
}

export interface TerminalAgentInfo {
  agentId: string
  sessionId: string
  status: 'running' | 'idle' | 'stopped'
  workingDir: string
}

export interface ITerminalService {
  /**
   * Ensure the terminal backend is available
   */
  ensureBackend(): void | Promise<void>

  /**
   * Start a new session
   */
  startSession(): void | Promise<void>

  /**
   * Check if we're inside the terminal environment
   */
  isInsideTerminal(): boolean

  /**
   * Attach to or focus the session
   */
  attach(): void | Promise<void>

  /**
   * Create a new pane
   */
  createPane(layout?: TerminalLayout): string | Promise<string>

  /**
   * Check if session exists
   */
  hasSession(): boolean | Promise<boolean>

  /**
   * Ensure session exists
   */
  ensureSession(): void | Promise<void>

  /**
   * Send keys/commands to a pane
   */
  sendKeys(paneId: string, command: string): void | Promise<void>

  /**
   * Send a line of text
   */
  sendLine(paneId: string, text: string): void | Promise<void>

  /**
   * Close a pane
   */
  closePane(paneId: string): void | Promise<void>

  /**
   * Capture output from a pane
   */
  captureOutput(paneId: string): string | Promise<string>

  /**
   * Create a new window
   */
  createWindow(): string | Promise<string>

  /**
   * Create a window with a command
   */
  createWindowWithCommand(command: string, cwd?: string): string | Promise<string>

  /**
   * Create a window with working directory
   */
  createWindowWithCwd(
    cwd?: string,
  ): { windowId: string; paneId: string } | Promise<{ windowId: string; paneId: string }>

  /**
   * Create a named window
   */
  createNamedWindow(
    name: string,
    cwd?: string,
  ):
    | { windowId: string; paneId: string; target: string }
    | Promise<{ windowId: string; paneId: string; target: string }>

  /**
   * Get any available pane ID
   */
  getAnyPaneId(): string | Promise<string>

  /**
   * Wait for a pane to be ready
   */
  waitForReady(paneId: string, timeoutMs?: number): Promise<boolean>

  /**
   * Clean up resources if needed
   */
  cleanup?(): void | Promise<void>
}

export interface IAgentTerminalService extends ITerminalService {
  /**
   * Create an agent workspace
   */
  createAgent(agentId: string, agentType?: string, workingDir?: string): Promise<TerminalAgentInfo>

  /**
   * Send a task to an agent
   */
  sendTask(agentId: string, task: Record<string, any>): Promise<void>

  /**
   * Get agent output
   */
  getAgentOutput(agentId: string, lines?: number): Promise<string>

  /**
   * Stop an agent
   */
  stopAgent(agentId: string): Promise<void>

  /**
   * List all agents
   */
  listAgents(): Promise<TerminalAgentInfo[]>

  /**
   * Get agent status
   */
  getAgentStatus(agentId: string): Promise<TerminalAgentInfo>

  /**
   * Create a grid layout of agents
   */
  createAgentGrid(agentIds: string[], rows?: number, cols?: number): Promise<string>

  /**
   * Broadcast command to multiple agents
   */
  broadcast(agentIds: string[], command: string): Promise<void>
}
