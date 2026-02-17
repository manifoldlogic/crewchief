/**
 * Tests for SDK agent spawner
 *
 * NOTE: These tests require Claude Code to be installed and configured
 * with valid API credentials. They are skipped in CI environments.
 */

import { mkdtemp, rm, readFile } from 'fs/promises'
import { tmpdir } from 'os'
import { join } from 'path'
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { updateSDKConfig, resetSDKConfig } from '../../src/sdk/config.js'
import { spawnAgent } from '../../src/sdk/spawner.js'
import type { ToolUseEvent, AgentResult } from '../../src/sdk/types.js'

// Requires Claude Code with valid API credentials — excluded from default vitest config.
// Run via: pnpm test:integration

describe('SDK Agent Spawner', () => {
  let testWorktree: string

  beforeEach(async () => {
    // Create temporary worktree for testing
    testWorktree = await mkdtemp(join(tmpdir(), 'sdk-test-'))

    // Configure SDK for testing
    updateSDKConfig({
      verbose: false,
      defaultPermissionMode: 'bypassPermissions', // Bypass prompts for testing
    })
  })

  afterEach(async () => {
    // Clean up test worktree
    await rm(testWorktree, { recursive: true, force: true })

    // Reset SDK config
    resetSDKConfig()
  })

  it('should spawn agent and execute simple task', async () => {
    const result: AgentResult = await spawnAgent({
      task: 'Use the Bash tool to run: echo "Hello from SDK test" > test.txt',
      worktreePath: testWorktree,
      permissionMode: 'bypassPermissions',
    })

    // Verify agent completed successfully
    expect(result.success).toBe(true)
    expect(result.sessionId).toBeTruthy()
    expect(result.messages.length).toBeGreaterThan(0)

    // Verify file was created
    const testFilePath = join(testWorktree, 'test.txt')
    const content = await readFile(testFilePath, 'utf-8')
    expect(content).toContain('Hello from SDK test')
  }, 60000) // 60 second timeout for agent execution

  it('should capture tool use events', async () => {
    const toolUseEvents: ToolUseEvent[] = []

    const result = await spawnAgent({
      task: 'Use the Bash tool to echo "test message"',
      worktreePath: testWorktree,
      permissionMode: 'bypassPermissions',
      hooks: {
        onToolUse: (event) => {
          toolUseEvents.push(event)
        },
      },
    })

    // Verify agent succeeded
    expect(result.success).toBe(true)

    // Verify at least one tool was used
    expect(toolUseEvents.length).toBeGreaterThan(0)

    // Verify event structure
    const firstEvent = toolUseEvents[0]
    expect(firstEvent).toHaveProperty('session_id')
    expect(firstEvent).toHaveProperty('tool_name')
    expect(firstEvent).toHaveProperty('tool_input')
    expect(firstEvent).toHaveProperty('timestamp')
  }, 60000)

  it('should track usage and performance', async () => {
    const result = await spawnAgent({
      task: 'Echo "test"',
      worktreePath: testWorktree,
      permissionMode: 'bypassPermissions',
    })

    expect(result.success).toBe(true)

    // Verify usage tracking
    if (result.usage) {
      expect(result.usage.inputTokens).toBeGreaterThanOrEqual(0)
      expect(result.usage.outputTokens).toBeGreaterThanOrEqual(0)
    }

    // Verify performance tracking
    expect(result.performance).toBeDefined()
    expect(result.performance!.durationMs).toBeGreaterThan(0)
  }, 60000)

  it('should call onComplete hook', async () => {
    let completeCalled = false
    let completedResult: AgentResult | undefined

    await spawnAgent({
      task: 'Echo "complete test"',
      worktreePath: testWorktree,
      permissionMode: 'bypassPermissions',
      hooks: {
        onComplete: (result) => {
          completeCalled = true
          completedResult = result
        },
      },
    })

    expect(completeCalled).toBe(true)
    expect(completedResult).toBeDefined()
    expect(completedResult!.success).toBe(true)
  }, 60000)

  it('should respect permission mode setting', async () => {
    const result = await spawnAgent({
      task: 'Echo "permission test"',
      worktreePath: testWorktree,
      permissionMode: 'bypassPermissions',
    })

    // With bypassPermissions, agent should complete without user interaction
    expect(result.success).toBe(true)
  }, 60000)
})
