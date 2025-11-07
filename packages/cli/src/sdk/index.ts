/**
 * Claude Code Agents SDK Integration
 *
 * This module provides a simplified interface for using the Claude Code Agents SDK
 * within the crewchief CLI. It wraps the SDK's query() function and provides:
 * - Type-safe agent spawning
 * - Lifecycle hooks (tool use, session events)
 * - Usage and performance tracking
 * - Configuration management
 *
 * @packageDocumentation
 */

export { spawnAgent } from './spawner.js'
export { getSDKConfig, updateSDKConfig, resetSDKConfig } from './config.js'
export type { AgentSpawnOptions, AgentResult, AgentHooks, ToolUseEvent, ToolResultEvent, SDKConfig } from './types.js'
