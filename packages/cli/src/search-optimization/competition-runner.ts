/**
 * Competition runner orchestrator
 *
 * Integrates all components to run end-to-end agent competitions with
 * three-phase validation:
 * 1. Phase 1: Setup (database check, base branch check, worktree creation, scanning)
 * 2. Phase 2: Per-variant validation (fail-fast)
 * 3. Phase 3: Agent execution (parallel, existing behavior preserved)
 */

import { mkdirSync, writeFileSync } from 'fs'
import { join } from 'path'
import { scanAllWorktrees } from './scan-orchestrator.js'
import type { SearchTask, Variant, SetupMetrics, VariantValidation } from './types.js'
import type { SearchEvaluationSummary } from '../evaluation/checks.js'
import { runSearchTaskEvaluation } from '../evaluation/search-checks.js'
import type { ToolUseEvent, AgentResult } from '../sdk/types.js'
import { validateCompetitionConfig } from './security/limits.js'
import { sanitizeDbUrl } from './security/sanitize.js'
import { PreFlightValidator } from './validation/pre-flight-validator.js'
import type { VariantEnvironment } from './validation/types.js'
import { createVariantWorktree } from '../sdk/variant-injection.js'
import { validateVariantId } from './security/validators.js'

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
  setupMetrics?: SetupMetrics
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
 * Run a complete competition with three-phase validation
 *
 * Phase 1: Setup (sequential) - Database check, base branch check, worktree creation, scanning
 * Phase 2: Validation (per-variant) - Validate each variant environment
 * Phase 3: Execution (parallel) - Run agents in validated environments
 *
 * @param config - Competition configuration
 * @returns Competition result with winner
 */
export async function runCompetition(config: CompetitionConfig): Promise<CompetitionResult> {
  const setupStartTime = Date.now()

  // ─────────────────────────────────────────────────────────
  // SECURITY VALIDATION (Before any operations)
  // ─────────────────────────────────────────────────────────

  // Validate competition config (resource limits)
  // Convert timeout from seconds to milliseconds for validation
  validateCompetitionConfig({
    variants: config.variants.map((v) => v.id),
    timeout: config.timeout ? config.timeout * 1000 : undefined,
  })

  // Validate all variant IDs (path traversal protection)
  for (const variant of config.variants) {
    validateVariantId(variant.id)
  }

  console.log('🏁 Starting competition with pre-flight validation')

  // ─────────────────────────────────────────────────────────
  // PHASE 1: SETUP (Sequential)
  // ─────────────────────────────────────────────────────────

  console.log('\n📋 Phase 1: Setup')
  console.log('='.repeat(60))

  // 1.1: Validate database connection
  const validator = new PreFlightValidator()
  const dbValid = await validator.checkDatabaseConnection()
  if (!dbValid) {
    const dbUrl = process.env.MAPROOM_DATABASE_URL || 'Not configured'
    throw new Error(
      `
❌ Pre-flight validation failed: Database connection failed

Troubleshooting:
- Verify PostgreSQL is running: docker ps | grep postgres
- Check MAPROOM_DATABASE_URL environment variable
- Test connection: psql $MAPROOM_DATABASE_URL -c "SELECT 1"

Current value: ${sanitizeDbUrl(dbUrl)}
    `.trim(),
    )
  }
  console.log('✅ Database connection verified')

  // 1.2: Verify base branch is indexed
  const baseIndexed = await validator.verifyBaseBranchIndexed('crewchief', 'main')
  if (!baseIndexed.indexed) {
    throw new Error(
      `
❌ Pre-flight validation failed: Base branch 'main' not indexed

Fix: Run scan on base branch first
$ maproom scan --repo crewchief --worktree main --root /workspace

This is a one-time setup step. Subsequent scans will be fast.
    `.trim(),
    )
  }
  console.log(`✅ Base branch indexed (${baseIndexed.chunkCount} chunks)`)

  // 1.3: Create competition directory
  const competitionId = `comp-${Date.now()}`
  const baseDir = config.baseDir || join('.crewchief', 'competitions', competitionId)
  mkdirSync(baseDir, { recursive: true })
  console.log(`✅ Competition directory: ${baseDir}`)
  console.log(`✅ Loaded ${config.variants.length} variants`)

  // 1.4: Create variant worktrees
  console.log('\n📦 Creating variant worktrees...')
  const variantEnvironments: Array<{
    variant: Variant
    worktreePath: string
    worktreeName: string
    cleanup: () => Promise<void>
  }> = []

  for (const variant of config.variants) {
    const { path: worktreePath, cleanup } = await createVariantWorktree(variant, baseDir)
    // Extract worktree name from path (last segment)
    const worktreeName = worktreePath.split('/').pop() || variant.id
    variantEnvironments.push({
      variant,
      worktreePath,
      worktreeName,
      cleanup,
    })
    console.log(`✅ Created worktree for ${variant.name}`)
  }

  // 1.5: Scan all worktrees
  console.log('\n📊 Scanning worktrees...')
  const scanConfigs = variantEnvironments.map((env) => ({
    worktreePath: env.worktreePath,
    repo: 'crewchief',
    worktree: env.worktreeName,
    commit: 'HEAD',
    baseDir,
  }))

  const scanResults = await scanAllWorktrees(scanConfigs)
  console.log(`✅ All worktrees scanned (${scanResults.length} total)`)

  // ─────────────────────────────────────────────────────────
  // PHASE 2: VALIDATION (Per-Variant)
  // ─────────────────────────────────────────────────────────

  console.log('\n🔍 Phase 2: Pre-Flight Validation')
  console.log('='.repeat(60))

  const validationResults: VariantValidation[] = []
  for (const env of variantEnvironments) {
    const variantEnv: VariantEnvironment = {
      variantId: env.variant.id,
      worktreePath: env.worktreePath,
      repo: 'crewchief',
      worktree: env.worktreeName,
    }

    const validation = await validator.validateVariantEnvironment(variantEnv)
    validationResults.push(validation)

    if (validation.overall === 'fail') {
      console.error(`❌ Validation failed for ${env.variant.name}: ${validation.failureReason}`)

      // Log all failed checks
      Object.entries(validation.checks).forEach(([check, result]) => {
        if (!result.passed) {
          console.error(`   - ${check}: ${result.message}`)
        }
      })

      // Cleanup all worktrees before throwing
      for (const cleanupEnv of variantEnvironments) {
        try {
          await cleanupEnv.cleanup()
        } catch (cleanupError) {
          console.error(`Failed to cleanup worktree ${cleanupEnv.worktreeName}:`, cleanupError)
        }
      }

      throw new Error(`Pre-flight validation failed: ${validation.failureReason}`)
    }

    console.log(`✅ ${env.variant.name}: All checks passed`)
  }

  console.log('\n✅ All variants validated - ready for execution')

  const setupEndTime = Date.now()
  const totalSetupTimeMs = setupEndTime - setupStartTime

  const setupMetrics: SetupMetrics = {
    scanResults,
    validationResults,
    totalSetupTimeMs,
  }

  // ─────────────────────────────────────────────────────────
  // PHASE 3: EXECUTION (Parallel)
  // ─────────────────────────────────────────────────────────

  console.log('\n🚀 Phase 3: Agent Execution')
  console.log('='.repeat(60))

  try {
    // Execute participants
    const results = await executeParticipants(config, baseDir, variantEnvironments)

    // Determine winner
    const winner = determineWinner(results)

    // Calculate metrics
    const metrics = calculateCompetitionMetrics(results)

    // ─────────────────────────────────────────────────────────
    // PHASE 4: EVALUATION
    // ─────────────────────────────────────────────────────────

    console.log('\n📊 Phase 4: Evaluation')
    console.log('='.repeat(60))

    // Generate report with setup metrics
    const report = generateCompetitionReport({
      competitionId,
      task: config.task,
      results,
      winner,
      metrics,
      setupMetrics,
    })

    // Save report
    writeFileSync(join(baseDir, 'report.txt'), report)
    console.log(`\n📄 Report saved: ${join(baseDir, 'report.txt')}`)

    console.log(`\nWinner: ${winner.variantName} (${(winner.score * 100).toFixed(1)}%)`)

    return {
      competitionId,
      task: config.task,
      participants: results,
      winner,
      metrics,
      setupMetrics,
      report,
    }
  } finally {
    // Always cleanup worktrees
    console.log('\n🧹 Cleaning up worktrees...')
    for (const env of variantEnvironments) {
      try {
        await env.cleanup()
        console.log(`✅ Cleaned up ${env.worktreeName}`)
      } catch (cleanupError) {
        console.error(`❌ Failed to cleanup worktree ${env.worktreeName}:`, cleanupError)
      }
    }
  }
}

/**
 * Execute all participants in their pre-created, validated worktrees
 */
async function executeParticipants(
  config: CompetitionConfig,
  baseDir: string,
  variantEnvironments: Array<{
    variant: Variant
    worktreePath: string
    worktreeName: string
    cleanup: () => Promise<void>
  }>,
): Promise<ParticipantResult[]> {
  if (config.parallelExecution) {
    // Execute in parallel with batching to respect MAX_PARALLEL_AGENTS
    const { runAgentsInParallel } = await import('./security/limits.js')
    return runAgentsInParallel(variantEnvironments, (env) => executeParticipant(env, config, baseDir))
  } else {
    // Execute sequentially
    const results: ParticipantResult[] = []
    for (const env of variantEnvironments) {
      const result = await executeParticipant(env, config, baseDir)
      results.push(result)
    }
    return results
  }
}

/**
 * Execute a single participant in their pre-created worktree
 */
async function executeParticipant(
  env: {
    variant: Variant
    worktreePath: string
    worktreeName: string
    cleanup: () => Promise<void>
  },
  config: CompetitionConfig,
  baseDir: string,
): Promise<ParticipantResult> {
  console.log(`  Running: ${env.variant.name}...`)

  const runDir = join(baseDir, `run-${env.variant.id}`)
  mkdirSync(runDir, { recursive: true })

  // Log file for tool usage
  const toolLogPath = join(runDir, 'tool-usage.log')

  // Track tool usage via hooks
  const toolsUsed = new Set<string>()
  let toolCallCount = 0

  // Spawn agent directly in the pre-created worktree
  // Note: We use spawnAgent instead of spawnAgentWithVariant since worktree is already set up
  const { spawnAgent } = await import('../sdk/spawner.js')
  const agentResult = await spawnAgent({
    task: config.task.description,
    worktreePath: env.worktreePath,
    hooks: {
      onToolUse: (event: ToolUseEvent) => {
        // Track tool usage
        toolsUsed.add(event.tool_name)
        toolCallCount++

        // Log tool usage to file
        writeFileSync(toolLogPath, JSON.stringify(event) + '\n', { flag: 'a' })
      },
    },
    maxTurns: config.timeout ? Math.floor(config.timeout / 10) : 30,
  })

  // Save agent result
  writeFileSync(join(runDir, 'agent-result.json'), JSON.stringify(agentResult, null, 2))

  // Use the pre-created worktree path for evaluation
  const worktreePath = env.worktreePath

  // Evaluate the result
  const evaluation = await runSearchTaskEvaluation(config.task, worktreePath, runDir)

  console.log(`  Completed: ${env.variant.name} - Score: ${(evaluation.compositeScore * 100).toFixed(1)}%`)

  return {
    variantId: env.variant.id,
    variantName: env.variant.name,
    score: evaluation.compositeScore,
    evaluation,
    agentResult,
    toolsUsed: Array.from(toolsUsed),
    searchCount: evaluation.searchMetrics.searchCount,
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
 * Generate competition report with setup metrics
 */
function generateCompetitionReport(data: {
  competitionId: string
  task: SearchTask
  results: ParticipantResult[]
  winner: ParticipantResult
  metrics: CompetitionMetrics
  setupMetrics?: SetupMetrics
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

  // Add setup metrics section
  if (data.setupMetrics) {
    lines.push('SETUP METRICS')
    lines.push('-'.repeat(60))
    lines.push(`Total setup time: ${(data.setupMetrics.totalSetupTimeMs / 1000).toFixed(1)}s`)
    lines.push('')

    lines.push('Scan Results:')
    data.setupMetrics.scanResults.forEach((scan) => {
      lines.push(`  - ${scan.worktree}: ${scan.chunkCount} chunks in ${(scan.durationMs / 1000).toFixed(1)}s`)
    })
    lines.push('')

    lines.push('Validation Results:')
    data.setupMetrics.validationResults.forEach((val) => {
      lines.push(`  - ${val.variantId}: ${val.overall}`)
    })
    lines.push('')
  }

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
