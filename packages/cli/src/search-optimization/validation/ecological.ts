/**
 * Ecological Validation - Ensures tasks reflect real-world developer activities
 *
 * This module validates that search tasks are realistic and based on actual
 * developer scenarios through automated realism checks, frequency classification,
 * and a developer survey framework.
 *
 * Key Features:
 * - Automated realism checks (scenario type, objectivity, coercion)
 * - Task frequency classification (daily/weekly/monthly/rare)
 * - Composite ecological scoring (0-1)
 * - Developer survey framework (optional enhancement)
 * - Actionable recommendations for improving task realism
 */

import type { SearchTask, SearchTarget } from '../types.js'

/**
 * Scenario types reflecting real developer activities
 */
export type ScenarioType = 'code-review' | 'debugging' | 'refactoring' | 'onboarding' | 'maintenance'

/**
 * Task frequency classifications
 */
export type TaskFrequency = 'daily' | 'weekly' | 'monthly' | 'rare'

/**
 * Comprehensive ecological validity checks
 */
export interface EcologicalChecks {
  /** Is the task based on a real-world scenario? */
  basedOnRealScenario: boolean

  /** Type of scenario (if real) */
  scenarioType?: ScenarioType

  /** Reference to source scenario (e.g., "GitHub issue #123") */
  scenarioReference?: string

  /** How often developers encounter this task */
  frequency: TaskFrequency

  /** Justification for frequency classification */
  frequencyJustification: string

  /** Does the task have objective success criteria? */
  objectiveSuccessCriteria: boolean

  /** Is the task free of subjective judgment requirements? */
  noSubjectiveJudgment: boolean

  /** Does the task have deterministic outcomes? */
  deterministicOutcome: boolean

  /** Does the description avoid hinting at specific tools? */
  noCoercion: boolean

  /** Are there multiple valid approaches to complete the task? */
  multipleValidApproaches: boolean

  /** Is the task clear without tool hints? */
  clearWithoutToolHint: boolean

  /** Optional survey data from real developers */
  surveyResults?: SurveyResults
}

/**
 * Result of ecological validation for a task
 */
export interface EcologicalValidationResult {
  /** Task that was validated */
  task: SearchTask

  /** All ecological checks performed */
  checks: EcologicalChecks

  /** Overall pass/fail */
  passed: boolean

  /** Composite ecological score (0-1) */
  score: number

  /** Actionable recommendations for improvement */
  recommendations: string[]

  /** Reasons for failure (if failed) */
  failureReasons?: string[]
}

/**
 * Task frequency classification with priority weighting
 */
export interface FrequencyClassification {
  /** Frequency category */
  frequency: TaskFrequency

  /** Priority weight (1.0 for daily, 0.1 for rare) */
  priority: number

  /** Justification for classification */
  justification: string
}

/**
 * Developer survey response
 */
export interface DeveloperSurvey {
  /** Unique task identifier */
  taskId: string

  /** Human-readable task description */
  taskDescription: string

  /** Survey responses */
  questions: {
    /** Would the developer actually do this task? */
    wouldActuallyDo: boolean

    /** How often would they do it? */
    howOften: 'daily' | 'weekly' | 'monthly' | 'rarely' | 'never'

    /** Realism rating (1-5, 5 = very realistic) */
    isRealistic: 1 | 2 | 3 | 4 | 5

    /** Would this task help them? */
    wouldHelpMe: boolean

    /** Free-form comments */
    comments?: string
  }

  /** Respondent information */
  respondent: {
    /** Developer role (e.g., "Senior Engineer") */
    role: string

    /** Years of experience */
    experience: number

    /** Codebase size they work with */
    codebaseSize: 'small' | 'medium' | 'large' | 'very-large'
  }
}

/**
 * Aggregated survey results
 */
export interface SurveyResults {
  /** Number of respondents */
  respondents: number

  /** Percentage who would actually do this task */
  wouldActuallyDo: number

  /** Average frequency (e.g., "weekly") */
  averageFrequency: string

  /** Average realism score (1-5) */
  realismScore: number

  /** Collected comments */
  comments: string[]
}

/**
 * Frequency priority mapping
 */
const FREQUENCY_PRIORITY: Record<TaskFrequency, { priority: number; description: string; examplesPerYear: number }> = {
  daily: {
    priority: 1.0,
    description: 'Multiple times per day',
    examplesPerYear: 250, // Work days
  },
  weekly: {
    priority: 0.7,
    description: 'Once or twice per week',
    examplesPerYear: 75,
  },
  monthly: {
    priority: 0.4,
    description: 'Once or twice per month',
    examplesPerYear: 18,
  },
  rare: {
    priority: 0.1,
    description: 'Few times per year',
    examplesPerYear: 4,
  },
}

/**
 * Subjective words that indicate non-objective criteria
 */
const SUBJECTIVE_WORDS = [
  'good',
  'bad',
  'better',
  'best',
  'thorough',
  'comprehensive',
  'quality',
  'clean',
  'elegant',
  'simple',
  'complex',
  'appropriate',
  'proper',
  'suitable',
]

/**
 * Tool hint phrases that coerce specific approaches
 */
const TOOL_HINTS = [
  'use semantic search',
  'use grep',
  'use maproom',
  'search semantically',
  'keyword search',
  'use the search tool',
  'with semantic',
  'with grep',
]

/**
 * Validate ecological validity of a search task
 *
 * Performs comprehensive realism checks including:
 * - Real scenario validation
 * - Frequency classification
 * - Objectivity checks
 * - Coercion detection
 * - Survey integration (if available)
 *
 * @param task - The task to validate
 * @returns Ecological validation result with score and recommendations
 */
export function validateEcologicalValidity(task: SearchTask): EcologicalValidationResult {
  // Perform all ecological checks
  const checks = validateRealism(task)

  // Calculate composite score
  const score = calculateEcologicalScore(checks)

  // Pass threshold: score >= 0.6
  const passed = score >= 0.6

  // Generate recommendations
  const recommendations = generateEcologicalRecommendations({ task, checks, score, passed, recommendations: [] })

  // Identify failure reasons if failed
  const failureReasons: string[] = []
  if (!passed) {
    if (!checks.basedOnRealScenario) {
      failureReasons.push('Not explicitly marked as based on real scenario')
    }
    if (checks.frequency === 'rare') {
      failureReasons.push('Task is rarely encountered by developers')
    }
    if (!checks.objectiveSuccessCriteria) {
      failureReasons.push('Success criteria are not objective')
    }
    if (!checks.noCoercion) {
      failureReasons.push('Task description hints at specific tools')
    }
    if (checks.surveyResults && checks.surveyResults.realismScore < 3) {
      failureReasons.push('Low realism score from developer surveys')
    }
  }

  return {
    task,
    checks,
    passed,
    score,
    recommendations,
    failureReasons: failureReasons.length > 0 ? failureReasons : undefined,
  }
}

/**
 * Classify task frequency based on metadata and characteristics
 *
 * Examines task metadata, scenario type, and description to determine
 * how often developers would encounter this task in practice.
 *
 * @param task - The task to classify
 * @returns Frequency classification with priority and justification
 */
export function classifyTaskFrequency(task: SearchTask): FrequencyClassification {
  const metadata = task as SearchTask & Record<string, unknown>

  // Check explicit frequency metadata
  if (metadata.frequency) {
    const freq = metadata.frequency as TaskFrequency
    const priorityInfo = FREQUENCY_PRIORITY[freq]
    return {
      frequency: freq,
      priority: priorityInfo.priority,
      justification: metadata.frequencyJustification || `Explicitly marked as ${freq} task`,
    }
  }

  // Infer from scenario type
  if (metadata.scenarioType) {
    const scenarioFrequencies: Record<ScenarioType, TaskFrequency> = {
      'code-review': 'daily',
      debugging: 'weekly',
      refactoring: 'monthly',
      onboarding: 'rare',
      maintenance: 'weekly',
    }

    const freq = scenarioFrequencies[metadata.scenarioType as ScenarioType]
    const priorityInfo = FREQUENCY_PRIORITY[freq]
    return {
      frequency: freq,
      priority: priorityInfo.priority,
      justification: `Typical frequency for ${metadata.scenarioType} scenarios`,
    }
  }

  // Infer from category
  const categoryFrequencies: Record<string, TaskFrequency> = {
    'relationship-discovery': 'weekly',
    'architectural-understanding': 'monthly',
    'cross-cutting-concerns': 'weekly',
    'negative-space': 'monthly',
    'implicit-knowledge': 'rare',
  }

  const freq = categoryFrequencies[task.category] || 'rare'
  const priorityInfo = FREQUENCY_PRIORITY[freq]

  return {
    frequency: freq,
    priority: priorityInfo.priority,
    justification: `Inferred from category: ${task.category}`,
  }
}

/**
 * Perform all realism checks on a task
 *
 * Checks include:
 * - Real scenario validation (from metadata)
 * - Scenario type classification
 * - Frequency classification
 * - Objective success criteria (no subjective words)
 * - Deterministic outcomes (from validator type)
 * - No tool coercion (description doesn't hint at tools)
 * - Multiple valid approaches
 * - Clarity without hints
 *
 * @param task - The task to validate
 * @returns Complete ecological checks
 */
export function validateRealism(task: SearchTask): EcologicalChecks {
  const metadata = task as SearchTask & Record<string, unknown>

  // Check 1: Based on real scenario?
  const basedOnRealScenario = metadata.basedOnRealScenario === true

  // Check 2: Scenario type
  const scenarioType = metadata.scenarioType as ScenarioType | undefined

  // Check 3: Scenario reference
  const scenarioReference = metadata.scenarioReference as string | undefined

  // Check 4: Frequency classification
  const frequencyClass = classifyTaskFrequency(task)
  const frequency = frequencyClass.frequency
  const frequencyJustification = frequencyClass.justification

  // Check 5: Objective success criteria
  const descriptionLower = task.description.toLowerCase()
  const promptLower = task.followUpTask.prompt.toLowerCase()
  const hasSubjectiveWords =
    SUBJECTIVE_WORDS.some((word) => descriptionLower.includes(word)) ||
    SUBJECTIVE_WORDS.some((word) => promptLower.includes(word))
  const objectiveSuccessCriteria = !hasSubjectiveWords

  // Check 6: No subjective judgment (same as objective criteria)
  const noSubjectiveJudgment = objectiveSuccessCriteria

  // Check 7: Deterministic outcome (based on validator type)
  const validatorType = task.followUpTask.validator.type
  const deterministicOutcome = validatorType !== 'explanation'

  // Check 8: No coercion (description doesn't hint at tools)
  const hasToolHints = TOOL_HINTS.some((hint) => descriptionLower.includes(hint) || promptLower.includes(hint))
  const noCoercion = !hasToolHints

  // Check 9: Multiple valid approaches
  // Tasks with pattern-based validators or explanation validators allow flexibility
  const multipleValidApproaches =
    validatorType === 'explanation' ||
    task.searchTarget.type === 'pattern' ||
    !!(task.searchTarget as SearchTarget & Record<string, unknown>).alternatives

  // Check 10: Clear without tool hint
  // Task should have sufficient context in description
  const clearWithoutToolHint = task.description.length > 50 && (task.context?.length || 0) > 0

  // Check 11: Survey results (if available)
  const surveyResults = metadata.surveyResults as SurveyResults | undefined

  return {
    basedOnRealScenario,
    scenarioType,
    scenarioReference,
    frequency,
    frequencyJustification,
    objectiveSuccessCriteria,
    noSubjectiveJudgment,
    deterministicOutcome,
    noCoercion,
    multipleValidApproaches,
    clearWithoutToolHint,
    surveyResults,
  }
}

/**
 * Calculate composite ecological score from checks
 *
 * Scoring weights:
 * - basedOnRealScenario: 30%
 * - frequency (higher = better): 25%
 * - objectiveSuccessCriteria: 20%
 * - noCoercion: 15%
 * - surveyResults (if available): 10%
 *
 * Pass threshold: 0.6
 *
 * @param checks - Ecological checks to score
 * @returns Composite score (0-1)
 */
export function calculateEcologicalScore(checks: EcologicalChecks): number {
  let score = 0

  // Weight 1: Real scenario (30%)
  if (checks.basedOnRealScenario) {
    score += 0.3
  } else {
    // Partial credit for long, concrete description
    if (checks.clearWithoutToolHint) {
      score += 0.15
    }
  }

  // Weight 2: Frequency (25%)
  const frequencyPriority = FREQUENCY_PRIORITY[checks.frequency].priority
  score += 0.25 * frequencyPriority

  // Weight 3: Objective criteria (20%)
  if (checks.objectiveSuccessCriteria) {
    score += 0.2
  } else if (checks.deterministicOutcome) {
    // Partial credit for deterministic outcome
    score += 0.1
  }

  // Weight 4: No coercion (15%)
  if (checks.noCoercion) {
    score += 0.15
  }

  // Weight 5: Survey results (10%, if available)
  if (checks.surveyResults) {
    const surveyScore = calculateSurveyScore(checks.surveyResults)
    score += 0.1 * surveyScore
  } else {
    // No survey data = neutral (0.05 baseline)
    score += 0.05
  }

  return Math.min(1.0, score)
}

/**
 * Calculate score from survey results
 *
 * @param results - Survey results to score
 * @returns Score (0-1)
 */
function calculateSurveyScore(results: SurveyResults): number {
  let score = 0

  // Would actually do: 40%
  score += 0.4 * results.wouldActuallyDo

  // Realism score (1-5 → 0-1): 60%
  score += 0.6 * ((results.realismScore - 1) / 4)

  return score
}

/**
 * Generate actionable recommendations for improving ecological validity
 *
 * @param result - Ecological validation result
 * @returns Array of specific, actionable recommendations
 */
export function generateEcologicalRecommendations(result: EcologicalValidationResult): string[] {
  const recommendations: string[] = []
  const { checks } = result

  // Real scenario
  if (!checks.basedOnRealScenario) {
    recommendations.push(
      "Mark task with 'basedOnRealScenario: true' if it comes from actual developer experience. Include scenarioType and scenarioReference for context.",
    )
  }

  // Frequency
  if (checks.frequency === 'rare') {
    recommendations.push(
      `Task is classified as rare (${FREQUENCY_PRIORITY.rare.examplesPerYear} times/year). Consider if this represents a common enough scenario. If it's actually more frequent, add 'frequency' metadata.`,
    )
  }

  // Objectivity
  if (!checks.objectiveSuccessCriteria) {
    const subjectiveWords = SUBJECTIVE_WORDS.filter(
      (word) =>
        result.task.description.toLowerCase().includes(word) ||
        result.task.followUpTask.prompt.toLowerCase().includes(word),
    )
    recommendations.push(
      `Remove subjective words from description: ${subjectiveWords.join(', ')}. Use measurable criteria instead.`,
    )
  }

  // Coercion
  if (!checks.noCoercion) {
    const hints = TOOL_HINTS.filter(
      (hint) =>
        result.task.description.toLowerCase().includes(hint) ||
        result.task.followUpTask.prompt.toLowerCase().includes(hint),
    )
    recommendations.push(
      `Remove tool hints that bias approach: ${hints.join(', ')}. Let agent choose its own strategy.`,
    )
  }

  // Determinism
  if (!checks.deterministicOutcome) {
    recommendations.push(
      "Validator type 'explanation' is subjective. Consider using 'code_change' or 'file_creation' with pattern matching for more objective validation.",
    )
  }

  // Clarity
  if (!checks.clearWithoutToolHint) {
    if (result.task.description.length < 50) {
      recommendations.push('Description is too brief. Add more context about the scenario and goals.')
    }
    if (!result.task.context || result.task.context.length === 0) {
      recommendations.push('Add context field to explain when and why developers would do this task.')
    }
  }

  // Survey
  if (!checks.surveyResults) {
    recommendations.push(
      'Consider collecting developer survey data to validate realism (see docs/research/task-realism-survey.md).',
    )
  } else if (checks.surveyResults.realismScore < 3) {
    recommendations.push(
      `Low developer realism score (${checks.surveyResults.realismScore}/5). Review survey comments: ${checks.surveyResults.comments.slice(0, 2).join('; ')}`,
    )
  }

  // Success message
  if (result.passed) {
    recommendations.push('Task passes ecological validation. Well-designed realistic scenario with objective criteria.')
  }

  return recommendations
}

/**
 * Format ecological validation result as markdown report
 *
 * @param result - Validation result to format
 * @returns Formatted markdown report
 */
export function formatEcologicalReport(result: EcologicalValidationResult): string {
  const lines: string[] = []
  const { task, checks, passed, score } = result

  // Header
  lines.push('# Ecological Validation Report')
  lines.push('')
  lines.push(`**Task**: ${task.name} (${task.id})`)
  lines.push(`**Category**: ${task.category}`)
  lines.push(`**Status**: ${passed ? '✓ PASSED' : '✗ FAILED'}`)
  lines.push(`**Score**: ${(score * 100).toFixed(1)}% (threshold: 60%)`)
  lines.push('')

  // Checks
  lines.push('## Ecological Checks')
  lines.push('')

  const checkResults = [
    { name: 'Real Scenario', value: checks.basedOnRealScenario, details: checks.scenarioReference },
    { name: 'Scenario Type', value: checks.scenarioType || 'Not specified', details: null },
    {
      name: 'Frequency',
      value: checks.frequency,
      details: `${checks.frequencyJustification} (priority: ${FREQUENCY_PRIORITY[checks.frequency].priority})`,
    },
    { name: 'Objective Criteria', value: checks.objectiveSuccessCriteria, details: null },
    { name: 'No Subjective Judgment', value: checks.noSubjectiveJudgment, details: null },
    { name: 'Deterministic Outcome', value: checks.deterministicOutcome, details: null },
    { name: 'No Tool Coercion', value: checks.noCoercion, details: null },
    { name: 'Multiple Approaches', value: checks.multipleValidApproaches, details: null },
    { name: 'Clear Without Hints', value: checks.clearWithoutToolHint, details: null },
  ]

  for (const check of checkResults) {
    const icon = typeof check.value === 'boolean' ? (check.value ? '✓' : '✗') : '•'
    const valueStr = typeof check.value === 'boolean' ? (check.value ? 'Pass' : 'Fail') : check.value
    lines.push(`${icon} **${check.name}**: ${valueStr}`)
    if (check.details) {
      lines.push(`  ${check.details}`)
    }
  }

  // Survey results
  if (checks.surveyResults) {
    lines.push('')
    lines.push('## Survey Results')
    lines.push('')
    lines.push(`- **Respondents**: ${checks.surveyResults.respondents}`)
    lines.push(`- **Would Actually Do**: ${(checks.surveyResults.wouldActuallyDo * 100).toFixed(0)}%`)
    lines.push(`- **Average Frequency**: ${checks.surveyResults.averageFrequency}`)
    lines.push(`- **Realism Score**: ${checks.surveyResults.realismScore.toFixed(1)}/5`)
    if (checks.surveyResults.comments.length > 0) {
      lines.push('- **Comments**:')
      checks.surveyResults.comments.forEach((comment) => {
        lines.push(`  - "${comment}"`)
      })
    }
  }

  // Recommendations
  lines.push('')
  lines.push('## Recommendations')
  lines.push('')
  result.recommendations.forEach((rec, i) => {
    lines.push(`${i + 1}. ${rec}`)
  })

  // Failure reasons
  if (result.failureReasons && result.failureReasons.length > 0) {
    lines.push('')
    lines.push('## Failure Reasons')
    lines.push('')
    result.failureReasons.forEach((reason) => {
      lines.push(`- ${reason}`)
    })
  }

  return lines.join('\n')
}

// ============================================================================
// Developer Survey Framework
// ============================================================================

/**
 * Create a survey template for a task
 *
 * Generates markdown survey that can be sent to developers
 * to collect realism and frequency data.
 *
 * @param task - Task to create survey for
 * @returns Markdown survey template
 */
export function createSurveyTemplate(task: SearchTask): string {
  const lines: string[] = []

  lines.push('# Developer Task Survey')
  lines.push('')
  lines.push(`**Task ID**: ${task.id}`)
  lines.push(`**Task Name**: ${task.name}`)
  lines.push('')

  lines.push('## Task Description')
  lines.push('')
  lines.push(task.description)
  if (task.context) {
    lines.push('')
    lines.push('**Context**: ' + task.context)
  }
  lines.push('')

  lines.push('## Survey Questions')
  lines.push('')

  lines.push('### 1. Would you actually do this task in your daily work?')
  lines.push('- [ ] Yes')
  lines.push('- [ ] No')
  lines.push('')

  lines.push('### 2. How often would you do this task?')
  lines.push('- [ ] Daily (multiple times per day)')
  lines.push('- [ ] Weekly (once or twice per week)')
  lines.push('- [ ] Monthly (once or twice per month)')
  lines.push('- [ ] Rarely (few times per year)')
  lines.push('- [ ] Never')
  lines.push('')

  lines.push('### 3. How realistic is this task? (1-5)')
  lines.push('- [ ] 1 - Very unrealistic, would never do this')
  lines.push('- [ ] 2 - Somewhat unrealistic')
  lines.push('- [ ] 3 - Neutral, could happen')
  lines.push('- [ ] 4 - Realistic, happens sometimes')
  lines.push('- [ ] 5 - Very realistic, happens regularly')
  lines.push('')

  lines.push('### 4. Would this task help you in your work?')
  lines.push('- [ ] Yes')
  lines.push('- [ ] No')
  lines.push('')

  lines.push('### 5. Comments (optional)')
  lines.push('Please share any thoughts on this task:')
  lines.push('')
  lines.push('```')
  lines.push('')
  lines.push('```')
  lines.push('')

  lines.push('## About You')
  lines.push('')
  lines.push('**Role**: _________________ (e.g., Senior Engineer, Tech Lead)')
  lines.push('')
  lines.push('**Years of Experience**: _______')
  lines.push('')
  lines.push('**Codebase Size**:')
  lines.push('- [ ] Small (<10k lines)')
  lines.push('- [ ] Medium (10k-100k lines)')
  lines.push('- [ ] Large (100k-1M lines)')
  lines.push('- [ ] Very Large (>1M lines)')

  return lines.join('\n')
}

/**
 * Aggregate multiple survey responses
 *
 * Calculates statistics from developer surveys:
 * - Percentage who would actually do the task
 * - Average frequency
 * - Average realism score
 * - Collected comments
 *
 * @param surveys - Array of survey responses
 * @returns Aggregated survey results
 */
export function aggregateSurveyResults(surveys: DeveloperSurvey[]): SurveyResults {
  if (surveys.length === 0) {
    return {
      respondents: 0,
      wouldActuallyDo: 0,
      averageFrequency: 'never',
      realismScore: 0,
      comments: [],
    }
  }

  // Calculate would actually do percentage
  const actuallyDoCount = surveys.filter((s) => s.questions.wouldActuallyDo).length
  const wouldActuallyDo = actuallyDoCount / surveys.length

  // Calculate average frequency
  const frequencyMap: Record<string, number> = {
    daily: 4,
    weekly: 3,
    monthly: 2,
    rarely: 1,
    never: 0,
  }

  const avgFreqValue = surveys.reduce((sum, s) => sum + frequencyMap[s.questions.howOften], 0) / surveys.length

  let averageFrequency: string
  if (avgFreqValue >= 3.5) averageFrequency = 'daily'
  else if (avgFreqValue >= 2.5) averageFrequency = 'weekly'
  else if (avgFreqValue >= 1.5) averageFrequency = 'monthly'
  else if (avgFreqValue >= 0.5) averageFrequency = 'rarely'
  else averageFrequency = 'never'

  // Calculate average realism score
  const realismScore = surveys.reduce((sum, s) => sum + s.questions.isRealistic, 0) / surveys.length

  // Collect comments
  const comments = surveys.map((s) => s.questions.comments).filter((c): c is string => !!c)

  return {
    respondents: surveys.length,
    wouldActuallyDo,
    averageFrequency,
    realismScore,
    comments,
  }
}

/**
 * Analyze survey feedback and generate insights
 *
 * Extracts key insights from survey results:
 * - High/low realism indicators
 * - Frequency patterns
 * - Common themes in comments
 *
 * @param results - Aggregated survey results
 * @returns Array of insight strings
 */
export function analyzeSurveyFeedback(results: SurveyResults): string[] {
  const insights: string[] = []

  if (results.respondents === 0) {
    return ['No survey data available']
  }

  // Realism insights
  if (results.realismScore >= 4) {
    insights.push(
      `High realism score (${results.realismScore.toFixed(1)}/5) - developers strongly agree this is a realistic task`,
    )
  } else if (results.realismScore >= 3) {
    insights.push(
      `Moderate realism score (${results.realismScore.toFixed(1)}/5) - developers see this as somewhat realistic`,
    )
  } else {
    insights.push(
      `Low realism score (${results.realismScore.toFixed(1)}/5) - developers question the realism of this task`,
    )
  }

  // Would actually do
  if (results.wouldActuallyDo >= 0.7) {
    insights.push(
      `${(results.wouldActuallyDo * 100).toFixed(0)}% would actually do this task - strong ecological validity`,
    )
  } else if (results.wouldActuallyDo >= 0.4) {
    insights.push(
      `${(results.wouldActuallyDo * 100).toFixed(0)}% would actually do this task - moderate ecological validity`,
    )
  } else {
    insights.push(
      `Only ${(results.wouldActuallyDo * 100).toFixed(0)}% would actually do this task - weak ecological validity`,
    )
  }

  // Frequency insights
  insights.push(`Average frequency: ${results.averageFrequency}`)

  // Comments
  if (results.comments.length > 0) {
    insights.push(`Collected ${results.comments.length} comments from developers`)
  }

  return insights
}
