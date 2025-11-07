/**
 * Example usage of ecological validation
 *
 * Demonstrates how to validate tasks for ecological validity
 * and generate developer surveys.
 */

import {
  validateEcologicalValidity,
  classifyTaskFrequency,
  formatEcologicalReport,
  createSurveyTemplate,
  aggregateSurveyResults,
  analyzeSurveyFeedback,
  type SearchTask,
  type DeveloperSurvey,
} from './index.js'

// ============================================================================
// Example 1: High Realism Task
// ============================================================================

const highRealismTask: SearchTask = {
  id: 'AUTH-001',
  name: 'Find authentication flow for rate limiting',
  description:
    'Locate all authentication endpoints in the API to add rate limiting protection against brute force attacks',
  category: 'relationship-discovery',
  difficulty: 'medium',
  context: 'Security review identified lack of rate limiting on auth endpoints',

  searchTarget: {
    type: 'pattern',
    pattern: /authenticate|login|signup/,
  },

  followUpTask: {
    type: 'code_change',
    prompt: 'Add rate limiting middleware to authentication routes',
    validator: {
      type: 'code_change',
      fileChanged: 'src/routes/auth.ts',
      containsPattern: /rateLimit/,
    },
  },

  successValidator: () => ({
    searchQuality: 1,
    taskCompletion: 1,
    efficiency: 1,
    total: 1,
    details: 'Success',
  }),

  // Ecological metadata
  basedOnRealScenario: true,
  scenarioType: 'code-review',
  scenarioReference: 'Security audit finding #42',
  frequency: 'weekly',
  frequencyJustification: 'Common during security reviews and new feature development',
} as SearchTask & Record<string, unknown>

console.log('Example 1: High Realism Task')
console.log('='.repeat(80))

const result1 = validateEcologicalValidity(highRealismTask)
console.log(`Status: ${result1.passed ? 'PASSED' : 'FAILED'}`)
console.log(`Score: ${(result1.score * 100).toFixed(1)}%`)
console.log(`Frequency: ${result1.checks.frequency}`)
console.log()

const report1 = formatEcologicalReport(result1)
console.log(report1)
console.log()

// ============================================================================
// Example 2: Low Realism Task (with issues)
// ============================================================================

const lowRealismTask: SearchTask = {
  id: 'SYNTH-001',
  name: 'Find good implementations',
  description: 'Use semantic search to find thorough and quality code examples',
  category: 'implicit-knowledge',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    pattern: /example/,
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Write a comprehensive explanation of the best patterns',
    validator: {
      type: 'explanation',
    },
  },

  successValidator: () => ({
    searchQuality: 1,
    taskCompletion: 1,
    efficiency: 1,
    total: 1,
    details: 'Success',
  }),

  // Missing ecological metadata
} as SearchTask & Record<string, unknown>

console.log('Example 2: Low Realism Task')
console.log('='.repeat(80))

const result2 = validateEcologicalValidity(lowRealismTask)
console.log(`Status: ${result2.passed ? 'PASSED' : 'FAILED'}`)
console.log(`Score: ${(result2.score * 100).toFixed(1)}%`)
console.log('Issues:')
result2.failureReasons?.forEach((reason) => {
  console.log(`  - ${reason}`)
})
console.log()
console.log('Recommendations:')
result2.recommendations.slice(0, 3).forEach((rec, i) => {
  console.log(`  ${i + 1}. ${rec}`)
})
console.log()

// ============================================================================
// Example 3: Frequency Classification
// ============================================================================

console.log('Example 3: Frequency Classification')
console.log('='.repeat(80))

const debugTask: SearchTask = {
  ...highRealismTask,
  id: 'DEBUG-001',
  scenarioType: 'debugging',
}

const freq = classifyTaskFrequency(debugTask as SearchTask & Record<string, unknown>)
console.log(`Task: ${debugTask.id}`)
console.log(`Frequency: ${freq.frequency}`)
console.log(`Priority: ${freq.priority}`)
console.log(`Justification: ${freq.justification}`)
console.log()

// ============================================================================
// Example 4: Survey Framework
// ============================================================================

console.log('Example 4: Developer Survey Framework')
console.log('='.repeat(80))

// Create survey template
const surveyTemplate = createSurveyTemplate(highRealismTask)
console.log('Survey template created (truncated):')
console.log(surveyTemplate.split('\n').slice(0, 10).join('\n'))
console.log('...')
console.log()

// Mock survey responses
const mockSurveys: DeveloperSurvey[] = [
  {
    taskId: 'AUTH-001',
    taskDescription: highRealismTask.description,
    questions: {
      wouldActuallyDo: true,
      howOften: 'weekly',
      isRealistic: 5,
      wouldHelpMe: true,
      comments: 'Very realistic security task',
    },
    respondent: {
      role: 'Senior Engineer',
      experience: 8,
      codebaseSize: 'large',
    },
  },
  {
    taskId: 'AUTH-001',
    taskDescription: highRealismTask.description,
    questions: {
      wouldActuallyDo: true,
      howOften: 'weekly',
      isRealistic: 4,
      wouldHelpMe: true,
      comments: 'Do this during every security review',
    },
    respondent: {
      role: 'Tech Lead',
      experience: 12,
      codebaseSize: 'very-large',
    },
  },
  {
    taskId: 'AUTH-001',
    taskDescription: highRealismTask.description,
    questions: {
      wouldActuallyDo: true,
      howOften: 'monthly',
      isRealistic: 4,
      wouldHelpMe: true,
      comments: 'Important but not daily',
    },
    respondent: {
      role: 'Engineer',
      experience: 4,
      codebaseSize: 'medium',
    },
  },
]

// Aggregate results
const surveyResults = aggregateSurveyResults(mockSurveys)
console.log('Survey Results:')
console.log(`  Respondents: ${surveyResults.respondents}`)
console.log(`  Would actually do: ${(surveyResults.wouldActuallyDo * 100).toFixed(0)}%`)
console.log(`  Average frequency: ${surveyResults.averageFrequency}`)
console.log(`  Realism score: ${surveyResults.realismScore.toFixed(1)}/5`)
console.log()

// Analyze feedback
const insights = analyzeSurveyFeedback(surveyResults)
console.log('Survey Insights:')
insights.forEach((insight) => {
  console.log(`  - ${insight}`)
})
console.log()

// ============================================================================
// Example 5: Task with Survey Data
// ============================================================================

console.log('Example 5: Task with Survey Data')
console.log('='.repeat(80))

const taskWithSurvey: SearchTask = {
  ...highRealismTask,
  surveyResults,
} as SearchTask & Record<string, unknown>

const result5 = validateEcologicalValidity(taskWithSurvey)
console.log(`Status: ${result5.passed ? 'PASSED' : 'FAILED'}`)
console.log(`Score: ${(result5.score * 100).toFixed(1)}%`)
console.log(`Survey contributed: Yes (${result5.checks.surveyResults?.respondents} respondents)`)
console.log()

// ============================================================================
// Summary
// ============================================================================

console.log('Summary')
console.log('='.repeat(80))
console.log('Ecological validation ensures tasks reflect real-world developer activities.')
console.log('Key checks:')
console.log('  1. Based on real scenario')
console.log('  2. Appropriate frequency classification')
console.log('  3. Objective success criteria')
console.log('  4. No tool coercion')
console.log('  5. Survey validation (optional)')
console.log()
console.log('Pass threshold: 60% composite score')
console.log()
console.log('For more information:')
console.log('  - Implementation: packages/cli/src/search-optimization/validation/ecological.ts')
console.log('  - Documentation: docs/research/task-realism-survey.md')
console.log('  - Tests: packages/cli/src/search-optimization/validation/__tests__/ecological.test.ts')
