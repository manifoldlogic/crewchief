import { spawnSync, spawn, ChildProcess } from 'node:child_process'
import { existsSync } from 'node:fs'
import type {
  ITermAgentInfo,
  ITermRpcRequest,
  ITermRpcResponse,
  ITermBridgeConfig,
  ITermCreateAgentParams,
  ITermSendTaskParams,
  ITermGetOutputParams,
  ITermSendCommandParams,
  ITermSplitPaneParams,
  ITermSetBadgeParams,
  ITermBroadcastParams,
  ITermCreateGridParams,
} from './iterm.types.js'

export type PaneLayout = 'horizontal' | 'vertical'

export class ITermService {
  private sessionName: string
  private bridgeConfig: ITermBridgeConfig
  private bridgeProcess?: ChildProcess
  private rpcId: number = 1
  private bridgeStarted: boolean = false

  constructor(sessionName: string, bridgeConfig?: Partial<ITermBridgeConfig>) {
    this.sessionName = sessionName
    this.bridgeConfig = {
      host: bridgeConfig?.host ?? 'localhost',
      port: bridgeConfig?.port ?? 8765,
      timeout: bridgeConfig?.timeout ?? 5000,
    }
  }

  /**
   * Check if iTerm2 is available on the system
   */
  ensureITerm(): void {
    const res = spawnSync('osascript', ['-e', 'tell application "System Events" to name of every application process'])
    const apps = res.stdout.toString()
    if (!apps.includes('iTerm')) {
      throw new Error('iTerm2 not found. Please install iTerm2.')
    }
  }

  /**
   * Start the Python bridge server
   */
  async startBridge(): Promise<void> {
    if (this.bridgeStarted) return

    this.ensureITerm()

    // Find the bridge script
    const bridgePath = new URL('../../../../scripts/iterm_scripts/iterm_bridge.py', import.meta.url).pathname

    if (!existsSync(bridgePath)) {
      throw new Error(`Bridge script not found at ${bridgePath}`)
    }

    // Start the bridge process
    this.bridgeProcess = spawn('python3', [bridgePath, '--port', String(this.bridgeConfig.port)], {
      detached: false,
      stdio: 'pipe',
    })

    this.bridgeProcess.on('error', (err) => {
      console.error('Bridge process error:', err)
      this.bridgeStarted = false
    })

    this.bridgeProcess.on('exit', (code) => {
      console.log(`Bridge process exited with code ${code}`)
      this.bridgeStarted = false
    })

    // Wait for bridge to be ready
    await this.waitForBridge()
    this.bridgeStarted = true
  }

  /**
   * Wait for the bridge to be ready
   */
  private async waitForBridge(maxAttempts = 30): Promise<void> {
    for (let i = 0; i < maxAttempts; i++) {
      try {
        const response = await fetch(`http://${this.bridgeConfig.host}:${this.bridgeConfig.port}/health`)
        if (response.ok) {
          const data = await response.json()
          if (data.status === 'healthy') {
            return
          }
        }
      } catch {
        // Bridge not ready yet
      }
      await new Promise((resolve) => setTimeout(resolve, 1000))
    }
    throw new Error('Bridge failed to start')
  }

  /**
   * Stop the bridge server
   */
  stopBridge(): void {
    if (this.bridgeProcess) {
      this.bridgeProcess.kill()
      this.bridgeProcess = undefined
      this.bridgeStarted = false
    }
  }

  /**
   * Send an RPC request to the bridge
   */
  private async sendRpc(method: string, params?: Record<string, any>): Promise<any> {
    if (!this.bridgeStarted) {
      await this.startBridge()
    }

    const request: ITermRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id: this.rpcId++,
    }

    const response = await fetch(`http://${this.bridgeConfig.host}:${this.bridgeConfig.port}/rpc`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
      timeout: this.bridgeConfig.timeout,
    })

    const data = (await response.json()) as ITermRpcResponse

    if (data.error) {
      throw new Error(`RPC Error: ${data.error.message}`)
    }

    return data.result
  }

  /**
   * Start a new iTerm2 session
   */
  async startSession(): Promise<void> {
    await this.sendRpc('createSession', { name: this.sessionName })
  }

  /**
   * Check if we're inside iTerm2
   */
  isInsideITerm(): boolean {
    return process.env.TERM_PROGRAM === 'iTerm.app'
  }

  /**
   * Focus the iTerm2 session
   */
  async attach(): Promise<void> {
    // Use AppleScript to focus iTerm2
    spawnSync('osascript', ['-e', 'tell application "iTerm2" to activate'])
  }

  /**
   * Create a new pane
   */
  async createPane(layout: PaneLayout = 'vertical'): Promise<string> {
    const sessions = await this.sendRpc('listSessions')
    if (sessions.length === 0) {
      await this.startSession()
    }

    const currentSession = sessions[0]
    const newSessionId = await this.sendRpc('splitPane', {
      sessionId: currentSession.sessionId,
      vertical: layout === 'vertical',
    } as ITermSplitPaneParams)

    return newSessionId
  }

  /**
   * Check if session exists
   */
  async hasSession(): Promise<boolean> {
    try {
      const sessions = await this.sendRpc('listSessions')
      return sessions.length > 0
    } catch {
      return false
    }
  }

  /**
   * Ensure session exists
   */
  async ensureSession(): Promise<void> {
    if (!(await this.hasSession())) {
      await this.startSession()
    }
  }

  /**
   * Send keys to a pane (legacy compatibility)
   */
  async sendKeys(paneId: string, command: string): Promise<void> {
    // Clear line and send command
    await this.sendRpc('sendText', {
      sessionId: paneId,
      text: '\x03', // Ctrl+C
    } as ITermSendCommandParams)
    await this.sendRpc('sendText', {
      sessionId: paneId,
      text: '\x15', // Ctrl+U
    } as ITermSendCommandParams)
    await this.sendRpc('sendCommand', {
      sessionId: paneId,
      command,
    } as ITermSendCommandParams)
  }

  /**
   * Send a line to a pane
   */
  async sendLine(paneId: string, text: string): Promise<void> {
    await this.sendRpc('sendCommand', {
      sessionId: paneId,
      command: text,
    } as ITermSendCommandParams)
  }

  /**
   * Close a pane
   */
  async closePane(paneId: string): Promise<void> {
    await this.sendRpc('closeSession', { sessionId: paneId })
  }

  /**
   * Capture output from a pane
   */
  async captureOutput(paneId: string): Promise<string> {
    return await this.sendRpc('getContents', {
      sessionId: paneId,
      lines: 1000,
    })
  }

  /**
   * Create a new window
   */
  async createWindow(): Promise<string> {
    const result = await this.sendRpc('createSession')
    return result.sessionId
  }

  /**
   * Create a window with a command
   */
  async createWindowWithCommand(command: string, cwd?: string): Promise<string> {
    const sessionId = await this.createWindow()
    if (cwd) {
      await this.sendRpc('sendCommand', {
        sessionId,
        command: `cd ${cwd}`,
      } as ITermSendCommandParams)
    }
    await this.sendRpc('sendCommand', {
      sessionId,
      command,
    } as ITermSendCommandParams)
    return sessionId
  }

  /**
   * Create a window with working directory
   */
  async createWindowWithCwd(cwd?: string): Promise<{ windowId: string; paneId: string }> {
    const sessionInfo = await this.sendRpc('createSession')
    if (cwd) {
      await this.sendRpc('sendCommand', {
        sessionId: sessionInfo.sessionId,
        command: `cd ${cwd}`,
      } as ITermSendCommandParams)
    }
    return {
      windowId: sessionInfo.windowId,
      paneId: sessionInfo.sessionId,
    }
  }

  /**
   * Create a named window
   */
  async createNamedWindow(name: string, cwd?: string): Promise<{ windowId: string; paneId: string; target: string }> {
    const result = await this.createWindowWithCwd(cwd)
    await this.sendRpc('setBadge', {
      sessionId: result.paneId,
      badge: name,
    } as ITermSetBadgeParams)
    return {
      ...result,
      target: `${this.sessionName}:${name}`,
    }
  }

  /**
   * Get any available pane ID
   */
  async getAnyPaneId(): Promise<string> {
    const sessions = await this.sendRpc('listSessions')
    if (sessions.length === 0) {
      await this.startSession()
      const newSessions = await this.sendRpc('listSessions')
      return newSessions[0].sessionId
    }
    return sessions[0].sessionId
  }

  /**
   * Wait for a pane to be ready
   */
  async waitForReady(paneId: string, timeoutMs = 5000): Promise<boolean> {
    const marker = `CC_READY_${Math.random().toString(36).slice(2, 8)}`
    const deadline = Date.now() + timeoutMs

    while (Date.now() < deadline) {
      await this.sendLine(paneId, `echo ${marker}`)
      const output = await this.captureOutput(paneId)
      if (output.includes(marker)) return true
      await new Promise((r) => setTimeout(r, 200))
    }
    return false
  }

  // Agent-specific methods

  /**
   * Create an agent workspace
   */
  async createAgent(params: ITermCreateAgentParams): Promise<ITermAgentInfo> {
    return await this.sendRpc('createAgent', params)
  }

  /**
   * Send a task to an agent
   */
  async sendTask(params: ITermSendTaskParams): Promise<void> {
    await this.sendRpc('sendTask', params)
  }

  /**
   * Get agent output
   */
  async getAgentOutput(params: ITermGetOutputParams): Promise<string> {
    return await this.sendRpc('getAgentOutput', params)
  }

  /**
   * Stop an agent
   */
  async stopAgent(agentId: string): Promise<void> {
    await this.sendRpc('stopAgent', { agentId })
  }

  /**
   * List all agents
   */
  async listAgents(): Promise<ITermAgentInfo[]> {
    return await this.sendRpc('listAgents')
  }

  /**
   * Get agent status
   */
  async getAgentStatus(agentId: string): Promise<ITermAgentInfo> {
    return await this.sendRpc('getAgentStatus', { agentId })
  }

  /**
   * Create a grid layout of agents
   */
  async createAgentGrid(params: ITermCreateGridParams): Promise<string> {
    return await this.sendRpc('createAgentGrid', params)
  }

  /**
   * Broadcast command to multiple agents
   */
  async broadcast(params: ITermBroadcastParams): Promise<void> {
    await this.sendRpc('broadcast', params)
  }

  /**
   * Set a badge on a session
   */
  async setBadge(sessionId: string, badge: string): Promise<void> {
    await this.sendRpc('setBadge', {
      sessionId,
      badge,
    } as ITermSetBadgeParams)
  }

  /**
   * Focus a specific session
   */
  async focusSession(sessionId: string): Promise<void> {
    await this.sendRpc('focusSession', { sessionId })
  }
}
