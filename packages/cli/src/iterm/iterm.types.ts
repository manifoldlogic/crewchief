/**
 * Type definitions for iTerm2 integration
 */

export interface ITermSessionInfo {
  sessionId: string
  tabId: string
  windowId: string
  name?: string
  profile?: string
}

export interface ITermAgentInfo {
  agentId: string
  sessionId: string
  status: 'running' | 'idle' | 'stopped'
  workingDir: string
  createdAt?: string
}

export interface ITermRpcRequest {
  jsonrpc: '2.0'
  method: string
  params?: Record<string, any>
  id: string | number
}

export interface ITermRpcResponse {
  jsonrpc: '2.0'
  result?: any
  error?: {
    code: number
    message: string
    data?: any
  }
  id: string | number
}

export interface ITermBridgeConfig {
  host: string
  port: number
  timeout?: number
}

export type ITermLayout = 'horizontal' | 'vertical' | 'grid'

export interface ITermGridConfig {
  rows: number
  cols: number
}

export interface ITermCreateAgentParams {
  agentId: string
  agentType?: string
  workingDir?: string
}

export interface ITermSendTaskParams {
  agentId: string
  task: Record<string, any>
}

export interface ITermGetOutputParams {
  agentId: string
  lines?: number
}

export interface ITermSendCommandParams {
  sessionId: string
  command: string
}

export interface ITermSplitPaneParams {
  sessionId: string
  vertical?: boolean
  before?: boolean
}

export interface ITermSetBadgeParams {
  sessionId: string
  badge: string
}

export interface ITermBroadcastParams {
  agentIds: string[]
  command: string
}

export interface ITermCreateGridParams {
  agentIds: string[]
  rows?: number
  cols?: number
}