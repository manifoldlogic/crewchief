/**
 * iTerm2 adapter for the terminal interface
 */

import type {
  IAgentTerminalService,
  TerminalLayout,
  TerminalAgentInfo,
} from './terminal.interface.js'
import { ITermService } from '../iterm/iterm.service.js'

export class ITermAdapter implements IAgentTerminalService {
  private iterm: ITermService

  constructor(sessionName: string) {
    this.iterm = new ITermService(sessionName)
  }

  async ensureBackend(): Promise<void> {
    this.iterm.ensureITerm()
    await this.iterm.startBridge()
  }

  async startSession(): Promise<void> {
    await this.iterm.startSession()
  }

  isInsideTerminal(): boolean {
    return this.iterm.isInsideITerm()
  }

  async attach(): Promise<void> {
    await this.iterm.attach()
  }

  async createPane(layout?: TerminalLayout): Promise<string> {
    return await this.iterm.createPane(layout)
  }

  async hasSession(): Promise<boolean> {
    return await this.iterm.hasSession()
  }

  async ensureSession(): Promise<void> {
    await this.iterm.ensureSession()
  }

  async sendKeys(paneId: string, command: string): Promise<void> {
    await this.iterm.sendKeys(paneId, command)
  }

  async sendLine(paneId: string, text: string): Promise<void> {
    await this.iterm.sendLine(paneId, text)
  }

  async closePane(paneId: string): Promise<void> {
    await this.iterm.closePane(paneId)
  }

  async captureOutput(paneId: string): Promise<string> {
    return await this.iterm.captureOutput(paneId)
  }

  async createWindow(): Promise<string> {
    return await this.iterm.createWindow()
  }

  async createWindowWithCommand(command: string, cwd?: string): Promise<string> {
    return await this.iterm.createWindowWithCommand(command, cwd)
  }

  async createWindowWithCwd(cwd?: string): Promise<{ windowId: string; paneId: string }> {
    return await this.iterm.createWindowWithCwd(cwd)
  }

  async createNamedWindow(
    name: string,
    cwd?: string,
  ): Promise<{ windowId: string; paneId: string; target: string }> {
    return await this.iterm.createNamedWindow(name, cwd)
  }

  async getAnyPaneId(): Promise<string> {
    return await this.iterm.getAnyPaneId()
  }

  async waitForReady(paneId: string, timeoutMs?: number): Promise<boolean> {
    return await this.iterm.waitForReady(paneId, timeoutMs)
  }

  async cleanup(): Promise<void> {
    this.iterm.stopBridge()
  }

  // Agent-specific methods

  async createAgent(agentId: string, agentType?: string, workingDir?: string): Promise<TerminalAgentInfo> {
    const result = await this.iterm.createAgent({
      agentId,
      agentType,
      workingDir,
    })
    return {
      agentId: result.agentId,
      sessionId: result.sessionId,
      status: result.status,
      workingDir: result.workingDir,
    }
  }

  async sendTask(agentId: string, task: Record<string, any>): Promise<void> {
    await this.iterm.sendTask({ agentId, task })
  }

  async getAgentOutput(agentId: string, lines?: number): Promise<string> {
    return await this.iterm.getAgentOutput({ agentId, lines })
  }

  async stopAgent(agentId: string): Promise<void> {
    await this.iterm.stopAgent(agentId)
  }

  async listAgents(): Promise<TerminalAgentInfo[]> {
    const agents = await this.iterm.listAgents()
    return agents.map((agent) => ({
      agentId: agent.agentId,
      sessionId: agent.sessionId,
      status: agent.status,
      workingDir: agent.workingDir,
    }))
  }

  async getAgentStatus(agentId: string): Promise<TerminalAgentInfo> {
    const agent = await this.iterm.getAgentStatus(agentId)
    return {
      agentId: agent.agentId,
      sessionId: agent.sessionId,
      status: agent.status,
      workingDir: agent.workingDir,
    }
  }

  async createAgentGrid(agentIds: string[], rows?: number, cols?: number): Promise<string> {
    return await this.iterm.createAgentGrid({
      agentIds,
      rows,
      cols,
    })
  }

  async broadcast(agentIds: string[], command: string): Promise<void> {
    await this.iterm.broadcast({ agentIds, command })
  }
}
