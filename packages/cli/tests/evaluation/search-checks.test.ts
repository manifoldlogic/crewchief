/**
 * Tests for search evaluation framework
 */

import { mkdtempSync, rmSync, writeFileSync, mkdirSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import type { ToolUseLog } from '../../src/evaluation/search-checks.js'
import {
  loadToolUsageLogs,
  extractSearchMetrics,
  calculateToolUsage,
  calculateTiming,
  runSearchTaskEvaluation,
} from '../../src/evaluation/search-checks.js'
import { TASK_FIND_WORKTREE_CREATION } from '../../src/search-optimization/tasks/implementation.js'
import type { SearchTarget } from '../../src/search-optimization/types.js'

describe('search-checks', () => {
  let testDir: string

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'search-checks-test-'))
  })

  afterEach(() => {
    try {
      rmSync(testDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  })

  describe('loadToolUsageLogs', () => {
    it('should load JSONL tool logs', async () => {
      const logsPath = join(testDir, 'tool-usage.log')
      const logs: ToolUseLog[] = [
        {
          timestamp: Date.now(),
          toolName: 'search',
          arguments: { query: 'test' },
          result: [],
        },
        {
          timestamp: Date.now() + 1000,
          toolName: 'read',
          arguments: { file: 'test.ts' },
          result: 'content',
        },
      ]

      writeFileSync(logsPath, logs.map((log) => JSON.stringify(log)).join('\n'))

      const loaded = await loadToolUsageLogs(testDir)

      expect(loaded).toHaveLength(2)
      expect(loaded[0].toolName).toBe('search')
      expect(loaded[1].toolName).toBe('read')
    })

    it('should return empty array when no logs exist', async () => {
      const loaded = await loadToolUsageLogs(testDir)
      expect(loaded).toEqual([])
    })

    it('should skip invalid JSON lines', async () => {
      const logsPath = join(testDir, 'tool-usage.log')
      writeFileSync(
        logsPath,
        [
          JSON.stringify({ timestamp: Date.now(), toolName: 'search', arguments: {}, result: [] }),
          'invalid json',
          JSON.stringify({ timestamp: Date.now(), toolName: 'read', arguments: {}, result: '' }),
        ].join('\n'),
      )

      const loaded = await loadToolUsageLogs(testDir)

      expect(loaded).toHaveLength(2)
    })
  })

  describe('extractSearchMetrics', () => {
    it('should extract search count and queries', () => {
      const logs: ToolUseLog[] = [
        {
          timestamp: Date.now(),
          toolName: 'search',
          arguments: { query: 'worktree' },
          result: [{ relpath: 'src/git/worktree.ts' }],
        },
        {
          timestamp: Date.now(),
          toolName: 'mcp__maproom__search',
          arguments: { query: 'agent spawn' },
          result: [{ relpath: 'src/sdk/spawner.ts' }],
        },
        {
          timestamp: Date.now(),
          toolName: 'read',
          arguments: { file: 'test.ts' },
          result: 'content',
        },
      ]

      const target: SearchTarget = {
        type: 'file',
        path: 'src/git/worktree.ts',
      }

      const metrics = extractSearchMetrics(logs, target)

      expect(metrics.searchCount).toBe(2)
      expect(metrics.queriesIssued).toEqual(['worktree', 'agent spawn'])
      expect(metrics.targetFound).toBe(true)
      expect(metrics.targetFoundInTop).toBe(1)
    })

    it('should detect when target not found', () => {
      const logs: ToolUseLog[] = [
        {
          timestamp: Date.now(),
          toolName: 'search',
          arguments: { query: 'test' },
          result: [{ relpath: 'other.ts' }],
        },
      ]

      const target: SearchTarget = {
        type: 'file',
        path: 'target.ts',
      }

      const metrics = extractSearchMetrics(logs, target)

      expect(metrics.targetFound).toBe(false)
      expect(metrics.targetFoundInTop).toBe(null)
    })

    it('should calculate average results per search', () => {
      const logs: ToolUseLog[] = [
        {
          timestamp: Date.now(),
          toolName: 'search',
          arguments: { query: 'test' },
          result: [1, 2, 3], // 3 results
        },
        {
          timestamp: Date.now(),
          toolName: 'search',
          arguments: { query: 'another' },
          result: [1], // 1 result
        },
      ]

      const target: SearchTarget = {
        type: 'file',
        path: 'test.ts',
      }

      const metrics = extractSearchMetrics(logs, target)

      expect(metrics.avgResultsPerSearch).toBe(2) // (3 + 1) / 2
    })

    it('should match pattern targets', () => {
      const logs: ToolUseLog[] = [
        {
          timestamp: Date.now(),
          toolName: 'search',
          arguments: { query: 'competition' },
          result: [{ content: 'CompetitionManager class' }],
        },
      ]

      const target: SearchTarget = {
        type: 'pattern',
        pattern: /CompetitionManager/,
      }

      const metrics = extractSearchMetrics(logs, target)

      expect(metrics.targetFound).toBe(true)
      expect(metrics.targetFoundInTop).toBe(1)
    })
  })

  describe('calculateToolUsage', () => {
    it('should count tool calls', () => {
      const logs: ToolUseLog[] = [
        { timestamp: Date.now(), toolName: 'search', arguments: {}, result: [] },
        { timestamp: Date.now(), toolName: 'search', arguments: {}, result: [] },
        { timestamp: Date.now(), toolName: 'read', arguments: {}, result: '' },
        { timestamp: Date.now(), toolName: 'write', arguments: {}, result: true },
        { timestamp: Date.now(), toolName: 'read', arguments: {}, result: '' },
      ]

      const usage = calculateToolUsage(logs)

      expect(usage.totalToolCalls).toBe(5)
      expect(usage.searchToolCalls).toBe(2)
      expect(usage.otherToolCalls.read).toBe(2)
      expect(usage.otherToolCalls.write).toBe(1)
    })

    it('should handle mcp__maproom__search', () => {
      const logs: ToolUseLog[] = [
        { timestamp: Date.now(), toolName: 'mcp__maproom__search', arguments: {}, result: [] },
        { timestamp: Date.now(), toolName: 'search', arguments: {}, result: [] },
      ]

      const usage = calculateToolUsage(logs)

      expect(usage.searchToolCalls).toBe(2)
    })
  })

  describe('calculateTiming', () => {
    it('should calculate duration from timestamps', () => {
      const logs: ToolUseLog[] = [
        { timestamp: 1000, toolName: 'search', arguments: {}, result: [] },
        { timestamp: 6000, toolName: 'read', arguments: {}, result: '' },
      ]

      const timing = calculateTiming(logs, true)

      expect(timing.totalSeconds).toBe(5) // (6000 - 1000) / 1000
      expect(timing.timeToTarget).toBe(5)
    })

    it('should return null timeToTarget when not found', () => {
      const logs: ToolUseLog[] = [
        { timestamp: 1000, toolName: 'search', arguments: {}, result: [] },
        { timestamp: 6000, toolName: 'read', arguments: {}, result: '' },
      ]

      const timing = calculateTiming(logs, false)

      expect(timing.totalSeconds).toBe(5)
      expect(timing.timeToTarget).toBe(null)
    })
  })

  describe('runSearchTaskEvaluation', () => {
    it('should run full evaluation', async () => {
      // Create test logs
      const logsPath = join(testDir, 'tool-usage.log')
      const logs: ToolUseLog[] = [
        {
          timestamp: Date.now(),
          toolName: 'search',
          arguments: { query: 'worktree creation' },
          result: [{ relpath: 'packages/cli/src/git/worktree.ts', content: 'worktree code' }],
        },
      ]
      writeFileSync(logsPath, logs.map((log) => JSON.stringify(log)).join('\n'))

      // Create work result
      const workResultPath = join(testDir, 'work-result.json')
      writeFileSync(
        workResultPath,
        JSON.stringify({
          explanationText: 'Worktree creation uses git worktree add',
          success: true,
        }),
      )

      // Create worktree directory (empty is fine for test)
      const worktreePath = join(testDir, 'worktree')
      mkdirSync(worktreePath, { recursive: true })

      const summary = await runSearchTaskEvaluation(TASK_FIND_WORKTREE_CREATION, worktreePath, testDir)

      expect(summary.task).toBe(TASK_FIND_WORKTREE_CREATION)
      expect(summary.taskScore).toBeDefined()
      expect(summary.searchMetrics).toBeDefined()
      expect(summary.toolUsage).toBeDefined()
      expect(summary.timing).toBeDefined()
      expect(summary.compositeScore).toBeGreaterThanOrEqual(0)
      expect(summary.compositeScore).toBeLessThanOrEqual(1)
    })
  })
})
