/**
 * Agent spawning using Claude Code Agents SDK
 */

import { query, type Options, type Query, type SDKMessage } from '@anthropic-ai/claude-agent-sdk'
import { getSDKConfig } from './config.js'
import type { AgentSpawnOptions, AgentResult, ToolUseEvent, ToolResultEvent } from './types.js'

/**
 * Spawn an agent using the Claude Code Agents SDK
 *
 * @param options - Agent spawn options
 * @returns Agent execution result
 *
 * @example
 * ```typescript
 * const result = await spawnAgent({
 *   task: 'Create a file test.txt with "Hello World"',
 *   worktreePath: '/path/to/worktree',
 *   hooks: {
 *     onToolUse: (event) => console.log('Tool used:', event.tool_name),
 *   },
 * })
 * ```
 */
export async function spawnAgent(options: AgentSpawnOptions): Promise<AgentResult> {
  const config = getSDKConfig()
  const startTime = Date.now()

  // Build SDK options
  const sdkOptions: Options = {
    cwd: options.worktreePath,
    permissionMode: options.permissionMode || config.defaultPermissionMode,
    model: options.model || config.defaultModel,
    maxThinkingTokens: options.maxThinkingTokens,
    maxTurns: options.maxTurns,
    env: options.env,
  }

  // Add system prompt if provided
  if (options.systemPrompt) {
    sdkOptions.systemPrompt = options.systemPrompt
  }

  // Add tool restrictions if provided
  if (options.allowedTools) {
    sdkOptions.allowedTools = options.allowedTools
  }
  if (options.disallowedTools) {
    sdkOptions.disallowedTools = options.disallowedTools
  }

  // Set up hooks
  if (options.hooks) {
    sdkOptions.hooks = buildSDKHooks(options.hooks)
  }

  if (config.verbose) {
    console.log('[SDK] Spawning agent with options:', {
      task: options.task.substring(0, 100),
      cwd: options.worktreePath,
      permissionMode: sdkOptions.permissionMode,
    })
  }

  // Spawn agent
  const agentQuery: Query = query({
    prompt: options.task,
    options: sdkOptions,
  })

  // Collect messages
  const messages: SDKMessage[] = []
  let finalMessage: SDKMessage | undefined
  let sessionId = ''
  let transcriptPath: string | undefined

  // Process messages from agent
  for await (const message of agentQuery) {
    messages.push(message)

    // Extract session info from first message
    if (!sessionId && 'session_id' in message) {
      sessionId = message.session_id
    }
    if (!transcriptPath && 'transcript_path' in message) {
      transcriptPath = message.transcript_path
    }

    // Track final result message
    if (message.type === 'result') {
      finalMessage = message
    }
  }

  const endTime = Date.now()

  // Build result
  const result: AgentResult = {
    success: finalMessage?.type === 'result',
    finalMessage,
    messages,
    sessionId,
    transcriptPath,
  }

  // Extract usage and performance from final message
  if (finalMessage && finalMessage.type === 'result') {
    const resultMessage = finalMessage as any

    if (resultMessage.usage) {
      result.usage = {
        inputTokens: resultMessage.usage.input_tokens || 0,
        outputTokens: resultMessage.usage.output_tokens || 0,
        cacheCreationTokens: resultMessage.usage.cache_creation_input_tokens,
        cacheReadTokens: resultMessage.usage.cache_read_input_tokens,
        totalCostUsd: resultMessage.usage.total_cost_usd,
      }
    }

    if (resultMessage.duration_ms) {
      result.performance = {
        durationMs: resultMessage.duration_ms,
        durationApiMs: resultMessage.duration_api_ms || 0,
        numTurns: resultMessage.num_turns || 0,
      }
    }
  }

  // If no performance data, use wall clock time
  if (!result.performance) {
    result.performance = {
      durationMs: endTime - startTime,
      durationApiMs: 0,
      numTurns: 0,
    }
  }

  // Call onComplete hook
  if (options.hooks?.onComplete) {
    await options.hooks.onComplete(result)
  }

  if (config.verbose) {
    console.log('[SDK] Agent completed:', {
      success: result.success,
      sessionId: result.sessionId,
      durationMs: result.performance?.durationMs,
      inputTokens: result.usage?.inputTokens,
      outputTokens: result.usage?.outputTokens,
    })
  }

  return result
}

/**
 * Build SDK hooks from agent hooks
 */
function buildSDKHooks(agentHooks: AgentSpawnOptions['hooks']): Options['hooks'] {
  const hooks: Options['hooks'] = {}

  // PreToolUse hook
  if (agentHooks?.onToolUse) {
    hooks.PreToolUse = [
      {
        hooks: [
          async (input: any, _toolUseID: string | undefined, _options: any) => {
            const event: ToolUseEvent = {
              session_id: input.session_id,
              tool_name: input.tool_name,
              tool_input: input.tool_input,
              timestamp: Date.now(),
            }
            await agentHooks.onToolUse!(event)
            return { continue: true }
          },
        ],
      },
    ]
  }

  // PostToolUse hook
  if (agentHooks?.onToolResult) {
    hooks.PostToolUse = [
      {
        hooks: [
          async (input: any, _toolUseID: string | undefined, _options: any) => {
            const event: ToolResultEvent = {
              session_id: input.session_id,
              tool_name: input.tool_name,
              tool_input: input.tool_input,
              tool_result: input.tool_response,
              success: !input.is_error,
              timestamp: Date.now(),
            }
            await agentHooks.onToolResult!(event)
            return { continue: true }
          },
        ],
      },
    ]
  }

  // SessionStart hook
  if (agentHooks?.onSessionStart) {
    hooks.SessionStart = [
      {
        hooks: [
          async (input: any, _toolUseID: string | undefined, _options: any) => {
            await agentHooks.onSessionStart!(input.session_id)
            return { continue: true }
          },
        ],
      },
    ]
  }

  // SessionEnd hook
  if (agentHooks?.onSessionEnd) {
    hooks.SessionEnd = [
      {
        hooks: [
          async (input: any, _toolUseID: string | undefined, _options: any) => {
            await agentHooks.onSessionEnd!(input.session_id)
            return { continue: true }
          },
        ],
      },
    ]
  }

  return hooks
}
