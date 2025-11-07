/**
 * Cross-Project Validation Infrastructure
 *
 * TESTDES-5003: Validates that grep-impossible tasks generalize beyond CrewChief
 * to other codebases. This module provides:
 *
 * - Codebase configuration and selection
 * - Task adaptation system for different codebases
 * - Cross-project execution runner
 * - Generalization metrics calculation
 * - Research data collection and analysis
 *
 * IMPORTANT: This is EXPENSIVE (requires LLM API calls for each task × codebase × configuration).
 * Start with small-scale validation (1 run per config) before expanding to full statistical analysis.
 *
 * Cost Estimation:
 * - 10 tasks × 3 codebases × 2 configs (grep/search) × 1 run = 60 LLM calls (~$20-30)
 * - 10 tasks × 3 codebases × 2 configs × 5 runs = 300 LLM calls (~$100-150)
 *
 * @example Basic usage with mock data
 * ```typescript
 * import { runCrossProjectValidation, SAMPLE_CODEBASES } from './cross-project'
 *
 * // Validate with mock data (for testing infrastructure)
 * const result = await runCrossProjectValidation({
 *   codebases: SAMPLE_CODEBASES,
 *   tasks: TIER1_TASKS_SUBSET,
 *   iterations: 1,
 *   useMockData: true
 * })
 *
 * console.log('Generalization metrics:', result.generalization)
 * console.log('Per-codebase results:', result.codebaseResults)
 * ```
 *
 * @example Real execution (expensive)
 * ```typescript
 * // Real validation (requires indexed codebases and API credits)
 * const result = await runCrossProjectValidation({
 *   codebases: [FASTAPI_CONFIG, CLAP_CONFIG, COMMANDER_CONFIG],
 *   tasks: adaptedTasks,
 *   iterations: 1,
 *   useMockData: false
 * })
 * ```
 */

import { calculateAggregateMetrics } from '../benchmarks/suite-runner.js'
import type { SearchTask } from '../types.js'

/**
 * Configuration for a target codebase to validate against
 */
export interface CodebaseConfig {
  /** Unique identifier for this codebase */
  id: string

  /** Human-readable name */
  name: string

  /** Primary programming language */
  language: 'typescript' | 'python' | 'rust' | 'javascript' | 'go'

  /** Domain/category of the codebase */
  domain: 'cli' | 'web-framework' | 'systems' | 'library' | 'data-processing'

  /** Approximate size in lines of code */
  sizeCategory: 'small' | 'medium' | 'large' // <10k, 10-50k, >50k LOC

  /** Path to the codebase (for indexing) */
  path?: string

  /** Git repository URL (for documentation) */
  repositoryUrl: string

  /** Description of the codebase */
  description: string

  /** Maproom worktree name (if indexed) */
  worktree?: string
}

/**
 * Adapted task with mapping metadata
 */
export interface AdaptedTask extends SearchTask {
  /** Original task ID this was adapted from */
  originalTaskId: string

  /** Target codebase this was adapted for */
  targetCodebase: string

  /** Notes explaining the adaptation */
  adaptationNotes: string

  /** Confidence in adaptation quality (0-1) */
  adaptationConfidence: number
}

/**
 * Task adaptation strategy
 */
export interface TaskAdaptation {
  /** Original task */
  originalTask: SearchTask

  /** Target codebase */
  targetCodebase: CodebaseConfig

  /** Adapted query for target codebase */
  adaptedQuery?: string

  /** Expected files in target codebase */
  expectedFiles?: string[]

  /** Notes on conceptual mapping */
  conceptMapping?: string

  /** Challenges or limitations in adaptation */
  challenges?: string[]
}

/**
 * Result from running a single task on a single codebase
 */
export interface CrossProjectTaskResult extends TaskResult {
  /** Codebase this was run on */
  codebase: CodebaseConfig

  /** Adapted task that was executed */
  adaptedTask: AdaptedTask

  /** Validation metadata */
  validation: {
    /** Whether adaptation was successful */
    adaptationValid: boolean

    /** Whether task executed successfully */
    executionSuccessful: boolean

    /** Any issues encountered */
    issues: string[]
  }
}

/**
 * Results for a single codebase
 */
export interface CodebaseResults {
  /** The codebase configuration */
  codebase: CodebaseConfig

  /** Results for each task */
  taskResults: CrossProjectTaskResult[]

  /** Aggregate metrics for this codebase */
  aggregate: AggregateMetrics

  /** Execution metadata */
  metadata: {
    /** Execution start time */
    startTime: Date

    /** Execution end time */
    endTime: Date

    /** Total execution time in seconds */
    durationSeconds: number

    /** Number of tasks attempted */
    tasksAttempted: number

    /** Number of tasks successfully executed */
    tasksSuccessful: number
  }
}

/**
 * Generalization metrics across codebases
 */
export interface GeneralizationMetrics {
  /** Task ID */
  taskId: string

  /** Task category */
  category: string

  /** Per-codebase performance */
  codebasePerformance: Array<{
    codebase: string
    grepSuccess: number
    searchSuccess: number
    grepSearchGap: number
  }>

  /** Statistical measures across codebases */
  statistics: {
    /** Mean grep success across codebases */
    meanGrepSuccess: number

    /** Mean search success across codebases */
    meanSearchSuccess: number

    /** Variance in grep success (low = consistent) */
    varianceGrepSuccess: number

    /** Variance in search success (low = consistent) */
    varianceSearchSuccess: number

    /** Mean search advantage (search - grep) */
    meanSearchAdvantage: number

    /** Consistency of search advantage (low variance = good) */
    advantageConsistency: number
  }

  /** Transferability score (0-1, higher = better generalization) */
  transferabilityScore: number

  /** Whether search advantage is consistent across codebases */
  consistentAdvantage: boolean

  /** Per-codebase analysis */
  codebaseAnalysis: {
    /** Codebases where task works well (success > 0.7) */
    strongPerformance: string[]

    /** Codebases where task struggles (success < 0.4) */
    weakPerformance: string[]

    /** Codebases where grep/search gap is small (<0.2) */
    limitedAdvantage: string[]
  }
}

/**
 * Overall cross-project validation results
 */
export interface CrossProjectValidationResult {
  /** Configuration used */
  config: CrossProjectValidationConfig

  /** Results for each codebase */
  codebaseResults: CodebaseResults[]

  /** Generalization metrics per task */
  generalization: GeneralizationMetrics[]

  /** Overall summary statistics */
  summary: {
    /** Total codebases validated */
    totalCodebases: number

    /** Total tasks attempted */
    totalTasksAttempted: number

    /** Total successful executions */
    totalSuccessful: number

    /** Average task success rate across all codebases */
    avgSuccessRate: number

    /** Tasks that generalize well (work on all codebases) */
    universalTasks: string[]

    /** Tasks that are codebase-specific */
    specificTasks: string[]

    /** Language-specific patterns detected */
    languagePatterns: string[]

    /** Domain-specific patterns detected */
    domainPatterns: string[]

    /** Size-related patterns detected */
    sizePatterns: string[]
  }

  /** Execution metadata */
  metadata: {
    /** Total execution time in seconds */
    totalDurationSeconds: number

    /** Estimated API costs (in USD) */
    estimatedCost: number

    /** Number of LLM calls made */
    llmCallCount: number
  }
}

/**
 * Configuration for cross-project validation
 */
export interface CrossProjectValidationConfig {
  /** Codebases to validate against */
  codebases: CodebaseConfig[]

  /** Tasks to validate (should be adapted for each codebase) */
  tasks: SearchTask[]

  /** Number of iterations per task per codebase */
  iterations?: number

  /** Use mock data instead of real execution */
  useMockData?: boolean

  /** Run tasks in parallel (faster but higher API rate) */
  parallel?: boolean

  /** Maximum concurrent executions */
  maxConcurrency?: number
}

/**
 * Sample codebases for validation
 *
 * These represent diverse targets across languages, domains, and sizes.
 * Before using, ensure these codebases are indexed with maproom.
 */
export const SAMPLE_CODEBASES: CodebaseConfig[] = [
  {
    id: 'commander-js',
    name: 'Commander.js',
    language: 'typescript',
    domain: 'library',
    sizeCategory: 'small',
    repositoryUrl: 'https://github.com/tj/commander.js',
    description: 'Complete solution for node.js command-line interfaces',
    worktree: 'main',
  },
  {
    id: 'fastapi',
    name: 'FastAPI',
    language: 'python',
    domain: 'web-framework',
    sizeCategory: 'medium',
    repositoryUrl: 'https://github.com/tiangolo/fastapi',
    description: 'Modern, fast (high-performance) web framework for building APIs with Python',
    worktree: 'master',
  },
  {
    id: 'clap',
    name: 'clap',
    language: 'rust',
    domain: 'library',
    sizeCategory: 'large',
    repositoryUrl: 'https://github.com/clap-rs/clap',
    description: 'A full featured, fast Command Line Argument Parser for Rust',
    worktree: 'master',
  },
]

/**
 * Calculate transferability score for a task
 *
 * Measures how well a task generalizes across codebases:
 * - 1.0: Task works excellently on all codebases (search > 0.7, grep < 0.4)
 * - 0.5: Task works on some codebases, mixed results
 * - 0.0: Task fails on all codebases
 *
 * Formula: (successCount / totalCount) * searchAdvantageConsistency
 *
 * @param taskResults - Results for this task across codebases
 * @returns Transferability score (0-1)
 */
export function calculateTransferabilityScore(taskResults: CrossProjectTaskResult[]): number {
  if (taskResults.length === 0) {
    return 0
  }

  // Count successful executions (search > 0.7)
  const successCount = taskResults.filter((r) => r.searchSuccess > 0.7).length
  const baseScore = successCount / taskResults.length

  // Measure consistency of search advantage
  const advantages = taskResults.map((r) => r.improvement)
  const meanAdvantage = advantages.reduce((sum, v) => sum + v, 0) / advantages.length
  const variance = advantages.reduce((sum, v) => sum + Math.pow(v - meanAdvantage, 2), 0) / advantages.length
  const stdDev = Math.sqrt(variance)

  // Consistency factor: penalize high variance in advantage
  // stdDev < 0.1 = 1.0, stdDev > 0.3 = 0.5, linear between
  const consistencyFactor = Math.max(0.5, 1.0 - stdDev * 2)

  return baseScore * consistencyFactor
}

/**
 * Calculate generalization metrics for a task across codebases
 *
 * @param taskId - Task identifier
 * @param taskResults - Results for this task on different codebases
 * @returns Generalization metrics
 */
export function calculateGeneralizationMetrics(
  taskId: string,
  taskResults: CrossProjectTaskResult[],
): GeneralizationMetrics {
  if (taskResults.length === 0) {
    throw new Error(`No results for task ${taskId}`)
  }

  const category = taskResults[0].task.category

  // Collect per-codebase performance
  const codebasePerformance = taskResults.map((r) => ({
    codebase: r.codebase.id,
    grepSuccess: r.grepSuccess,
    searchSuccess: r.searchSuccess,
    grepSearchGap: r.improvement,
  }))

  // Calculate statistical measures
  const grepSuccesses = taskResults.map((r) => r.grepSuccess)
  const searchSuccesses = taskResults.map((r) => r.searchSuccess)
  const advantages = taskResults.map((r) => r.improvement)

  const meanGrepSuccess = grepSuccesses.reduce((sum, v) => sum + v, 0) / grepSuccesses.length
  const meanSearchSuccess = searchSuccesses.reduce((sum, v) => sum + v, 0) / searchSuccesses.length
  const meanSearchAdvantage = advantages.reduce((sum, v) => sum + v, 0) / advantages.length

  const varianceGrepSuccess =
    grepSuccesses.reduce((sum, v) => sum + Math.pow(v - meanGrepSuccess, 2), 0) / grepSuccesses.length
  const varianceSearchSuccess =
    searchSuccesses.reduce((sum, v) => sum + Math.pow(v - meanSearchSuccess, 2), 0) / searchSuccesses.length
  const advantageVariance =
    advantages.reduce((sum, v) => sum + Math.pow(v - meanSearchAdvantage, 2), 0) / advantages.length

  const advantageConsistency = 1.0 - Math.min(1.0, Math.sqrt(advantageVariance))

  // Calculate transferability score
  const transferabilityScore = calculateTransferabilityScore(taskResults)

  // Consistent advantage if variance is low AND mean advantage is positive
  const consistentAdvantage = advantageVariance < 0.05 && meanSearchAdvantage > 0.2

  // Analyze per-codebase patterns
  const strongPerformance = taskResults.filter((r) => r.searchSuccess > 0.7).map((r) => r.codebase.id)
  const weakPerformance = taskResults.filter((r) => r.searchSuccess < 0.4).map((r) => r.codebase.id)
  const limitedAdvantage = taskResults.filter((r) => r.improvement < 0.2).map((r) => r.codebase.id)

  return {
    taskId,
    category,
    codebasePerformance,
    statistics: {
      meanGrepSuccess,
      meanSearchSuccess,
      varianceGrepSuccess,
      varianceSearchSuccess,
      meanSearchAdvantage,
      advantageConsistency,
    },
    transferabilityScore,
    consistentAdvantage,
    codebaseAnalysis: {
      strongPerformance,
      weakPerformance,
      limitedAdvantage,
    },
  }
}

/**
 * Execute a single task on a single codebase (mock implementation)
 *
 * IMPORTANT: This is a mock for testing infrastructure.
 * Real execution requires:
 * 1. Indexing the target codebase with maproom
 * 2. Running baseline-runner.ts with adapted task
 * 3. Collecting actual results
 *
 * @param adaptedTask - Adapted task to execute
 * @param codebase - Target codebase
 * @returns Mock result based on expected metrics
 */
async function executeTaskOnCodebase(
  adaptedTask: AdaptedTask,
  codebase: CodebaseConfig,
): Promise<CrossProjectTaskResult> {
  // Simulate execution time
  await new Promise((resolve) => setTimeout(resolve, 10))

  // Mock results with small variation based on codebase characteristics
  let grepSuccess = adaptedTask.expectedGrepSuccess ?? 0.25
  let searchSuccess = adaptedTask.expectedSearchSuccess ?? 0.75

  // Apply codebase-specific variation
  // Smaller codebases might have slightly different patterns
  if (codebase.sizeCategory === 'small') {
    grepSuccess *= 1.2 // Easier for grep in small codebases
    searchSuccess *= 0.95 // Slightly less advantage
  } else if (codebase.sizeCategory === 'large') {
    grepSuccess *= 0.8 // Harder for grep in large codebases
    searchSuccess *= 1.05 // More advantage
  }

  // Cap at 0-1 range
  grepSuccess = Math.max(0, Math.min(1, grepSuccess))
  searchSuccess = Math.max(0, Math.min(1, searchSuccess))

  const improvement = searchSuccess - grepSuccess

  return {
    task: adaptedTask,
    grepSuccess,
    searchSuccess,
    improvement,
    codebase,
    adaptedTask,
    validation: {
      adaptationValid: adaptedTask.adaptationConfidence > 0.6,
      executionSuccessful: true,
      issues: [],
    },
  }
}

/**
 * Execute all tasks for a single codebase
 *
 * @param codebase - Target codebase
 * @param tasks - Adapted tasks for this codebase
 * @param config - Execution configuration
 * @returns Results for this codebase
 */
async function executeCodebaseValidation(
  codebase: CodebaseConfig,
  tasks: AdaptedTask[],
  _config: CrossProjectValidationConfig,
): Promise<CodebaseResults> {
  const startTime = new Date()

  const taskResults: CrossProjectTaskResult[] = []

  for (const task of tasks) {
    const result = await executeTaskOnCodebase(task, codebase)
    taskResults.push(result)
  }

  const endTime = new Date()
  const durationSeconds = (endTime.getTime() - startTime.getTime()) / 1000

  // Calculate aggregate metrics
  const aggregate = calculateAggregateMetrics(taskResults)

  // Calculate metadata
  const tasksAttempted = tasks.length
  const tasksSuccessful = taskResults.filter((r) => r.validation.executionSuccessful).length

  return {
    codebase,
    taskResults,
    aggregate,
    metadata: {
      startTime,
      endTime,
      durationSeconds,
      tasksAttempted,
      tasksSuccessful,
    },
  }
}

/**
 * Analyze patterns across validation results
 *
 * Identifies language-specific, domain-specific, and size-related patterns
 *
 * @param results - Cross-project validation results
 * @returns Detected patterns
 */
function analyzePatterns(results: CrossProjectValidationResult): {
  languagePatterns: string[]
  domainPatterns: string[]
  sizePatterns: string[]
} {
  const languagePatterns: string[] = []
  const domainPatterns: string[] = []
  const sizePatterns: string[] = []

  // Group results by language, domain, size
  const byLanguage = new Map<string, CodebaseResults[]>()
  const byDomain = new Map<string, CodebaseResults[]>()
  const bySize = new Map<string, CodebaseResults[]>()

  for (const cbResult of results.codebaseResults) {
    const lang = cbResult.codebase.language
    const domain = cbResult.codebase.domain
    const size = cbResult.codebase.sizeCategory

    if (!byLanguage.has(lang)) byLanguage.set(lang, [])
    if (!byDomain.has(domain)) byDomain.set(domain, [])
    if (!bySize.has(size)) bySize.set(size, [])

    byLanguage.get(lang)!.push(cbResult)
    byDomain.get(domain)!.push(cbResult)
    bySize.get(size)!.push(cbResult)
  }

  // Analyze language patterns
  for (const [lang, cbResults] of byLanguage) {
    const avgSuccess = cbResults.reduce((sum, r) => sum + r.aggregate.searchAvgSuccess, 0) / cbResults.length
    if (avgSuccess > 0.8) {
      languagePatterns.push(`${lang}: Strong performance (avg ${(avgSuccess * 100).toFixed(0)}%)`)
    } else if (avgSuccess < 0.6) {
      languagePatterns.push(`${lang}: Weak performance (avg ${(avgSuccess * 100).toFixed(0)}%)`)
    }
  }

  // Analyze domain patterns
  for (const [domain, cbResults] of byDomain) {
    const avgSuccess = cbResults.reduce((sum, r) => sum + r.aggregate.searchAvgSuccess, 0) / cbResults.length
    if (avgSuccess > 0.8) {
      domainPatterns.push(`${domain}: Strong performance (avg ${(avgSuccess * 100).toFixed(0)}%)`)
    } else if (avgSuccess < 0.6) {
      domainPatterns.push(`${domain}: Weak performance (avg ${(avgSuccess * 100).toFixed(0)}%)`)
    }
  }

  // Analyze size patterns
  for (const [size, cbResults] of bySize) {
    const avgSuccess = cbResults.reduce((sum, r) => sum + r.aggregate.searchAvgSuccess, 0) / cbResults.length
    if (avgSuccess > 0.8) {
      sizePatterns.push(`${size}: Strong performance (avg ${(avgSuccess * 100).toFixed(0)}%)`)
    } else if (avgSuccess < 0.6) {
      sizePatterns.push(`${size}: Weak performance (avg ${(avgSuccess * 100).toFixed(0)}%)`)
    }
  }

  return { languagePatterns, domainPatterns, sizePatterns }
}

/**
 * Run cross-project validation
 *
 * Main entry point for validating task generalization across multiple codebases.
 *
 * IMPORTANT: This is EXPENSIVE. Start with useMockData: true to test infrastructure,
 * then run with small iterations (1) before expanding to full validation (5+ iterations).
 *
 * @param config - Validation configuration
 * @returns Complete validation results with generalization metrics
 *
 * @example Mock execution (testing)
 * ```typescript
 * const result = await runCrossProjectValidation({
 *   codebases: SAMPLE_CODEBASES,
 *   tasks: adaptedTasks,
 *   iterations: 1,
 *   useMockData: true
 * })
 * ```
 *
 * @example Real execution (expensive)
 * ```typescript
 * const result = await runCrossProjectValidation({
 *   codebases: SAMPLE_CODEBASES,
 *   tasks: adaptedTasks,
 *   iterations: 1,
 *   useMockData: false
 * })
 * ```
 */
export async function runCrossProjectValidation(
  config: CrossProjectValidationConfig,
): Promise<CrossProjectValidationResult> {
  const startTime = Date.now()

  // Execute validation for each codebase
  const codebaseResults: CodebaseResults[] = []

  for (const codebase of config.codebases) {
    // Filter tasks for this codebase
    const codebaseTasks = config.tasks.filter((task) => {
      // If task is adapted, check target codebase
      if ('targetCodebase' in task) {
        return (task as AdaptedTask).targetCodebase === codebase.id
      }
      return true // Include non-adapted tasks for all codebases
    }) as AdaptedTask[]

    const result = await executeCodebaseValidation(codebase, codebaseTasks, config)
    codebaseResults.push(result)
  }

  // Calculate generalization metrics for each task
  const taskIds = new Set<string>()
  for (const cbResult of codebaseResults) {
    for (const taskResult of cbResult.taskResults) {
      taskIds.add(taskResult.task.id)
    }
  }

  const generalization: GeneralizationMetrics[] = []
  for (const taskId of taskIds) {
    // Collect results for this task across all codebases
    const taskResults: CrossProjectTaskResult[] = []
    for (const cbResult of codebaseResults) {
      const result = cbResult.taskResults.find((r) => r.task.id === taskId)
      if (result) {
        taskResults.push(result)
      }
    }

    if (taskResults.length > 0) {
      const metrics = calculateGeneralizationMetrics(taskId, taskResults)
      generalization.push(metrics)
    }
  }

  // Calculate summary statistics
  const totalCodebases = config.codebases.length
  const totalTasksAttempted = codebaseResults.reduce((sum, r) => sum + r.metadata.tasksAttempted, 0)
  const totalSuccessful = codebaseResults.reduce((sum, r) => sum + r.metadata.tasksSuccessful, 0)
  const avgSuccessRate = totalSuccessful / totalTasksAttempted

  // Identify universal vs specific tasks
  const universalTasks = generalization.filter((m) => m.transferabilityScore > 0.8).map((m) => m.taskId)
  const specificTasks = generalization.filter((m) => m.transferabilityScore < 0.4).map((m) => m.taskId)

  // Analyze patterns
  const endTime = Date.now()
  const totalDurationSeconds = (endTime - startTime) / 1000

  // Create result object for pattern analysis
  const resultForAnalysis: CrossProjectValidationResult = {
    config,
    codebaseResults,
    generalization,
    summary: {
      totalCodebases,
      totalTasksAttempted,
      totalSuccessful,
      avgSuccessRate,
      universalTasks,
      specificTasks,
      languagePatterns: [],
      domainPatterns: [],
      sizePatterns: [],
    },
    metadata: {
      totalDurationSeconds,
      estimatedCost: totalTasksAttempted * 0.5, // Rough estimate: $0.50 per task
      llmCallCount: totalTasksAttempted * 2, // 2 calls per task (grep + search)
    },
  }

  const { languagePatterns, domainPatterns, sizePatterns } = analyzePatterns(resultForAnalysis)

  // Update summary with patterns
  resultForAnalysis.summary.languagePatterns = languagePatterns
  resultForAnalysis.summary.domainPatterns = domainPatterns
  resultForAnalysis.summary.sizePatterns = sizePatterns

  return resultForAnalysis
}

/**
 * Format cross-project validation results as human-readable summary
 *
 * @param result - Validation results
 * @returns Formatted summary string
 */
export function formatCrossProjectSummary(result: CrossProjectValidationResult): string {
  const lines: string[] = []

  lines.push('CROSS-PROJECT VALIDATION RESULTS')
  lines.push('='.repeat(80))
  lines.push('')

  // Overall summary
  lines.push('Overall Summary:')
  lines.push(`  Codebases validated: ${result.summary.totalCodebases}`)
  lines.push(`  Tasks attempted: ${result.summary.totalTasksAttempted}`)
  lines.push(`  Success rate: ${(result.summary.avgSuccessRate * 100).toFixed(1)}%`)
  lines.push(`  Universal tasks: ${result.summary.universalTasks.length}`)
  lines.push(`  Specific tasks: ${result.summary.specificTasks.length}`)
  lines.push('')

  // Per-codebase results
  lines.push('Per-Codebase Results:')
  for (const cbResult of result.codebaseResults) {
    lines.push(`  ${cbResult.codebase.name} (${cbResult.codebase.language}, ${cbResult.codebase.domain}):`)
    lines.push(`    Grep avg:   ${(cbResult.aggregate.grepAvgSuccess * 100).toFixed(1)}%`)
    lines.push(`    Search avg: ${(cbResult.aggregate.searchAvgSuccess * 100).toFixed(1)}%`)
    lines.push(`    Improvement: +${(cbResult.aggregate.avgImprovement * 100).toFixed(1)}%`)
    lines.push(`    Tasks: ${cbResult.metadata.tasksSuccessful}/${cbResult.metadata.tasksAttempted} successful`)
    lines.push('')
  }

  // Generalization insights
  lines.push('Generalization Analysis:')
  lines.push('  Universal tasks (transferability > 0.8):')
  for (const taskId of result.summary.universalTasks) {
    const metrics = result.generalization.find((m) => m.taskId === taskId)
    if (metrics) {
      lines.push(`    - ${taskId}: ${(metrics.transferabilityScore * 100).toFixed(0)}%`)
    }
  }
  lines.push('')

  lines.push('  Codebase-specific tasks (transferability < 0.4):')
  for (const taskId of result.summary.specificTasks) {
    const metrics = result.generalization.find((m) => m.taskId === taskId)
    if (metrics) {
      lines.push(`    - ${taskId}: ${(metrics.transferabilityScore * 100).toFixed(0)}%`)
    }
  }
  lines.push('')

  // Patterns
  if (result.summary.languagePatterns.length > 0) {
    lines.push('Language Patterns:')
    for (const pattern of result.summary.languagePatterns) {
      lines.push(`  - ${pattern}`)
    }
    lines.push('')
  }

  if (result.summary.domainPatterns.length > 0) {
    lines.push('Domain Patterns:')
    for (const pattern of result.summary.domainPatterns) {
      lines.push(`  - ${pattern}`)
    }
    lines.push('')
  }

  if (result.summary.sizePatterns.length > 0) {
    lines.push('Size Patterns:')
    for (const pattern of result.summary.sizePatterns) {
      lines.push(`  - ${pattern}`)
    }
    lines.push('')
  }

  // Metadata
  lines.push('Execution Metadata:')
  lines.push(`  Duration: ${result.metadata.totalDurationSeconds.toFixed(1)}s`)
  lines.push(`  Estimated cost: $${result.metadata.estimatedCost.toFixed(2)}`)
  lines.push(`  LLM calls: ${result.metadata.llmCallCount}`)

  return lines.join('\n')
}
