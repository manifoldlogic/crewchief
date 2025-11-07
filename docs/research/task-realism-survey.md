# Task Realism Survey Framework

## Overview

The Task Realism Survey Framework validates that search optimization tasks reflect real-world developer activities through structured feedback from practicing developers.

## Purpose

Ecological validity ensures benchmark tasks represent genuine developer needs rather than synthetic academic exercises. Developer surveys provide empirical evidence that:

1. **Tasks are realistic** - Developers would actually perform these tasks
2. **Frequency is accurate** - Task classification matches real-world patterns
3. **Value is clear** - Tasks help developers accomplish meaningful work
4. **Tools are appropriate** - Tasks leverage the right capabilities

## Survey Structure

### Core Questions

1. **Would Actually Do** (Boolean)
   - "Would you actually do this task in your daily work?"
   - Validates basic ecological validity

2. **Frequency** (Categorical)
   - "How often would you do this task?"
   - Options: Daily, Weekly, Monthly, Rarely, Never
   - Validates frequency classification

3. **Realism Rating** (1-5 Scale)
   - "How realistic is this task?"
   - 1 = Very unrealistic, 5 = Very realistic
   - Provides granular validity assessment

4. **Helpfulness** (Boolean)
   - "Would this task help you in your work?"
   - Validates practical value

5. **Comments** (Free Text)
   - "Please share any thoughts on this task"
   - Captures qualitative insights

### Respondent Information

- **Role** - Job title (e.g., "Senior Engineer", "Tech Lead")
- **Experience** - Years of professional development
- **Codebase Size** - Small (<10k), Medium (10k-100k), Large (100k-1M), Very Large (>1M lines)

## Scoring Methodology

### Survey Score Calculation

```typescript
surveyScore = (0.4 * wouldActuallyDo) + (0.6 * normalizedRealismScore)

where:
  wouldActuallyDo = percentage responding "yes" (0-1)
  normalizedRealismScore = (averageRealism - 1) / 4
```

### Integration with Ecological Score

Survey results contribute 10% to the composite ecological score:

```
ecologicalScore =
  0.30 * basedOnRealScenario +
  0.25 * frequencyPriority +
  0.20 * objectiveSuccessCriteria +
  0.15 * noCoercion +
  0.10 * surveyScore
```

## Usage

### Creating a Survey

```typescript
import { createSurveyTemplate } from './validation/ecological.js'

const task = { /* your task definition */ }
const surveyMarkdown = createSurveyTemplate(task)

// Distribute survey to developers
console.log(surveyMarkdown)
```

### Collecting Responses

```typescript
import type { DeveloperSurvey } from './validation/ecological.js'

const response: DeveloperSurvey = {
  taskId: 'TASK-001',
  taskDescription: 'Find authentication flow',
  questions: {
    wouldActuallyDo: true,
    howOften: 'weekly',
    isRealistic: 4,
    wouldHelpMe: true,
    comments: 'This is a common security review task'
  },
  respondent: {
    role: 'Senior Engineer',
    experience: 8,
    codebaseSize: 'large'
  }
}
```

### Aggregating Results

```typescript
import { aggregateSurveyResults, analyzeSurveyFeedback } from './validation/ecological.js'

const surveys: DeveloperSurvey[] = [ /* multiple responses */ ]

const results = aggregateSurveyResults(surveys)
console.log(`Respondents: ${results.respondents}`)
console.log(`Would actually do: ${results.wouldActuallyDo * 100}%`)
console.log(`Average frequency: ${results.averageFrequency}`)
console.log(`Realism score: ${results.realismScore}/5`)

const insights = analyzeSurveyFeedback(results)
insights.forEach(insight => console.log(`- ${insight}`))
```

### Attaching Results to Tasks

```typescript
const task = {
  id: 'TASK-001',
  name: 'Find authentication flow',
  // ... other task fields ...
  surveyResults: {
    respondents: 10,
    wouldActuallyDo: 0.8,
    averageFrequency: 'weekly',
    realismScore: 4.2,
    comments: [
      'Very realistic security review task',
      'Do this during every feature that touches auth',
      'Critical for preventing vulnerabilities'
    ]
  }
}
```

## Best Practices

### Sample Size

- **Minimum**: 5 respondents for basic validity
- **Target**: 10-15 respondents for statistical confidence
- **Ideal**: 20+ respondents for robust validation

### Respondent Diversity

Target developers with:
- **Various experience levels** - Junior (0-2yr), Mid (3-7yr), Senior (8-15yr), Staff+ (15+yr)
- **Different roles** - IC engineers, tech leads, architects, managers
- **Diverse codebases** - Small startups to large enterprises
- **Multiple domains** - Web, mobile, systems, data, etc.

### Survey Distribution

1. **Internal teams** - Start with developers on your team
2. **Open source communities** - Post to relevant forums/Discord/Slack
3. **Developer surveys** - Include in regular developer experience surveys
4. **Conference attendees** - Distribute at workshops/talks
5. **Academic partners** - Collaborate with HCI/SE researchers

### Interpreting Results

**Strong Ecological Validity** (Pass)
- Would actually do: ≥70%
- Realism score: ≥4.0
- Frequency matches classification
- Positive comments about realism

**Moderate Ecological Validity** (Review)
- Would actually do: 40-69%
- Realism score: 3.0-3.9
- Frequency close to classification
- Mixed comments

**Weak Ecological Validity** (Fail)
- Would actually do: <40%
- Realism score: <3.0
- Frequency doesn't match
- Negative comments about realism

## Example Survey Data

### High Realism Task

```typescript
{
  taskId: 'ARCH-042',
  taskDescription: 'Find all places where user authentication is checked',
  surveyResults: {
    respondents: 12,
    wouldActuallyDo: 0.92,  // 92% would do this
    averageFrequency: 'weekly',
    realismScore: 4.7,
    comments: [
      'Do this every security review',
      'Critical for understanding auth boundaries',
      'Needed when adding new protected routes',
      'Common task when onboarding to a new codebase'
    ]
  }
}
```

### Low Realism Task

```typescript
{
  taskId: 'SYNTH-001',
  taskDescription: 'Find all functions with exactly 42 parameters',
  surveyResults: {
    respondents: 8,
    wouldActuallyDo: 0.12,  // Only 12% would do this
    averageFrequency: 'never',
    realismScore: 1.4,
    comments: [
      'Never needed to do this',
      'Seems like a synthetic edge case',
      'Would just run a linter',
      'Not a real-world scenario'
    ]
  }
}
```

## Integration with Validation

Ecological validation automatically uses survey results if present:

```typescript
import { validateEcologicalValidity } from './validation/ecological.js'

const task = {
  id: 'TASK-001',
  name: 'Find authentication flow',
  // ... task definition ...
  surveyResults: { /* aggregated survey data */ }
}

const result = validateEcologicalValidity(task)

console.log(`Ecological score: ${result.score}`)
console.log(`Passed: ${result.passed}`)
console.log(`Survey contributed: ${result.checks.surveyResults ? 'Yes' : 'No'}`)
```

## Research Applications

### Academic Studies

Survey data supports research on:
- **Developer tool usage** - How developers actually use code search
- **Task frequency patterns** - What developers do daily vs. rarely
- **Tool effectiveness** - Which capabilities provide most value
- **Benchmark validity** - Are benchmarks measuring real capabilities?

### Industry Applications

Organizations can use surveys to:
- **Validate internal benchmarks** - Ensure evals match team needs
- **Prioritize features** - Build tools for common tasks first
- **Training effectiveness** - Verify training scenarios are realistic
- **Tool adoption** - Understand why developers do/don't use tools

## Limitations

Survey data is **supplementary, not required**:

1. **Automated checks are primary** - Scenario markers, objectivity, frequency classification
2. **Surveys are expensive** - Require developer time and coordination
3. **Surveys can be biased** - Selection bias, social desirability, response rate issues
4. **Surveys are lagging indicators** - Can't predict future developer needs

Use surveys to **validate** automated ecological checks, not replace them.

## Future Enhancements

Potential improvements to the survey framework:

1. **Task variants** - Survey different approaches to the same goal
2. **Tool comparisons** - Rate effectiveness of different search strategies
3. **Video demonstrations** - Show tasks being performed, gather feedback
4. **Longitudinal studies** - Track how task realism changes over time
5. **Context-specific surveys** - Different questions for different roles/domains

## References

- **Ecological Validity in HCI**: [ISO 9241-11:2018](https://www.iso.org/standard/63500.html)
- **Developer Experience Research**: [DX DevEx Framework](https://queue.acm.org/detail.cfm?id=3595878)
- **Survey Design**: [Dillman's Tailored Design Method](https://www.wiley.com/en-us/Internet%2C+Phone%2C+Mail%2C+and+Mixed+Mode+Surveys%3A+The+Tailored+Design+Method%2C+4th+Edition-p-9781118456149)
- **Benchmark Validation**: [SWE-bench: Can LLMs Resolve GitHub Issues?](https://www.swebench.com/)

## Contact

For questions about the survey framework:
- **Implementation**: See `packages/cli/src/search-optimization/validation/ecological.ts`
- **Documentation**: This file
- **Issues**: File GitHub issues with label `ecological-validation`
