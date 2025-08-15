export interface AgentType {
  id: string
  name: string
  platform: 'claude' | 'gemini' | 'custom'
  capabilities: string[]
  agentDefinitionPath: string
  executionCommand: string
  environmentVars?: Record<string, string>
}

export interface AgentRun {
  agentId: string
  type: AgentType
  paneId: string
  worktreePath: string
  startedAt: Date
  status: 'running' | 'closed' | 'failed'
}
