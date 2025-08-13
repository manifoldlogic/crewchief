export interface AgentMessage {
  type: 'instruction' | 'result' | 'status' | 'error'
  from: 'orchestrator' | string
  to: 'orchestrator' | string
  payload: unknown
  timestamp: Date
  worktreeContext?: {
    branch: string
    modifiedFiles: string[]
    lastCommit: string
  }
}
