export interface AgentMessage {
  type: 'instruction' | 'result' | 'status' | 'completion' | 'error' | 'conflict-alert' | 'file-change'
  from: 'orchestrator' | string
  to: 'orchestrator' | string
  payload: unknown
  timestamp: Date
  runId?: string
  worktreeContext?: {
    branch: string
    modifiedFiles: string[]
    lastCommit: string
  }
}

/**
 * Status message payload - reports agent activity and progress
 */
export interface StatusPayload {
  activity: string
  progress?: number // 0-100
}

/**
 * Completion message payload - signals agent task completion
 */
export interface CompletionPayload {
  summary: string
  exitCode?: number
  artifacts?: string[]
}

/**
 * Error message payload - reports agent errors
 */
export interface ErrorPayload {
  message: string
  recoverable: boolean
  code?: string
  stack?: string
}

/**
 * Conflict alert payload - notifies about file conflicts between agents
 */
export interface ConflictAlertPayload {
  sourceAgent: string
  affectedAgents: string[]
  conflictFiles: string[]
  severity: 'warning' | 'critical'
}

/**
 * File change payload - tracks file modifications by an agent
 */
export interface FileChangePayload {
  files: Array<{
    path: string
    status: 'added' | 'modified' | 'deleted'
  }>
}
