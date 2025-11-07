/**
 * Baseline runner for grep-only evaluation
 *
 * Executes SearchTask with grep/glob/read tools only (no semantic search)
 * to establish performance baselines for objective comparison.
 *
 * This implements the "Baseline Measurement" component from TESTDES architecture,
 * providing the control condition against which semantic search performance
 * will be compared in Phase 2.
 */

import { mkdirSync, writeFileSync } from 'fs'
import { join } from 'path'
import { spawnAgent } from '../../sdk/spawner.js'
import type { ToolUseEvent, AgentResult } from '../../sdk/types.js'
import type { SearchTask } from '../types.js'

/**
 * Configuration for baseline execution
 */
export interface BaselineConfig {
  /** Task to execute */
  task: SearchTask

  /** Tools available to the agent (defaults to grep/glob/read only) */
  availableTools?: string[]

  /** Maximum execution time in seconds (default: 300 = 5 minutes) */
  timeout?: number

  /** Base directory for baseline runs (default: .crewchief/baselines) */
  baseDir?: string

  /** Working directory (worktree path) */
  worktreePath?: string
}

/**
 * Baseline execution result
 */
export interface BaselineResult {
  /** Task that was executed */
  task: SearchTask

  /** Whether the agent completed the task successfully */
  success: boolean

  /** Comprehensive metrics */
  metrics: BaselineMetrics

  /** Full agent execution result */
  agentResult: AgentResult

  /** Path to the run directory with logs and transcript */
  runDir: string

  /** Path to the transcript file */
  transcriptPath?: string
}

/**
 * Metrics captured during baseline execution
 */
export interface BaselineMetrics {
  /** Total execution time in seconds */
  durationSeconds: number

  /** Count of tool calls by type */
  toolCalls: Record<string, number>

  /** All grep/search queries attempted */
  searchQueries: string[]

  /** Number of unique files examined (read) */
  filesExamined: number

  /** Whether execution hit the timeout */
  timedOut: boolean

  /** Error message if execution failed */
  error?: string
}

/**
 * Default tools available in baseline mode (grep/glob/read only)
 */
const DEFAULT_BASELINE_TOOLS = [
  'Grep',
  'Glob',
  'Read',
  'Bash', // Allow bash for basic operations
  'Edit', // Allow editing for task completion
  'Write', // Allow writing for task completion
]

/**
 * Run a baseline evaluation with grep/glob/read tools only
 *
 * @param config - Baseline configuration
 * @returns Baseline result with metrics
 *
 * @example
 * ```typescript
 * const result = await runBaseline({
 *   task: mySearchTask,
 *   timeout: 300, // 5 minutes
 *   worktreePath: '/path/to/worktree',
 * })
 *
 * console.log('Success:', result.success)
 * console.log('Duration:', result.metrics.durationSeconds)
 * console.log('Tool calls:', result.metrics.toolCalls)
 * ```
 */
export async function runBaseline(config: BaselineConfig): Promise<BaselineResult> {
  const baselineId = `baseline-${Date.now()}`
  const baseDir = config.baseDir || join('.crewchief', 'baselines', baselineId)
  const runDir = baseDir

  console.log(`\nStarting baseline: ${config.task.name}`)
  console.log('Tools: grep/glob/read only')
  console.log(`Timeout: ${config.timeout || 300}s`)

  // Create baseline directory
  mkdirSync(runDir, { recursive: true })

  // Track metrics during execution
  const startTime = Date.now()
  const toolCalls: Record<string, number> = {}
  const searchQueries: string[] = []
  const filesExamined = new Set<string>()
  let timedOut = false
  let error: string | undefined

  // Log file for tool usage
  const toolLogPath = join(runDir, 'tool-usage.log')

  try {
    // Set up timeout
    const timeoutMs = (config.timeout || 300) * 1000
    const timeoutPromise = new Promise<AgentResult>((resolve) => {
      setTimeout(() => {
        timedOut = true
        resolve({
          success: false,
          messages: [],
          sessionId: '',
          transcriptPath: undefined,
        })
      }, timeoutMs)
    })

    // Spawn agent with restricted tools
    const agentPromise = spawnAgent({
      task: config.task.description,
      worktreePath: config.worktreePath || process.cwd(),
      hooks: {
        onToolUse: (event: ToolUseEvent) => {
          // Log tool usage
          const logEntry = {
            timestamp: event.timestamp,
            toolName: event.tool_name,
            arguments: event.tool_input,
            result: null, // Will be filled by onToolResult
          }
          writeFileSync(toolLogPath, JSON.stringify(logEntry) + '\n', { flag: 'a' })

          // Track metrics
          toolCalls[event.tool_name] = (toolCalls[event.tool_name] || 0) + 1

          // Capture search queries (from Grep tool)
          if (event.tool_name === 'Grep' && event.tool_input.pattern) {
            searchQueries.push(event.tool_input.pattern as string)
          }

          // Track files examined (from Read tool)
          if (event.tool_name === 'Read' && event.tool_input.file_path) {
            filesExamined.add(event.tool_input.file_path as string)
          }
        },
      },
      allowedTools: config.availableTools || DEFAULT_BASELINE_TOOLS,
      maxTurns: Math.floor((config.timeout || 300) / 10), // Estimate ~10s per turn
    })

    // Race between agent execution and timeout
    const agentResult = await Promise.race([agentPromise, timeoutPromise])

    const endTime = Date.now()
    const durationSeconds = (endTime - startTime) / 1000

    // Save agent result
    writeFileSync(join(runDir, 'agent-result.json'), JSON.stringify(agentResult, null, 2))

    // Build metrics
    const metrics: BaselineMetrics = {
      durationSeconds,
      toolCalls,
      searchQueries,
      filesExamined: filesExamined.size,
      timedOut,
    }

    // Determine success
    const success = agentResult.success && !timedOut

    console.log(`  Completed: ${success ? 'SUCCESS' : 'FAILED'}`)
    console.log(`  Duration: ${durationSeconds.toFixed(1)}s`)
    console.log(`  Tool calls: ${Object.values(toolCalls).reduce((a, b) => a + b, 0)}`)
    console.log(`  Search queries: ${searchQueries.length}`)
    console.log(`  Files examined: ${filesExamined.size}`)

    return {
      task: config.task,
      success,
      metrics,
      agentResult,
      runDir,
      transcriptPath: agentResult.transcriptPath,
    }
  } catch (err) {
    // Handle errors gracefully
    const endTime = Date.now()
    const durationSeconds = (endTime - startTime) / 1000

    error = err instanceof Error ? err.message : String(err)

    console.error(`  Error: ${error}`)

    const metrics: BaselineMetrics = {
      durationSeconds,
      toolCalls,
      searchQueries,
      filesExamined: filesExamined.size,
      timedOut: false,
      error,
    }

    // Return failure result with error details
    return {
      task: config.task,
      success: false,
      metrics,
      agentResult: {
        success: false,
        messages: [],
        sessionId: '',
        transcriptPath: undefined,
      },
      runDir,
    }
  }
}

/**
 * Format baseline result as a human-readable report
 *
 * @param result - Baseline result to format
 * @returns Formatted report string
 */
export function formatBaselineReport(result: BaselineResult): string {
  const lines: string[] = []

  lines.push('BASELINE EVALUATION REPORT')
  lines.push('='.repeat(60))
  lines.push('')
  lines.push(`Task: ${result.task.name}`)
  lines.push(`Difficulty: ${result.task.difficulty}`)
  lines.push(`Category: ${result.task.category}`)
  lines.push('')

  lines.push('RESULT')
  lines.push('-'.repeat(60))
  lines.push(`Success: ${result.success ? 'YES' : 'NO'}`)
  lines.push(`Duration: ${result.metrics.durationSeconds.toFixed(1)}s`)
  lines.push(`Timed Out: ${result.metrics.timedOut ? 'YES' : 'NO'}`)
  if (result.metrics.error) {
    lines.push(`Error: ${result.metrics.error}`)
  }
  lines.push('')

  lines.push('METRICS')
  lines.push('-'.repeat(60))
  lines.push(`Total Tool Calls: ${Object.values(result.metrics.toolCalls).reduce((a, b) => a + b, 0)}`)
  lines.push('')
  lines.push('Tool Usage:')
  Object.entries(result.metrics.toolCalls)
    .sort(([, a], [, b]) => b - a)
    .forEach(([tool, count]) => {
      lines.push(`  ${tool}: ${count}`)
    })
  lines.push('')
  lines.push(`Search Queries Issued: ${result.metrics.searchQueries.length}`)
  if (result.metrics.searchQueries.length > 0 && result.metrics.searchQueries.length <= 10) {
    result.metrics.searchQueries.forEach((query, i) => {
      lines.push(`  ${i + 1}. ${query}`)
    })
  }
  lines.push(`Files Examined: ${result.metrics.filesExamined}`)
  lines.push('')

  lines.push('ARTIFACTS')
  lines.push('-'.repeat(60))
  lines.push(`Run Directory: ${result.runDir}`)
  if (result.transcriptPath) {
    lines.push(`Transcript: ${result.transcriptPath}`)
  }
  lines.push('')

  return lines.join('\n')
}
