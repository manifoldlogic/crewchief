export interface WindowOptions {
  title?: string
  profile?: string
  workingDirectory?: string
}

export type SplitDirection = 'vertical' | 'horizontal'

/**
 * Information about a spawned agent
 */
export interface AgentInfo {
  /** Pane ID or process ID */
  id: string
  /** Human-readable name (typically task__type format) */
  name: string
  /** Agent type: claude, gemini, codex, etc. */
  type: string
  /** Current status */
  status: 'running' | 'stopped'
}

export interface TerminalProvider {
  /**
   * Unique identifier for the provider implementation
   * e.g. 'iterm', 'headless', 'mock'
   */
  readonly id: string

  /**
   * Initialize the provider.
   * Should check for environment compatibility and set up any necessary resources.
   */
  initialize(): Promise<void>

  /**
   * Clean up resources.
   * For headless: kill child processes.
   * For iTerm: clean up temp files/connections.
   */
  dispose(): Promise<void>

  /**
   * Create a new window.
   * Returns the window identifier.
   */
  createWindow(options?: WindowOptions): Promise<string>

  /**
   * Create a new tab in an existing window.
   * Returns the tab identifier (or pane ID if tabs are treated as panes).
   */
  createTab(windowId: string): Promise<string>

  /**
   * Split a pane in a given direction.
   * Returns the new pane identifier.
   */
  splitPane(targetId: string, direction: SplitDirection): Promise<string>

  /**
   * Execute a command in a specific pane.
   * The promise resolves when the command is *sent*, not necessarily when it completes.
   */
  runCommand(paneId: string, command: string): Promise<void>

  /**
   * Focus a specific pane/window.
   */
  focus(paneId: string): Promise<void>

  // Optional messaging methods (Phase 3 - ITERMCLN)

  /**
   * Send a message to an agent in a specific pane.
   * @param paneId - The pane/process identifier
   * @param message - The message text to send
   * @returns true if message was sent successfully, false otherwise
   */
  sendMessage?(paneId: string, message: string): Promise<boolean>

  /**
   * List all agents managed by this provider.
   * @returns Array of agent information objects
   */
  listAgents?(): Promise<AgentInfo[]>
}
