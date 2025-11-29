# Ticket: TESTDES-3003: Implement Validation Report Generator

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement a comprehensive validation report generator that produces markdown reports showing per-task validation results across all 5 quality dimensions, identifies common failure patterns across the task suite, generates suite-level summaries with overall pass rates, and provides actionable recommendations for improving failed tasks.

## Background
Once tasks are validated by TESTDES-3001, we need human-readable reports to understand:
- Which tasks pass/fail each validation dimension
- What patterns emerge across failures (e.g., "conceptual-similarity tasks consistently have high grep success")
- Overall suite health (how many tasks are production-ready)
- Specific recommendations for fixing problem tasks

These reports are critical for:
1. **Quality Control**: Understanding which tasks are ready for benchmarking
2. **Iterative Improvement**: Identifying systematic issues in task design
3. **Communication**: Explaining validation results to stakeholders
4. **Documentation**: Creating audit trail of task validation over time

**Reference**: See quality-strategy.md Section "Reporting" (lines 480-497) and architecture.md Section "Validation Pipeline" (lines 286-364) for report structure requirements.

## Acceptance Criteria
- [x] Generate markdown reports with clear structure (summary, per-task results, patterns, recommendations)
- [x] Per-task section shows pass/fail for all 5 validation dimensions (grep-impossibility, search-suitability, objective-criteria, ecological-validity, statistical-power)
- [x] Pattern identification section groups tasks by common failure types (too-easy, too-hard, insufficient-advantage, unreliable, ecologically-invalid)
- [x] Suite-level summary shows overall statistics (total tasks, pass rate, by category breakdown)
- [x] Actionable recommendations section provides specific fixes for each failed task
- [x] Reports are saved to `packages/cli/src/search-optimization/validation/reports/` with timestamp
- [x] Unit tests validate report structure and content generation

## Technical Requirements
- TypeScript implementation in `packages/cli/src/search-optimization/validation/reporter.ts`
- Accept ValidationResult[] from TaskValidator (TESTDES-3001) as input
- Generate markdown using template literals or markdown library
- Implement pattern detection logic to group failures by type
- Support multiple output formats: markdown file, console output, JSON data
- Include report metadata (timestamp, framework version, total tasks validated)
- Export ReportGenerator class and utility functions
- Use Vitest for unit tests

## Implementation Notes

### Report Structure

```markdown
# Task Validation Report
Generated: 2025-11-07 14:32:00
Framework Version: 1.0.0
Total Tasks: 15

## Summary
- ✅ Passed: 12 (80%)
- ❌ Failed: 3 (20%)
- By Category:
  - Relationship Discovery: 4/5 passed (80%)
  - Conceptual Similarity: 3/3 passed (100%)
  - Architectural Understanding: 3/4 passed (75%)
  - Negative Space: 2/3 passed (67%)

## Per-Task Results
### ✅ TASK-001: Find Transitive Dependencies
- Grep Impossibility: ✅ Pass (grep success: 15%)
- Search Suitability: ✅ Pass (search advantage: 45%)
- Objective Criteria: ✅ Pass (deterministic: 98%)
- Ecological Validity: ✅ Pass (realism score: 4.2/5)
- Statistical Power: ✅ Pass (p < 0.01)

### ❌ TASK-005: Find Retry Implementations
- Grep Impossibility: ❌ Fail (grep success: 65% - too easy)
- Search Suitability: ⚠️  Warn (search advantage: 18% - low)
- Objective Criteria: ✅ Pass
- Ecological Validity: ✅ Pass
- Statistical Power: ✅ Pass
**Recommendation**: Add anti-keyword constraints. Replace "retry" mentions with conceptual description.

## Failure Patterns
### Pattern: Task Too Easy (2 tasks)
Tasks where grep succeeds >60%, indicating insufficient difficulty:
- TASK-005: Find Retry Implementations (65% grep success)
- TASK-012: Locate Authentication Middleware (72% grep success)

**Common Issue**: Tasks contain obvious keywords
**Fix Strategy**: Use conceptual descriptions, avoid direct terminology

### Pattern: Insufficient Search Advantage (1 task)
Tasks where search provides <20% improvement over grep:
- TASK-009: Find Config Files (12% advantage)

**Common Issue**: Task doesn't leverage semantic search strengths
**Fix Strategy**: Redesign to emphasize relationships or concepts

## Recommendations
1. **TASK-005** (Too Easy): Replace "retry" with "re-attempts failed operations"
2. **TASK-009** (Low Advantage): Change to "Find code that depends on configuration state"
3. **TASK-012** (Too Easy): Change to "Find code that verifies user authorization"
```

### Report Components

**1. Summary Section**
- Total tasks, pass/fail counts
- Percentage breakdown
- Category-level statistics
- Quick health assessment

**2. Per-Task Section**
- Task ID and name
- 5 validation dimensions with pass/fail/warn
- Specific metric values (grep success %, search advantage %, etc.)
- Individual recommendations for failed tasks

**3. Pattern Identification**
- Group tasks by failure type (from quality-strategy.md lines 370-401):
  - Type 1: Task Too Easy (grep >60%)
  - Type 2: Task Too Hard (both fail)
  - Type 3: Insufficient Advantage (search <20% better)
  - Type 4: Unreliable Results (>20% variance)
  - Type 5: Ecologically Invalid (low realism score)
- Show tasks in each group
- Explain common root cause
- Suggest fix strategy

**4. Actionable Recommendations**
- Specific per-task fixes
- Reference to task design patterns
- Priority ranking (critical/important/minor)

### ReportGenerator Interface

```typescript
interface ReportConfig {
  format: 'markdown' | 'json' | 'console'
  outputDir?: string
  includePatterns?: boolean
  includeRecommendations?: boolean
  verbose?: boolean
}

class ReportGenerator {
  constructor(config: ReportConfig)

  generate(results: ValidationResult[]): Report
  save(report: Report, filename?: string): Promise<void>
  print(report: Report): void

  private generateSummary(results: ValidationResult[]): SummarySection
  private generatePerTaskResults(results: ValidationResult[]): TaskResultSection[]
  private identifyPatterns(results: ValidationResult[]): PatternSection[]
  private generateRecommendations(results: ValidationResult[]): RecommendationSection[]
}
```

### Pattern Detection Logic

Group failures by examining ValidationResult fields:
- **Too Easy**: `grepBaseline.success > 0.6`
- **Too Hard**: `grepBaseline.success < 0.1 && searchAvailable.success < 0.5`
- **Insufficient Advantage**: `searchAvailable.success - grepBaseline.success < 0.2`
- **Unreliable**: `variance > 0.2`
- **Ecologically Invalid**: `ecologicalScore < 3.0`

### Integration with TaskValidator

```typescript
// Usage pattern
const validator = new TaskValidator()
const results = await validator.validateSuite(TIER1_SUITE)

const reporter = new ReportGenerator({ format: 'markdown' })
const report = reporter.generate(results)
await reporter.save(report, 'tier1-validation-report.md')
```

### Output Location
- Save reports to `packages/cli/src/search-optimization/validation/reports/`
- Naming convention: `validation-report-{suite-name}-{timestamp}.md`
- Also support custom filenames

## Dependencies
- TESTDES-3001: Task Validator (provides ValidationResult[] input)

## Risk Assessment
- **Risk**: Report format might need refinement after first use
  - **Mitigation**: Keep report generation modular, easy to adjust sections independently

- **Risk**: Pattern detection might miss edge cases
  - **Mitigation**: Start with simple heuristics, refine based on actual validation data

- **Risk**: Recommendations might be too generic
  - **Mitigation**: Reference specific sections of architecture.md for detailed guidance

## Files/Packages Affected
**Files to Create**:
- `packages/cli/src/search-optimization/validation/reporter.ts`
- `packages/cli/src/search-optimization/validation/reports/` (directory)
- `packages/cli/src/search-optimization/validation/__tests__/reporter.test.ts`
