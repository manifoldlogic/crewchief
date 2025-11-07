/**
 * Tests for ecological validation
 *
 * Comprehensive test coverage for:
 * - Realism validation with various task metadata
 * - Frequency classification
 * - Ecological score calculation
 * - Survey aggregation
 * - Report formatting
 * - Edge cases (missing metadata, invalid values)
 */

import { describe, it, expect } from 'vitest'
import {
  validateEcologicalValidity,
  classifyTaskFrequency,
  validateRealism,
  calculateEcologicalScore,
  formatEcologicalReport,
  createSurveyTemplate,
  aggregateSurveyResults,
  analyzeSurveyFeedback,
  type SearchTask,
  type DeveloperSurvey,
  type EcologicalChecks,
} from '../index.js'

// ============================================================================
// Test Fixtures
// ============================================================================

/**
 * Create a mock task with configurable properties
 */
function createMockTask(overrides?: Partial<SearchTask & Record<string, unknown>>): SearchTask {
  return {
    id: 'eco-test-001',
    name: 'Find authentication flow',
    description:
      'Locate the main authentication flow in the codebase to understand how users are validated and tokens are issued',
    category: 'relationship-discovery',
    difficulty: 'hard',
    searchTarget: {
      type: 'pattern',
      pattern: /authenticate/,
    },
    followUpTask: {
      type: 'code_change',
      prompt: 'Add rate limiting to the authentication endpoint',
      validator: {
        type: 'code_change',
        fileChanged: 'auth.ts',
      },
    },
    successValidator: () => ({
      searchQuality: 1,
      taskCompletion: 1,
      efficiency: 1,
      total: 1,
      details: 'Success',
    }),
    context: 'Security review identified need for rate limiting on auth endpoints',
    ...overrides,
  } as SearchTask
}

/**
 * Create a mock survey response
 */
function createMockSurvey(overrides?: Partial<DeveloperSurvey>): DeveloperSurvey {
  return {
    taskId: 'eco-test-001',
    taskDescription: 'Test task',
    questions: {
      wouldActuallyDo: true,
      howOften: 'weekly',
      isRealistic: 4,
      wouldHelpMe: true,
      comments: 'Looks realistic',
    },
    respondent: {
      role: 'Senior Engineer',
      experience: 8,
      codebaseSize: 'large',
    },
    ...overrides,
  }
}

// ============================================================================
// Ecological Validation Tests
// ============================================================================

describe('validateEcologicalValidity', () => {
  it('should pass for high-quality realistic task', () => {
    const task = createMockTask({
      basedOnRealScenario: true,
      scenarioType: 'code-review',
      frequency: 'weekly',
      frequencyJustification: 'Common security review task',
    })

    const result = validateEcologicalValidity(task)

    expect(result.passed).toBe(true)
    expect(result.score).toBeGreaterThanOrEqual(0.6)
    expect(result.checks.basedOnRealScenario).toBe(true)
    expect(result.checks.objectiveSuccessCriteria).toBe(true)
    expect(result.checks.noCoercion).toBe(true)
    expect(result.failureReasons).toBeUndefined()
  })

  it('should fail for synthetic task without real scenario marker', () => {
    const task = createMockTask({
      basedOnRealScenario: false,
      description: 'A synthetic test task',
      context: undefined,
    })

    const result = validateEcologicalValidity(task)

    expect(result.passed).toBe(false)
    expect(result.score).toBeLessThan(0.6)
    expect(result.checks.basedOnRealScenario).toBe(false)
    expect(result.failureReasons).toContain('Not explicitly marked as based on real scenario')
  })

  it('should warn about rare frequency tasks', () => {
    const task = createMockTask({
      basedOnRealScenario: true,
      frequency: 'rare',
    })

    const result = validateEcologicalValidity(task)

    // Rare frequency alone doesn't fail if other factors are strong
    expect(result.checks.frequency).toBe('rare')
    expect(result.score).toBeLessThan(0.75) // Lower score but may still pass

    // Should have recommendation about frequency
    expect(result.recommendations.some((r) => r.includes('rare'))).toBe(true)
  })

  it('should fail for subjective success criteria', () => {
    const task = createMockTask({
      description: 'Find a good implementation of the authentication flow',
      followUpTask: {
        type: 'explanation',
        prompt: 'Write a thorough explanation of the best approach',
        validator: {
          type: 'explanation',
        },
      },
    })

    const result = validateEcologicalValidity(task)

    expect(result.passed).toBe(false)
    expect(result.checks.objectiveSuccessCriteria).toBe(false)
    expect(result.failureReasons).toContain('Success criteria are not objective')
  })

  it('should detect and warn about tool coercion', () => {
    const task = createMockTask({
      description: 'Use semantic search to find the authentication flow',
      basedOnRealScenario: true,
    })

    const result = validateEcologicalValidity(task)

    expect(result.checks.noCoercion).toBe(false)

    // May still pass overall if other factors are strong, but should have recommendation
    expect(result.recommendations.some((r) => r.includes('semantic search'))).toBe(true)
  })

  it('should include survey results in validation', () => {
    const task = createMockTask({
      basedOnRealScenario: true,
      surveyResults: {
        respondents: 10,
        wouldActuallyDo: 0.9,
        averageFrequency: 'weekly',
        realismScore: 4.5,
        comments: ['Very realistic', 'Do this all the time'],
      },
    })

    const result = validateEcologicalValidity(task)

    expect(result.passed).toBe(true)
    expect(result.checks.surveyResults).toBeDefined()
    expect(result.checks.surveyResults?.realismScore).toBe(4.5)
  })
})

// ============================================================================
// Frequency Classification Tests
// ============================================================================

describe('classifyTaskFrequency', () => {
  it('should use explicit frequency metadata', () => {
    const task = createMockTask({
      frequency: 'daily',
      frequencyJustification: 'Done multiple times per day during code reviews',
    })

    const result = classifyTaskFrequency(task)

    expect(result.frequency).toBe('daily')
    expect(result.priority).toBe(1.0)
    expect(result.justification).toContain('Done multiple times per day')
  })

  it('should infer frequency from scenario type', () => {
    const task = createMockTask({
      scenarioType: 'debugging',
    })

    const result = classifyTaskFrequency(task)

    expect(result.frequency).toBe('weekly')
    expect(result.priority).toBe(0.7)
    expect(result.justification).toContain('debugging')
  })

  it('should infer frequency from category', () => {
    const task = createMockTask({
      category: 'relationship-discovery',
    })

    const result = classifyTaskFrequency(task)

    expect(result.frequency).toBe('weekly')
    expect(result.priority).toBe(0.7)
  })

  it('should default to rare for unknown categories', () => {
    const task = createMockTask({
      category: 'unknown-category',
    })

    const result = classifyTaskFrequency(task)

    expect(result.frequency).toBe('rare')
    expect(result.priority).toBe(0.1)
  })

  it('should classify code-review scenario as daily', () => {
    const task = createMockTask({
      scenarioType: 'code-review',
    })

    const result = classifyTaskFrequency(task)

    expect(result.frequency).toBe('daily')
    expect(result.priority).toBe(1.0)
  })

  it('should classify onboarding scenario as rare', () => {
    const task = createMockTask({
      scenarioType: 'onboarding',
    })

    const result = classifyTaskFrequency(task)

    expect(result.frequency).toBe('rare')
    expect(result.priority).toBe(0.1)
  })
})

// ============================================================================
// Realism Validation Tests
// ============================================================================

describe('validateRealism', () => {
  it('should detect real scenario marker', () => {
    const task = createMockTask({
      basedOnRealScenario: true,
      scenarioType: 'debugging',
      scenarioReference: 'Issue #123',
    })

    const checks = validateRealism(task)

    expect(checks.basedOnRealScenario).toBe(true)
    expect(checks.scenarioType).toBe('debugging')
    expect(checks.scenarioReference).toBe('Issue #123')
  })

  it('should detect subjective words in description', () => {
    const task = createMockTask({
      description: 'Find a good implementation with quality code',
    })

    const checks = validateRealism(task)

    expect(checks.objectiveSuccessCriteria).toBe(false)
    expect(checks.noSubjectiveJudgment).toBe(false)
  })

  it('should detect subjective words in follow-up prompt', () => {
    const task = createMockTask({
      followUpTask: {
        type: 'explanation',
        prompt: 'Write a thorough and comprehensive explanation',
        validator: {
          type: 'explanation',
        },
      },
    })

    const checks = validateRealism(task)

    expect(checks.objectiveSuccessCriteria).toBe(false)
  })

  it('should detect tool hints in description', () => {
    const task = createMockTask({
      description: 'Use semantic search to find the authentication logic',
    })

    const checks = validateRealism(task)

    expect(checks.noCoercion).toBe(false)
  })

  it('should detect tool hints in follow-up prompt', () => {
    const task = createMockTask({
      followUpTask: {
        type: 'code_change',
        prompt: 'Use grep to search for the pattern',
        validator: {
          type: 'code_change',
        },
      },
    })

    const checks = validateRealism(task)

    expect(checks.noCoercion).toBe(false)
  })

  it('should mark explanation validators as non-deterministic', () => {
    const task = createMockTask({
      followUpTask: {
        type: 'explanation',
        prompt: 'Explain the pattern',
        validator: {
          type: 'explanation',
        },
      },
    })

    const checks = validateRealism(task)

    expect(checks.deterministicOutcome).toBe(false)
  })

  it('should mark code_change validators as deterministic', () => {
    const task = createMockTask({
      followUpTask: {
        type: 'code_change',
        prompt: 'Add the pattern',
        validator: {
          type: 'code_change',
        },
      },
    })

    const checks = validateRealism(task)

    expect(checks.deterministicOutcome).toBe(true)
  })

  it('should detect multiple valid approaches for pattern targets', () => {
    const task = createMockTask({
      searchTarget: {
        type: 'pattern',
        pattern: /auth/,
      },
    })

    const checks = validateRealism(task)

    expect(checks.multipleValidApproaches).toBe(true)
  })

  it('should detect task clarity from description length and context', () => {
    const task = createMockTask({
      description: 'Find auth', // Too short
      context: undefined,
    })

    const checks = validateRealism(task)

    expect(checks.clearWithoutToolHint).toBe(false)
  })
})

// ============================================================================
// Ecological Score Tests
// ============================================================================

describe('calculateEcologicalScore', () => {
  it('should give maximum score for perfect task', () => {
    const checks: EcologicalChecks = {
      basedOnRealScenario: true,
      scenarioType: 'code-review',
      frequency: 'daily',
      frequencyJustification: 'Very common',
      objectiveSuccessCriteria: true,
      noSubjectiveJudgment: true,
      deterministicOutcome: true,
      noCoercion: true,
      multipleValidApproaches: true,
      clearWithoutToolHint: true,
    }

    const score = calculateEcologicalScore(checks)

    expect(score).toBeGreaterThanOrEqual(0.9)
    expect(score).toBeLessThanOrEqual(1.0)
  })

  it('should penalize rare frequency', () => {
    const checks: EcologicalChecks = {
      basedOnRealScenario: true,
      frequency: 'rare', // Low priority (0.1)
      frequencyJustification: 'Rarely done',
      objectiveSuccessCriteria: true,
      noSubjectiveJudgment: true,
      deterministicOutcome: true,
      noCoercion: true,
      multipleValidApproaches: true,
      clearWithoutToolHint: true,
    }

    const score = calculateEcologicalScore(checks)

    // Score calculation: 0.3 (real) + 0.025 (rare freq at 0.1 priority) + 0.2 (objective) + 0.15 (no coercion) + 0.05 (no survey) = 0.725
    // Still passes threshold (0.6) because other factors are strong
    expect(score).toBeGreaterThan(0.6) // Passes overall
    expect(score).toBeLessThan(0.8) // But notably lower than daily/weekly tasks
  })

  it('should penalize subjective criteria', () => {
    const checks: EcologicalChecks = {
      basedOnRealScenario: true,
      frequency: 'weekly',
      frequencyJustification: 'Weekly task',
      objectiveSuccessCriteria: false, // Subjective
      noSubjectiveJudgment: false,
      deterministicOutcome: false,
      noCoercion: true,
      multipleValidApproaches: true,
      clearWithoutToolHint: true,
    }

    const score = calculateEcologicalScore(checks)

    expect(score).toBeLessThan(0.7)
  })

  it('should penalize tool coercion', () => {
    const checks: EcologicalChecks = {
      basedOnRealScenario: true,
      frequency: 'weekly',
      frequencyJustification: 'Weekly task',
      objectiveSuccessCriteria: true,
      noSubjectiveJudgment: true,
      deterministicOutcome: true,
      noCoercion: false, // Coercion present
      multipleValidApproaches: true,
      clearWithoutToolHint: true,
    }

    const score = calculateEcologicalScore(checks)

    expect(score).toBeLessThan(0.85)
  })

  it('should incorporate survey results when available', () => {
    const checksWithSurvey: EcologicalChecks = {
      basedOnRealScenario: true,
      frequency: 'weekly',
      frequencyJustification: 'Weekly task',
      objectiveSuccessCriteria: true,
      noSubjectiveJudgment: true,
      deterministicOutcome: true,
      noCoercion: true,
      multipleValidApproaches: true,
      clearWithoutToolHint: true,
      surveyResults: {
        respondents: 10,
        wouldActuallyDo: 1.0,
        averageFrequency: 'weekly',
        realismScore: 5,
        comments: [],
      },
    }

    const checksWithoutSurvey: EcologicalChecks = {
      ...checksWithSurvey,
      surveyResults: undefined,
    }

    const scoreWithSurvey = calculateEcologicalScore(checksWithSurvey)
    const scoreWithoutSurvey = calculateEcologicalScore(checksWithoutSurvey)

    expect(scoreWithSurvey).toBeGreaterThan(scoreWithoutSurvey)
  })

  it('should give partial credit for clear description without real scenario', () => {
    const checks: EcologicalChecks = {
      basedOnRealScenario: false,
      frequency: 'weekly',
      frequencyJustification: 'Weekly task',
      objectiveSuccessCriteria: true,
      noSubjectiveJudgment: true,
      deterministicOutcome: true,
      noCoercion: true,
      multipleValidApproaches: true,
      clearWithoutToolHint: true, // Clear description
    }

    const score = calculateEcologicalScore(checks)

    expect(score).toBeGreaterThan(0.5)
    expect(score).toBeLessThan(0.8)
  })
})

// ============================================================================
// Recommendations Tests
// ============================================================================

describe('generateEcologicalRecommendations', () => {
  it('should recommend marking real scenarios', () => {
    const result = validateEcologicalValidity(
      createMockTask({
        basedOnRealScenario: false,
      }),
    )

    expect(result.recommendations.some((r) => r.includes('basedOnRealScenario'))).toBe(true)
  })

  it('should recommend frequency reclassification for rare tasks', () => {
    const result = validateEcologicalValidity(
      createMockTask({
        frequency: 'rare',
        basedOnRealScenario: true,
      }),
    )

    expect(result.recommendations.some((r) => r.includes('rare'))).toBe(true)
  })

  it('should identify subjective words to remove', () => {
    const result = validateEcologicalValidity(
      createMockTask({
        description: 'Find a good and thorough implementation',
      }),
    )

    expect(result.recommendations.some((r) => r.includes('good'))).toBe(true)
  })

  it('should identify tool hints to remove', () => {
    const result = validateEcologicalValidity(
      createMockTask({
        description: 'Use semantic search to find the code',
      }),
    )

    expect(result.recommendations.some((r) => r.includes('semantic search'))).toBe(true)
  })

  it('should recommend more objective validators', () => {
    const result = validateEcologicalValidity(
      createMockTask({
        followUpTask: {
          type: 'explanation',
          prompt: 'Explain',
          validator: {
            type: 'explanation',
          },
        },
      }),
    )

    expect(result.recommendations.some((r) => r.includes('code_change'))).toBe(true)
  })

  it('should recommend adding context', () => {
    const result = validateEcologicalValidity(
      createMockTask({
        description: 'Short',
        context: undefined,
      }),
    )

    expect(result.recommendations.some((r) => r.includes('context'))).toBe(true)
  })

  it('should recommend survey collection', () => {
    const result = validateEcologicalValidity(
      createMockTask({
        basedOnRealScenario: true,
      }),
    )

    expect(result.recommendations.some((r) => r.includes('survey'))).toBe(true)
  })

  it('should provide success message for passing tasks', () => {
    const result = validateEcologicalValidity(
      createMockTask({
        basedOnRealScenario: true,
        frequency: 'daily',
      }),
    )

    if (result.passed) {
      expect(result.recommendations.some((r) => r.includes('passes ecological validation'))).toBe(true)
    }
  })
})

// ============================================================================
// Report Formatting Tests
// ============================================================================

describe('formatEcologicalReport', () => {
  it('should format complete report with all sections', () => {
    const task = createMockTask({
      basedOnRealScenario: true,
      frequency: 'weekly',
    })

    const result = validateEcologicalValidity(task)
    const report = formatEcologicalReport(result)

    expect(report).toContain('Ecological Validation Report')
    expect(report).toContain(task.name)
    expect(report).toContain(task.id)
    expect(report).toContain('Ecological Checks')
    expect(report).toContain('Recommendations')
    expect(report).toContain('Real Scenario')
    expect(report).toContain('Frequency')
  })

  it('should show pass status', () => {
    const task = createMockTask({
      basedOnRealScenario: true,
      frequency: 'daily',
    })

    const result = validateEcologicalValidity(task)
    const report = formatEcologicalReport(result)

    if (result.passed) {
      expect(report).toContain('✓ PASSED')
    }
  })

  it('should show fail status and reasons', () => {
    const task = createMockTask({
      basedOnRealScenario: false,
      frequency: 'rare',
    })

    const result = validateEcologicalValidity(task)
    const report = formatEcologicalReport(result)

    expect(report).toContain('✗ FAILED')
    if (result.failureReasons) {
      expect(report).toContain('Failure Reasons')
    }
  })

  it('should include survey results when available', () => {
    const task = createMockTask({
      basedOnRealScenario: true,
      surveyResults: {
        respondents: 5,
        wouldActuallyDo: 0.8,
        averageFrequency: 'weekly',
        realismScore: 4.2,
        comments: ['Great task', 'Very realistic'],
      },
    })

    const result = validateEcologicalValidity(task)
    const report = formatEcologicalReport(result)

    expect(report).toContain('Survey Results')
    expect(report).toContain('5')
    expect(report).toContain('4.2')
    expect(report).toContain('Great task')
  })
})

// ============================================================================
// Survey Framework Tests
// ============================================================================

describe('createSurveyTemplate', () => {
  it('should create complete survey template', () => {
    const task = createMockTask()
    const template = createSurveyTemplate(task)

    expect(template).toContain('Developer Task Survey')
    expect(template).toContain(task.id)
    expect(template).toContain(task.name)
    expect(template).toContain(task.description)
    expect(template).toContain('Would you actually do this task')
    expect(template).toContain('How often')
    expect(template).toContain('How realistic')
    expect(template).toContain('About You')
  })

  it('should include context if available', () => {
    const task = createMockTask({
      context: 'This is important context',
    })

    const template = createSurveyTemplate(task)

    expect(template).toContain('Context')
    expect(template).toContain('This is important context')
  })
})

describe('aggregateSurveyResults', () => {
  it('should return empty results for no surveys', () => {
    const results = aggregateSurveyResults([])

    expect(results.respondents).toBe(0)
    expect(results.wouldActuallyDo).toBe(0)
    expect(results.realismScore).toBe(0)
  })

  it('should calculate would-actually-do percentage', () => {
    const surveys = [
      createMockSurvey({ questions: { ...createMockSurvey().questions, wouldActuallyDo: true } }),
      createMockSurvey({ questions: { ...createMockSurvey().questions, wouldActuallyDo: true } }),
      createMockSurvey({ questions: { ...createMockSurvey().questions, wouldActuallyDo: false } }),
    ]

    const results = aggregateSurveyResults(surveys)

    expect(results.respondents).toBe(3)
    expect(results.wouldActuallyDo).toBeCloseTo(2 / 3)
  })

  it('should calculate average frequency', () => {
    const surveys = [
      createMockSurvey({ questions: { ...createMockSurvey().questions, howOften: 'daily' } }),
      createMockSurvey({ questions: { ...createMockSurvey().questions, howOften: 'daily' } }),
      createMockSurvey({ questions: { ...createMockSurvey().questions, howOften: 'weekly' } }),
    ]

    const results = aggregateSurveyResults(surveys)

    expect(results.averageFrequency).toBe('daily')
  })

  it('should calculate average realism score', () => {
    const surveys = [
      createMockSurvey({ questions: { ...createMockSurvey().questions, isRealistic: 5 } }),
      createMockSurvey({ questions: { ...createMockSurvey().questions, isRealistic: 4 } }),
      createMockSurvey({ questions: { ...createMockSurvey().questions, isRealistic: 3 } }),
    ]

    const results = aggregateSurveyResults(surveys)

    expect(results.realismScore).toBeCloseTo(4)
  })

  it('should collect comments', () => {
    const surveys = [
      createMockSurvey({ questions: { ...createMockSurvey().questions, comments: 'Good task' } }),
      createMockSurvey({ questions: { ...createMockSurvey().questions, comments: 'Very realistic' } }),
      createMockSurvey({ questions: { ...createMockSurvey().questions, comments: undefined } }),
    ]

    const results = aggregateSurveyResults(surveys)

    expect(results.comments).toHaveLength(2)
    expect(results.comments).toContain('Good task')
    expect(results.comments).toContain('Very realistic')
  })
})

describe('analyzeSurveyFeedback', () => {
  it('should return no data message for empty results', () => {
    const results = aggregateSurveyResults([])
    const insights = analyzeSurveyFeedback(results)

    expect(insights).toContain('No survey data available')
  })

  it('should identify high realism score', () => {
    const results = {
      respondents: 10,
      wouldActuallyDo: 0.8,
      averageFrequency: 'weekly',
      realismScore: 4.5,
      comments: [],
    }

    const insights = analyzeSurveyFeedback(results)

    expect(insights.some((i) => i.includes('High realism'))).toBe(true)
  })

  it('should identify low realism score', () => {
    const results = {
      respondents: 10,
      wouldActuallyDo: 0.3,
      averageFrequency: 'rarely',
      realismScore: 2.1,
      comments: [],
    }

    const insights = analyzeSurveyFeedback(results)

    expect(insights.some((i) => i.includes('Low realism'))).toBe(true)
  })

  it('should report would-actually-do percentage', () => {
    const results = {
      respondents: 10,
      wouldActuallyDo: 0.75,
      averageFrequency: 'weekly',
      realismScore: 4,
      comments: [],
    }

    const insights = analyzeSurveyFeedback(results)

    expect(insights.some((i) => i.includes('75%'))).toBe(true)
  })

  it('should report average frequency', () => {
    const results = {
      respondents: 10,
      wouldActuallyDo: 0.8,
      averageFrequency: 'weekly',
      realismScore: 4,
      comments: [],
    }

    const insights = analyzeSurveyFeedback(results)

    expect(insights.some((i) => i.includes('weekly'))).toBe(true)
  })

  it('should report comment count', () => {
    const results = {
      respondents: 10,
      wouldActuallyDo: 0.8,
      averageFrequency: 'weekly',
      realismScore: 4,
      comments: ['Good', 'Great', 'Excellent'],
    }

    const insights = analyzeSurveyFeedback(results)

    expect(insights.some((i) => i.includes('3 comments'))).toBe(true)
  })
})
