/**
 * Search-specific evaluation checks
 */

import fs from 'node:fs'
import path from 'node:path'
import type { SearchEvaluationSummary } from './checks.js'
import { runDefaultChecks } from './checks.js'
import type { SearchTask, AgentOutput, SearchResult, WorkResult, SearchTarget } from '../search-optimization/types.js'

/**
 * Tool use log entry
 */
export interface ToolUseLog {
  timestamp: number
  toolName: string
  arguments: Record<string, unknown>
  result: unknown
}

/**
 * Run complete search task evaluation
 *
 * @param task - The search task being evaluated
 * @param worktreePath - Path to the worktree where agent worked
 * @param runDir - Path to the run directory with logs
 * @returns Complete evaluation summary
 */
export async function runSearchTaskEvaluation(
  task: SearchTask,
  worktreePath: string,
  runDir: string,
): Promise<SearchEvaluationSummary> {
  // 1. Load tool usage logs
  const toolLogs = await loadToolUsageLogs(runDir)

  // 2. Extract search metrics
  const searchMetrics = extractSearchMetrics(toolLogs, task.searchTarget)

  // 3. Validate task completion
  const workResult = await loadWorkResult(worktreePath, runDir)

  // 4. Build AgentOutput for validator
  const agentOutput: AgentOutput = {
    searchResults: extractSearchResults(toolLogs),
    workResult,
    searchCount: searchMetrics.searchCount,
    toolCallCount: toolLogs.length,
    durationSeconds: calculateDuration(toolLogs),
  }

  // 5. Run task validator
  const taskScore = task.successValidator(agentOutput)

  // 6. Calculate tool usage stats
  const toolUsage = calculateToolUsage(toolLogs)

  // 7. Calculate timing
  const timing = calculateTiming(toolLogs, searchMetrics.targetFoundInTop !== null)

  // 8. Run generic checks (from existing framework)
  const genericChecks = await runDefaultChecks(worktreePath, runDir)

  // 9. Combine into summary
  return {
    ...genericChecks,
    task,
    taskScore,
    searchMetrics,
    toolUsage,
    timing,
    compositeScore: taskScore.total,
  }
}

/**
 * Load tool usage logs from run directory
 */
export async function loadToolUsageLogs(runDir: string): Promise<ToolUseLog[]> {
  const logsPath = path.join(runDir, 'tool-usage.log')

  if (!fs.existsSync(logsPath)) {
    // No logs found - return empty array
    return []
  }

  try {
    const content = fs.readFileSync(logsPath, 'utf-8')
    const lines = content.trim().split('\n')

    return lines
      .filter((line) => line.trim())
      .map((line) => {
        try {
          return JSON.parse(line) as ToolUseLog
        } catch {
          return null
        }
      })
      .filter((log): log is ToolUseLog => log !== null)
  } catch {
    return []
  }
}

/**
 * Extract search metrics from tool logs
 */
export function extractSearchMetrics(
  logs: ToolUseLog[],
  target: SearchTarget,
): SearchEvaluationSummary['searchMetrics'] {
  const searchLogs = logs.filter((log) => log.toolName === 'search' || log.toolName === 'mcp__maproom__search')

  // Extract queries
  const queriesIssued = searchLogs.map((log) => (log.arguments.query as string) || '').filter((q) => q.length > 0)

  // Calculate average results per search
  const totalResults = searchLogs.reduce((sum, log) => {
    const results = log.result as Array<unknown> | undefined
    return sum + (results?.length || 0)
  }, 0)
  const avgResultsPerSearch = searchLogs.length > 0 ? totalResults / searchLogs.length : 0

  // Check if target was found
  const { found, rank } = findTargetInLogs(searchLogs, target)

  return {
    searchCount: searchLogs.length,
    avgResultsPerSearch,
    queriesIssued,
    targetFound: found,
    targetFoundInTop: rank,
  }
}

/**
 * Find target in search logs
 */
function findTargetInLogs(logs: ToolUseLog[], target: SearchTarget): { found: boolean; rank: number | null } {
  for (const log of logs) {
    const results = (log.result as Array<Record<string, unknown>>) || []

    for (let i = 0; i < Math.min(results.length, 20); i++) {
      const result = results[i]
      if (matchesTarget(result, target)) {
        return { found: true, rank: i + 1 }
      }
    }
  }

  return { found: false, rank: null }
}

/**
 * Check if a search result matches the target
 */
function matchesTarget(result: Record<string, unknown>, target: SearchTarget): boolean {
  switch (target.type) {
    case 'file': {
      const filePath = (result.relpath || result.path || result.file) as string | undefined
      if (!filePath) return false

      if (target.path && filePath.includes(target.path)) return true
      if (target.alternatives) {
        return target.alternatives.some((alt) => filePath.includes(alt))
      }
      return false
    }

    case 'function':
    case 'class': {
      const name = target.name
      if (!name) return false

      const content = ((result.content || result.text) as string) || ''
      const symbolName = ((result.symbol || result.name) as string) || ''

      return content.includes(name) || symbolName.includes(name)
    }

    case 'pattern': {
      if (!target.pattern) return false

      const text = ((result.content || result.text) as string) || ''
      return target.pattern.test(text)
    }

    default:
      return false
  }
}

/**
 * Extract search results from logs
 */
function extractSearchResults(logs: ToolUseLog[]): SearchResult[] {
  const searchLogs = logs.filter((log) => log.toolName === 'search' || log.toolName === 'mcp__maproom__search')

  return searchLogs.map((log) => ({
    query: (log.arguments.query as string) || '',
    results: (log.result as Array<Record<string, unknown>>) || [],
  }))
}

/**
 * Load work result from worktree
 */
export async function loadWorkResult(worktreePath: string, runDir: string): Promise<WorkResult> {
  // Try to load work result from JSON file
  const resultPath = path.join(runDir, 'work-result.json')

  if (fs.existsSync(resultPath)) {
    try {
      const content = fs.readFileSync(resultPath, 'utf-8')
      return JSON.parse(content) as WorkResult
    } catch {
      // Fall through to default
    }
  }

  // Fallback: check git status for changed files
  try {
    const { execSync } = await import('child_process')
    const output = execSync('git status --porcelain', {
      cwd: worktreePath,
      encoding: 'utf-8',
    })

    const filesChanged = output
      .split('\n')
      .filter((line) => line.trim())
      .map((line) => line.substring(3).trim())

    return {
      filesChanged,
      success: filesChanged.length > 0,
    }
  } catch {
    return {
      success: false,
    }
  }
}

/**
 * Calculate tool usage statistics
 */
export function calculateToolUsage(logs: ToolUseLog[]): SearchEvaluationSummary['toolUsage'] {
  const searchToolCalls = logs.filter(
    (log) => log.toolName === 'search' || log.toolName === 'mcp__maproom__search',
  ).length

  const otherToolCalls: Record<string, number> = {}

  for (const log of logs) {
    if (log.toolName !== 'search' && log.toolName !== 'mcp__maproom__search') {
      otherToolCalls[log.toolName] = (otherToolCalls[log.toolName] || 0) + 1
    }
  }

  return {
    totalToolCalls: logs.length,
    searchToolCalls,
    otherToolCalls,
  }
}

/**
 * Calculate timing information
 */
export function calculateTiming(logs: ToolUseLog[], targetFound: boolean): SearchEvaluationSummary['timing'] {
  if (logs.length === 0) {
    return {
      totalSeconds: 0,
      timeToTarget: null,
    }
  }

  const firstTimestamp = logs[0].timestamp
  const lastTimestamp = logs[logs.length - 1].timestamp

  const totalSeconds = (lastTimestamp - firstTimestamp) / 1000

  // Time to target is when first search found it
  // For now, just use null or total time
  const timeToTarget = targetFound ? totalSeconds : null

  return {
    totalSeconds,
    timeToTarget,
  }
}

/**
 * Calculate duration from logs
 */
function calculateDuration(logs: ToolUseLog[]): number {
  if (logs.length === 0) return 0

  const firstTimestamp = logs[0].timestamp
  const lastTimestamp = logs[logs.length - 1].timestamp

  return (lastTimestamp - firstTimestamp) / 1000
}
