# TESTDES-3002: Implement Ecological Validation

**Status**: 🔵 Not Started
**Priority**: High
**Complexity**: Medium (4-6 hours)
**Phase**: 3 - Validation Infrastructure
**Dependencies**: TESTDES-3001

## Summary

Implement ecological validation to ensure tasks reflect real-world developer activities. This includes automated realism checks, a developer survey framework for external validation, and task frequency classification. Ecological validity ensures our benchmarks measure practically useful capabilities, not artificial scenarios.

## Background

Ecological validity is a critical quality dimension from the quality strategy. A task can be perfectly designed (grep fails, search succeeds, statistically significant) but still be useless if developers never actually perform that work.

This ticket addresses the "would developers actually do this?" question through:
1. Automated realism checks (scenario-based validation)
2. External validation framework (developer surveys)
3. Frequency classification (daily/weekly/monthly/rare tasks)
4. Task origin tracking (based on real scenarios vs synthetic)

Research foundation: Ecological validity from psychology research, task analysis from HCI, frequency-based prioritization from user research.

From quality-strategy.md: "Developer survey: 70%+ say 'I would actually do this task'" and "Tasks based on real scenarios (not synthetic)".

## Acceptance Criteria

- [x] Realism validation checks implemented: scenario-based, frequency classification, task-origin tracking
- [x] Developer survey framework created with standardized questions and scoring
- [x] Task frequency classification system works: tags tasks as daily/weekly/monthly/rare
- [x] Integration with task validator (TESTDES-3001) for automated ecological checks
- [x] Survey template documented in `docs/research/task-realism-survey.md`
- [x] Ecological validation report generator creates markdown summaries

## Technical Requirements

**Architecture**:
- TypeScript implementation in `packages/cli/src/search-optimization/validation/`
- Integration point with task-validator from TESTDES-3001
- Survey framework documented for manual execution
- Frequency classification based on task metadata

**Interfaces**:
```typescript
interface EcologicalChecks {
  // Realism validation
  basedOnRealScenario: boolean
  scenarioType?: 'code-review' | 'debugging' | 'refactoring' | 'onboarding' | 'maintenance'
  scenarioReference?: string  // Link to PR, issue, actual work

  // Frequency assessment
  frequency: 'daily' | 'weekly' | 'monthly' | 'rare'
  frequencyJustification: string

  // Developer validation
  surveyResults?: {
    respondents: number
    wouldActuallyDo: number  // Count who said "yes"
    averageFrequency: string
    realismScore: number  // 1-5 scale
    comments: string[]
  }

  // Task quality
  objectiveSuccessCriteria: boolean
  noSubjectiveJudgment: boolean
  deterministicOutcome: boolean

  // Fairness
  noCoercion: boolean  // Task doesn't force tool choice
  multipleValidApproaches: boolean
  clearWithoutToolHint: boolean
}

interface EcologicalValidationResult {
  task: SearchTask
  checks: EcologicalChecks
  passed: boolean
  score: number  // 0-1 composite score
  recommendations: string[]
  failureReasons?: string[]
}

interface DeveloperSurvey {
  taskId: string
  taskDescription: string
  questions: {
    wouldActuallyDo: boolean  // "Would you do this in real work?"
    howOften: 'daily' | 'weekly' | 'monthly' | 'rarely' | 'never'
    isRealistic: 1 | 2 | 3 | 4 | 5  // "How realistic is this scenario?"
    wouldHelpMe: boolean  // "Would semantic search help here?"
    comments?: string
  }
  respondent: {
    role: string  // e.g., "Senior Engineer", "Junior Dev"
    experience: number  // years
    codebaseSize: 'small' | 'medium' | 'large' | 'very-large'
  }
}
```

**Automated Validation**:
- Check task has `basedOnRealScenario` metadata
- Verify success criteria are objective (no "good explanation" wording)
- Ensure task description doesn't hint at tools to use
- Validate frequency classification is documented

**Survey Framework**:
- Standardized questions for consistency across tasks
- Scoring rubric for quantitative analysis
- Template in `docs/research/task-realism-survey.md`
- Data collection format (JSON or CSV)

**Frequency Classification**:
```typescript
interface FrequencyClassification {
  daily: {
    description: 'Tasks developers do multiple times per day'
    examples: ['Find error handling', 'Trace function calls', 'Review code changes']
    priority: 'high'
  }
  weekly: {
    description: 'Tasks done once or twice per week'
    examples: ['Find all retry logic', 'Understand initialization flow']
    priority: 'medium'
  }
  monthly: {
    description: 'Occasional tasks during major work'
    examples: ['Architectural analysis', 'Security audit']
    priority: 'medium'
  }
  rare: {
    description: 'Infrequent specialized tasks'
    examples: ['Find all circular dependencies']
    priority: 'low'
  }
}
```

## Implementation Notes

### Automated Realism Checks
```typescript
function validateRealism(task: SearchTask): EcologicalChecks {
  const checks: Partial<EcologicalChecks> = {}

  // Scenario validation
  checks.basedOnRealScenario = !!task.metadata?.scenarioReference
  checks.scenarioType = task.metadata?.scenarioType

  // Frequency classification
  checks.frequency = task.metadata?.frequency || 'rare'

  // Success criteria objectivity
  checks.objectiveSuccessCriteria = validateObjectiveCriteria(task.successValidator)
  checks.deterministicOutcome = !hasSubjectiveWords(task.description)

  // Fairness checks
  checks.noCoercion = !hasToolHints(task.description)
  checks.multipleValidApproaches = true  // Default assumption

  return checks as EcologicalChecks
}
```

### Survey Template Structure
```markdown
# Task Realism Survey

**Task**: [Task description]

## Questions

1. **Would you actually do this task in your real development work?**
   - [ ] Yes, frequently
   - [ ] Yes, occasionally
   - [ ] Maybe in specific situations
   - [ ] Probably not
   - [ ] Definitely not

2. **How often do you perform similar tasks?**
   - [ ] Multiple times per day
   - [ ] Once or twice per week
   - [ ] Once or twice per month
   - [ ] Rarely (few times per year)
   - [ ] Never

3. **How realistic is this scenario? (1-5 scale)**
   - 1 = Completely artificial
   - 5 = Exactly what I do

4. **Would semantic code search help you with this task?**
   - [ ] Yes, significantly
   - [ ] Yes, somewhat
   - [ ] Not sure
   - [ ] Probably not
   - [ ] Definitely not

5. **Comments/suggestions**: [Free text]

## About You
- Role: [Senior Engineer, Junior Dev, etc.]
- Years of experience: [number]
- Typical codebase size: [Small (<10k LOC), Medium (10-100k), Large (100k-1M), Very Large (>1M)]
```

### Frequency Classification Logic
```typescript
function classifyFrequency(task: SearchTask): FrequencyClassification {
  // Priority scoring based on frequency
  const frequencyPriority = {
    daily: 1.0,    // Highest value
    weekly: 0.7,   // Medium-high value
    monthly: 0.4,  // Medium value
    rare: 0.1      // Low priority
  }

  return {
    frequency: task.metadata?.frequency || inferFrequency(task),
    priority: frequencyPriority[task.metadata?.frequency || 'rare'],
    justification: task.metadata?.frequencyJustification || 'Not specified'
  }
}
```

### Integration with Task Validator
```typescript
// From TESTDES-3001, add ecological validation step
class TaskValidator {
  async validate(task: SearchTask): Promise<ValidationResult> {
    const checks = await Promise.all([
      this.checkGrepBaseline(task),
      this.checkSearchAdvantage(task),
      this.checkDeterminism(task),
      this.checkEcological(task)  // NEW: Ecological validation
    ])

    return {
      passed: checks.every(c => c.passed),
      ecologicalScore: checks[3].score
    }
  }
}
```

## Files to Create/Modify

**New Files**:
- `packages/cli/src/search-optimization/validation/ecological.ts` - Main implementation
- `packages/cli/src/search-optimization/validation/__tests__/ecological.test.ts` - Unit tests
- `docs/research/task-realism-survey.md` - Survey template and methodology

**Updated Files**:
- `packages/cli/src/search-optimization/validation/task-validator.ts` - Add ecological validation step (from TESTDES-3001)
- `packages/cli/src/search-optimization/validation/index.ts` - Export ecological validation
- `packages/cli/src/search-optimization/taxonomy/categories.ts` - Add frequency metadata to TaskCategory

## Dependencies

**Required Tickets**:
- TESTDES-3001: Task validator (provides validation framework to integrate with)

**Task Metadata Requirements**:
Tasks should include:
```typescript
interface TaskMetadata {
  basedOnRealScenario?: boolean
  scenarioReference?: string  // Link to PR, issue, or description
  scenarioType?: 'code-review' | 'debugging' | 'refactoring' | 'onboarding'
  frequency?: 'daily' | 'weekly' | 'monthly' | 'rare'
  frequencyJustification?: string
}
```

## Agent Assignments

**Primary Agent**: general-purpose
**Responsibilities**: TypeScript implementation of validation checks, integration with task validator

**Secondary Agent**: technical-researcher
**Responsibilities**: Design developer survey framework, create survey template documentation, define frequency classification methodology

**Supporting Agents**:
- unit-test-runner: Execute and validate tests
- verify-ticket: Check acceptance criteria
- commit-ticket: Create conventional commit

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Subjective frequency classification | Inconsistent prioritization | Provide clear examples and guidelines for each frequency tier |
| Low survey response rate | Insufficient external validation | Start with internal team, incentivize participation |
| Bias in "would do this" responses | False validation of unrealistic tasks | Include free-text comments for qualitative insights |
| Automated checks too rigid | False negatives on valid tasks | Make checks warnings, not blockers; allow overrides with justification |

## Testing Strategy

**Unit Tests**:
- Realism check functions with various task metadata
- Frequency classification with edge cases
- Survey scoring calculation
- Integration with task validator

**Integration Tests**:
- Full ecological validation on sample tasks
- Report generation with all checks
- Survey data aggregation

**Manual Validation**:
- Pilot survey with 3-5 developers
- Test survey template clarity
- Validate frequency classifications against actual usage

## Success Metrics

- [x] Automated ecological checks work on all existing tasks
- [x] Survey template is clear and completable in <5 minutes
- [x] Frequency classification covers all task types
- [x] Integration with task validator provides actionable feedback
- [x] Documentation enables others to run surveys

## Workflow Status

- [x] Implementation completed
- [x] Tests passing (52/52 tests)
- [x] Integration verified with task-validator
- [x] Documentation created
- [x] Verified

## References

**Code References**:
- `/workspace/packages/cli/src/search-optimization/validators.ts:222-235` - Validation patterns
- `/workspace/packages/cli/src/search-optimization/competition-runner.ts:113-178` - Task execution context

**Planning References**:
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md:62-90` - Ecological validity definition
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:329-346` - Ecological validation architecture
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/plan.md` - Phase 3.2 requirements

**Research Background**:
- Ecological validity in psychological testing
- Task analysis methodology from HCI research
- Frequency-based prioritization from user research

## Notes

Ecological validation is what prevents us from optimizing for impressive-looking benchmarks that don't matter in practice. The goal is to ensure every task in our suite represents real work that developers actually do.

The dual approach (automated checks + manual surveys) balances scalability with validity:
- Automated checks catch obvious issues quickly
- Surveys provide external validation and qualitative insights

Frequency classification enables prioritization: a "daily" task that shows 40% improvement is more valuable than a "rare" task with the same improvement.

This ticket requires collaboration between general-purpose (code) and technical-researcher (survey design) agents to ensure both technical and methodological quality.
