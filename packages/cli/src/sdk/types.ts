/**
 * TypeScript interfaces for Claude Code Agents SDK integration
 */

import type { SDKMessage } from '@anthropic-ai/claude-agent-sdk'

/**
 * Event fired when an agent uses a tool
 */
export interface ToolUseEvent {
  session_id: string
  tool_name: string
  tool_input: Record<string, any>
  timestamp: number
}

/**
 * Event fired when a tool use completes
 */
export interface ToolResultEvent extends ToolUseEvent {
  tool_result: any
  success: boolean
  duration_ms?: number
}

/**
 * Agent execution result
 */
export interface AgentResult {
  success: boolean
  finalMessage?: SDKMessage
  messages: SDKMessage[]
  sessionId: string
  transcriptPath?: string
  usage?: {
    inputTokens: number
    outputTokens: number
    cacheCreationTokens?: number
    cacheReadTokens?: number
    totalCostUsd?: number
  }
  performance?: {
    durationMs: number
    durationApiMs: number
    numTurns: number
  }
}

/**
 * Hooks for agent lifecycle events
 */
export interface AgentHooks {
  onToolUse?: (event: ToolUseEvent) => void | Promise<void>
  onToolResult?: (event: ToolResultEvent) => void | Promise<void>
  onSessionStart?: (sessionId: string) => void | Promise<void>
  onSessionEnd?: (sessionId: string) => void | Promise<void>
  onComplete?: (result: AgentResult) => void | Promise<void>
}

/**
 * Options for spawning an agent
 */
export interface AgentSpawnOptions {
  /** Task/prompt for the agent to execute */
  task: string

  /** Working directory (worktree path) */
  worktreePath: string

  /** Lifecycle hooks */
  hooks?: AgentHooks

  /** Permission mode */
  permissionMode?: 'default' | 'acceptEdits' | 'bypassPermissions' | 'plan'

  /** Model override (default: uses Claude Code default) */
  model?: string

  /** Maximum thinking tokens */
  maxThinkingTokens?: number

  /** Maximum turns */
  maxTurns?: number

  /** Environment variables */
  env?: Record<string, string>

  /** Custom system prompt */
  systemPrompt?: string

  /** Allowed tools (undefined = all tools allowed) */
  allowedTools?: string[]

  /** Disallowed tools */
  disallowedTools?: string[]
}

/**
 * SDK configuration
 */
export interface SDKConfig {
  /** Default permission mode for agents */
  defaultPermissionMode: 'default' | 'acceptEdits' | 'bypassPermissions' | 'plan'

  /** Default model */
  defaultModel?: string

  /** Default working directory */
  defaultCwd?: string

  /** Global hooks (applied to all agents) */
  globalHooks?: AgentHooks

  /** Enable verbose logging */
  verbose?: boolean
}
