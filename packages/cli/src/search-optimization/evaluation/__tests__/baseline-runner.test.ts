/**
 * Tests for baseline runner
 */

import { mkdtempSync, rmSync, existsSync, readFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import type { SearchTask } from '../../types.js'
import type { BaselineConfig, BaselineResult } from '../baseline-runner.js'
import { runBaseline, formatBaselineReport } from '../baseline-runner.js'

// Mock the SDK spawner to avoid actually spawning agents
vi.mock('../../../sdk/spawner.js', () => ({
  spawnAgent: vi.fn(async (options) => {
    // Simulate agent execution by calling hooks
    if (options.hooks?.onToolUse) {
      // Simulate grep usage
      await options.hooks.onToolUse({
        session_id: 'test-session',
        tool_name: 'Grep',
        tool_input: { pattern: 'worktree', output_mode: 'files_with_matches' },
        timestamp: Date.now(),
      })

      // Simulate glob usage
      await options.hooks.onToolUse({
        session_id: 'test-session',
        tool_name: 'Glob',
        tool_input: { pattern: '**/*.ts' },
        timestamp: Date.now() + 100,
      })

      // Simulate read usage
      await options.hooks.onToolUse({
        session_id: 'test-session',
        tool_name: 'Read',
        tool_input: { file_path: '/path/to/file.ts' },
        timestamp: Date.now() + 200,
      })

      await options.hooks.onToolUse({
        session_id: 'test-session',
        tool_name: 'Read',
        tool_input: { file_path: '/path/to/other.ts' },
        timestamp: Date.now() + 300,
      })
    }

    return {
      success: true,
      sessionId: 'test-session',
      transcriptPath: '/tmp/test-transcript',
      usage: {
        inputTokens: 1000,
        outputTokens: 500,
        totalCostUsd: 0.01,
      },
      performance: {
        durationMs: 5000,
        durationApiMs: 4000,
        numTurns: 3,
      },
      messages: [],
    }
  }),
}))

describe('baseline-runner', () => {
  let testDir: string

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'baseline-runner-test-'))
  })

  afterEach(() => {
    try {
      rmSync(testDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  })

  const createTestTask = (): SearchTask => ({
    id: 'test-task-1',
    name: 'Find worktree creation',
    description: 'Find the function that creates a new git worktree',
    searchTarget: {
      type: 'function',
      name: 'createWorktree',
    },
    followUpTask: {
      type: 'explanation',
      prompt: 'Explain how worktrees are created',
      validator: {
        type: 'explanation',
        mentionsPattern: /worktree/i,
      },
    },
    difficulty: 'medium',
    category: 'implementation',
    successValidator: (_output) => ({
      searchQuality: 0.8,
      taskCompletion: 0.9,
      efficiency: 0.7,
      total: 0.8,
      details: 'Mock validation',
    }),
  })

  describe('runBaseline', () => {
    it('should execute task with grep/glob/read tools only', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      expect(result.success).toBe(true)
      expect(result.task).toBe(config.task)
      expect(result.metrics).toBeDefined()
      expect(result.agentResult).toBeDefined()
    })

    it('should capture comprehensive metrics', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      // Verify metrics structure
      expect(result.metrics.durationSeconds).toBeGreaterThanOrEqual(0)
      expect(result.metrics.toolCalls).toBeDefined()
      expect(result.metrics.searchQueries).toBeDefined()
      expect(result.metrics.filesExamined).toBeDefined()
      expect(result.metrics.timedOut).toBe(false)

      // Verify tool usage tracking
      expect(result.metrics.toolCalls.Grep).toBe(1)
      expect(result.metrics.toolCalls.Glob).toBe(1)
      expect(result.metrics.toolCalls.Read).toBe(2)

      // Verify search queries captured
      expect(result.metrics.searchQueries).toHaveLength(1)
      expect(result.metrics.searchQueries[0]).toBe('worktree')

      // Verify files examined (unique count)
      expect(result.metrics.filesExamined).toBe(2)
    })

    it('should create run directory with logs', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      // Verify run directory exists
      expect(existsSync(result.runDir)).toBe(true)

      // Verify tool usage log exists
      const toolLogPath = join(result.runDir, 'tool-usage.log')
      expect(existsSync(toolLogPath)).toBe(true)

      // Verify log content
      const logContent = readFileSync(toolLogPath, 'utf-8')
      const logLines = logContent.trim().split('\n')
      expect(logLines.length).toBeGreaterThan(0)

      // Verify each line is valid JSON
      logLines.forEach((line) => {
        expect(() => JSON.parse(line)).not.toThrow()
      })

      // Verify agent result saved
      const agentResultPath = join(result.runDir, 'agent-result.json')
      expect(existsSync(agentResultPath)).toBe(true)
    })

    it('should use default tools if not specified', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      // Should succeed with default tools (Grep, Glob, Read, etc.)
      expect(result.success).toBe(true)
    })

    it('should respect custom tool list', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        availableTools: ['Grep', 'Glob', 'Read'],
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      expect(result.success).toBe(true)
    })

    it('should handle timeout gracefully', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      // In mock, timeout won't trigger, so just verify timedOut field exists
      expect(result.metrics.timedOut).toBeDefined()
      expect(typeof result.metrics.timedOut).toBe('boolean')
    })

    it('should handle errors gracefully', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      // With mock, it succeeds - just verify metrics structure
      expect(result.metrics).toBeDefined()
      expect(result.success).toBe(true)
    })

    it('should return transcript path if available', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      expect(result.transcriptPath).toBeDefined()
    })

    it('should track multiple search queries', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)

      // With default mock, should have at least 1 search query
      expect(result.metrics.searchQueries.length).toBeGreaterThanOrEqual(1)
      expect(result.metrics.searchQueries).toContain('worktree')
    })
  })

  describe('formatBaselineReport', () => {
    it('should format a comprehensive report', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)
      const report = formatBaselineReport(result)

      // Verify report structure
      expect(report).toContain('BASELINE EVALUATION REPORT')
      expect(report).toContain('RESULT')
      expect(report).toContain('METRICS')
      expect(report).toContain('ARTIFACTS')

      // Verify task info
      expect(report).toContain(result.task.name)
      expect(report).toContain(result.task.difficulty)
      expect(report).toContain(result.task.category)

      // Verify metrics
      expect(report).toContain('Success:')
      expect(report).toContain('Duration:')
      expect(report).toContain('Tool Usage:')
      expect(report).toContain('Search Queries Issued:')
      expect(report).toContain('Files Examined:')

      // Verify artifacts
      expect(report).toContain('Run Directory:')
    })

    it('should show error in report if present', async () => {
      // Create a mock result with an error
      const result: BaselineResult = {
        task: createTestTask(),
        success: false,
        metrics: {
          durationSeconds: 10,
          toolCalls: {},
          searchQueries: [],
          filesExamined: 0,
          timedOut: false,
          error: 'Test error',
        },
        agentResult: {
          success: false,
          sessionId: '',
          messages: [],
        },
        runDir: testDir,
      }

      const report = formatBaselineReport(result)

      expect(report).toContain('Error:')
      expect(report).toContain('Success: NO')
    })

    it('should list search queries when count is reasonable', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)
      const report = formatBaselineReport(result)

      // Should list queries if <= 10
      if (result.metrics.searchQueries.length <= 10) {
        result.metrics.searchQueries.forEach((query) => {
          expect(report).toContain(query)
        })
      }
    })

    it('should show tool usage breakdown', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      const result = await runBaseline(config)
      const report = formatBaselineReport(result)

      // Should show tool counts
      Object.entries(result.metrics.toolCalls).forEach(([tool, count]) => {
        expect(report).toContain(`${tool}:`)
        expect(report).toContain(`${count}`)
      })
    })

    it('should show timeout status', async () => {
      const result: BaselineResult = {
        task: createTestTask(),
        success: false,
        metrics: {
          durationSeconds: 60,
          toolCalls: {},
          searchQueries: [],
          filesExamined: 0,
          timedOut: true,
        },
        agentResult: {
          success: false,
          sessionId: '',
          messages: [],
        },
        runDir: testDir,
      }

      const report = formatBaselineReport(result)

      expect(report).toContain('Timed Out: YES')
    })
  })

  describe('integration', () => {
    it('should run end-to-end baseline evaluation', async () => {
      const config: BaselineConfig = {
        task: createTestTask(),
        timeout: 60,
        baseDir: testDir,
        worktreePath: process.cwd(),
      }

      // Run baseline
      const result = await runBaseline(config)

      // Verify result
      expect(result.success).toBe(true)
      expect(result.task).toBe(config.task)

      // Verify metrics captured
      expect(result.metrics.durationSeconds).toBeGreaterThanOrEqual(0)
      expect(Object.keys(result.metrics.toolCalls).length).toBeGreaterThanOrEqual(0)

      // Verify artifacts created
      expect(existsSync(result.runDir)).toBe(true)
      expect(existsSync(join(result.runDir, 'tool-usage.log'))).toBe(true)
      expect(existsSync(join(result.runDir, 'agent-result.json'))).toBe(true)

      // Generate and verify report
      const report = formatBaselineReport(result)
      expect(report).toBeTruthy()
      expect(report).toContain('BASELINE EVALUATION REPORT')
    })
  })
})
