/**
 * Competition runner orchestrator
 *
 * Integrates all components to run end-to-end agent competitions
 */

import { mkdirSync, writeFileSync } from 'fs'
import { join } from 'path'
import type { SearchTask, Variant } from './types.js'
import type { SearchEvaluationSummary } from '../evaluation/checks.js'
import { runSearchTaskEvaluation } from '../evaluation/search-checks.js'
import { spawnAgentWithVariant } from '../sdk/spawner.js'
import type { ToolUseEvent, AgentResult } from '../sdk/types.js'

/**
 * Competition configuration
 */
export interface CompetitionConfig {
  task: SearchTask // Which task to run
  variants: Variant[] // Variants to compete
  parallelExecution?: boolean // Run agents in parallel? (default: false)
  timeout?: number // Max time per agent in seconds (default: 300)
  baseDir?: string // Base directory for competition runs (default: .crewchief/competitions)
}

/**
 * Competition result
 */
export interface CompetitionResult {
  competitionId: string
  task: SearchTask
  participants: ParticipantResult[]
  winner: ParticipantResult
  metrics: CompetitionMetrics
  report: string
}

/**
 * Participant result
 */
export interface ParticipantResult {
  variantId: string
  variantName: string
  score: number
  evaluation: SearchEvaluationSummary
  agentResult: AgentResult
  toolsUsed?: string[] // Tool names used during task execution
  searchCount: number // Number of searches performed
  toolCallCount: number // Total tool calls made
  durationSeconds: number // Time taken to complete task
}

/**
 * Competition metrics
 */
export interface CompetitionMetrics {
  avgScore: number
  scoreRange: { min: number; max: number }
  avgSearchCount: number
  successRate: number
}

/**
 * Run a complete competition
 *
 * @param config - Competition configuration
 * @returns Competition result with winner
 */
export async function runCompetition(config: CompetitionConfig): Promise<CompetitionResult> {
  const competitionId = `comp-${Date.now()}`
  const baseDir = config.baseDir || join('.crewchief', 'competitions', competitionId)

  console.log(`\nStarting competition: ${config.task.name}`)
  console.log(`Variants: ${config.variants.map((v) => v.name).join(', ')}`)
  console.log(`Parallel: ${config.parallelExecution || false}`)

  // Create competition directory
  mkdirSync(baseDir, { recursive: true })

  // Execute participants
  const results = await executeParticipants(config, baseDir)

  // Determine winner
  const winner = determineWinner(results)

  // Calculate metrics
  const metrics = calculateCompetitionMetrics(results)

  // Generate report
  const report = generateCompetitionReport({
    competitionId,
    task: config.task,
    results,
    winner,
    metrics,
  })

  // Save report
  writeFileSync(join(baseDir, 'report.txt'), report)

  console.log(`\nWinner: ${winner.variantName} (${(winner.score * 100).toFixed(1)}%)`)
  console.log(`Report saved to: ${baseDir}/report.txt`)

  return {
    competitionId,
    task: config.task,
    participants: results,
    winner,
    metrics,
    report,
  }
}

/**
 * Execute all participants
 */
async function executeParticipants(config: CompetitionConfig, baseDir: string): Promise<ParticipantResult[]> {
  if (config.parallelExecution) {
    // Execute in parallel
    return Promise.all(config.variants.map((variant) => executeParticipant(variant, config, baseDir)))
  } else {
    // Execute sequentially
    const results: ParticipantResult[] = []
    for (const variant of config.variants) {
      const result = await executeParticipant(variant, config, baseDir)
      results.push(result)
    }
    return results
  }
}

/**
 * Execute a single participant
 */
async function executeParticipant(
  variant: Variant,
  config: CompetitionConfig,
  baseDir: string,
): Promise<ParticipantResult> {
  console.log(`  Running: ${variant.name}...`)

  const runDir = join(baseDir, `run-${variant.id}`)
  mkdirSync(runDir, { recursive: true })

  // Log file for tool usage
  const toolLogPath = join(runDir, 'tool-usage.log')

  // Spawn agent with variant
  const agentResult = await spawnAgentWithVariant(
    config.task.description,
    variant,
    {
      onToolUse: (event: ToolUseEvent) => {
        // Log tool usage to file
        writeFileSync(toolLogPath, JSON.stringify(event) + '\n', { flag: 'a' })
      },
    },
    {
      maxTurns: config.timeout ? Math.floor(config.timeout / 10) : 30,
    },
  )

  // Save agent result
  writeFileSync(join(runDir, 'agent-result.json'), JSON.stringify(agentResult, null, 2))

  // Note: For now, we use the worktree from the agent result
  // In a real implementation, we'd track the worktree path during execution
  const worktreePath = agentResult.transcriptPath ? join(agentResult.transcriptPath, '..', '..') : process.cwd()

  // Evaluate the result
  const evaluation = await runSearchTaskEvaluation(config.task, worktreePath, runDir)

  console.log(`  Completed: ${variant.name} - Score: ${(evaluation.compositeScore * 100).toFixed(1)}%`)

  // Extract tool usage from messages
  const toolsUsed = new Set<string>()
  let toolCallCount = 0
  for (const message of agentResult.messages) {
    if (message.content) {
      for (const block of Array.isArray(message.content) ? message.content : [message.content]) {
        if (typeof block === 'object' && block !== null && 'type' in block && block.type === 'tool_use') {
          toolCallCount++
          if ('name' in block && typeof block.name === 'string') {
            toolsUsed.add(block.name)
          }
        }
      }
    }
  }

  return {
    variantId: variant.id,
    variantName: variant.name,
    score: evaluation.compositeScore,
    evaluation,
    agentResult,
    toolsUsed: Array.from(toolsUsed),
    searchCount: evaluation.searchUsageScore || 0,
    toolCallCount,
    durationSeconds: (agentResult.performance?.durationMs || 0) / 1000,
  }
}

/**
 * Determine the winner
 */
function determineWinner(results: ParticipantResult[]): ParticipantResult {
  if (results.length === 0) {
    throw new Error('No participants to determine winner from')
  }

  // Sort by score descending, then by efficiency, then by search quality
  const sorted = [...results].sort((a, b) => {
    // Primary: composite score
    if (b.score !== a.score) {
      return b.score - a.score
    }

    // Tiebreaker 1: efficiency
    if (b.evaluation.taskScore.efficiency !== a.evaluation.taskScore.efficiency) {
      return b.evaluation.taskScore.efficiency - a.evaluation.taskScore.efficiency
    }

    // Tiebreaker 2: search quality
    return b.evaluation.taskScore.searchQuality - a.evaluation.taskScore.searchQuality
  })

  return sorted[0]
}

/**
 * Calculate competition metrics
 */
function calculateCompetitionMetrics(results: ParticipantResult[]): CompetitionMetrics {
  const scores = results.map((r) => r.score)
  const searchCounts = results.map((r) => r.evaluation.searchMetrics.searchCount)
  const successfulParticipants = results.filter((r) => r.evaluation.taskScore.taskCompletion > 0.5)

  return {
    avgScore: scores.reduce((sum, score) => sum + score, 0) / scores.length,
    scoreRange: {
      min: Math.min(...scores),
      max: Math.max(...scores),
    },
    avgSearchCount: searchCounts.reduce((sum, count) => sum + count, 0) / searchCounts.length,
    successRate: successfulParticipants.length / results.length,
  }
}

/**
 * Generate competition report
 */
function generateCompetitionReport(data: {
  competitionId: string
  task: SearchTask
  results: ParticipantResult[]
  winner: ParticipantResult
  metrics: CompetitionMetrics
}): string {
  const lines: string[] = []

  lines.push('COMPETITION REPORT')
  lines.push('='.repeat(60))
  lines.push('')
  lines.push(`Competition ID: ${data.competitionId}`)
  lines.push(`Task: ${data.task.name}`)
  lines.push(`Difficulty: ${data.task.difficulty}`)
  lines.push(`Category: ${data.task.category}`)
  lines.push('')

  lines.push('RESULTS')
  lines.push('-'.repeat(60))
  data.results
    .sort((a, b) => b.score - a.score)
    .forEach((r, i) => {
      lines.push(`${i + 1}. ${r.variantName}`)
      lines.push(`   Score: ${(r.score * 100).toFixed(1)}%`)
      lines.push(`   Search Quality: ${(r.evaluation.taskScore.searchQuality * 100).toFixed(1)}%`)
      lines.push(`   Task Completion: ${(r.evaluation.taskScore.taskCompletion * 100).toFixed(1)}%`)
      lines.push(`   Efficiency: ${(r.evaluation.taskScore.efficiency * 100).toFixed(1)}%`)
      lines.push(`   Searches: ${r.evaluation.searchMetrics.searchCount}`)
      lines.push(`   Target Found: ${r.evaluation.searchMetrics.targetFound ? 'YES' : 'NO'}`)
      lines.push('')
    })

  lines.push('WINNER')
  lines.push('-'.repeat(60))
  lines.push(`${data.winner.variantName} (${(data.winner.score * 100).toFixed(1)}%)`)
  lines.push('')
  lines.push(data.winner.evaluation.taskScore.details)
  lines.push('')

  lines.push('COMPETITION METRICS')
  lines.push('-'.repeat(60))
  lines.push(`Average Score: ${(data.metrics.avgScore * 100).toFixed(1)}%`)
  lines.push(
    `Score Range: ${(data.metrics.scoreRange.min * 100).toFixed(1)}% - ${(data.metrics.scoreRange.max * 100).toFixed(1)}%`,
  )
  lines.push(`Average Searches: ${data.metrics.avgSearchCount.toFixed(1)}`)
  lines.push(`Success Rate: ${(data.metrics.successRate * 100).toFixed(1)}%`)
  lines.push('')

  lines.push('NEXT STEPS')
  lines.push('-'.repeat(60))
  lines.push('- Use winner as baseline for next generation')
  lines.push('- Generate mutations from winner')
  lines.push('- Run next competition')
  lines.push('')

  return lines.join('\n')
}
