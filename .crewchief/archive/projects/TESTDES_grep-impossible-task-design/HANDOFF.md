# TESTDES Project Handoff Documentation

**Audience**: Future maintainers and contributors to the grep-impossible task framework
**Last Updated**: November 7, 2025
**Framework Version**: 1.0.0

## Purpose

This document provides guidance for maintaining and extending the grep-impossible task framework. It covers common maintenance scenarios, extension patterns, troubleshooting, and ownership information.

## Framework Overview

The grep-impossible task framework validates semantic code search through three tiers of benchmarks:
- **Tier 1**: Grep-impossible tasks (<30% grep success)
- **Tier 2**: Grep-hard tasks (30-60% grep success)
- **Tier 3**: Real-world tasks (natural tool selection)

**Current Status**: Production-ready with 35+ validated tasks across 6 categories.

## Code Locations

### Primary Implementation
```
/workspace/packages/cli/src/search-optimization/
├── tasks/                    # Task implementations by category
├── benchmarks/               # Three-tier suite definitions
├── evaluation/               # Baseline comparison and metrics
├── validation/               # Five-dimension task validator
├── types.ts                  # Core type definitions
└── genetic-iterator.ts       # Genetic optimization integration
```

### Documentation
```
/workspace/docs/
├── search-optimization/      # Framework guides
│   ├── README.md
│   ├── task-design-guide.md
│   ├── validation-guide.md
│   └── benchmark-usage.md
├── architecture/
│   └── SEARCH_EVALUATION.md  # Architecture integration
└── research/
    └── grep-impossible-tasks-report.md  # Research findings
```

### Tests
```
/workspace/packages/cli/src/search-optimization/
├── tasks/**/__tests__/       # Task-specific tests
├── validation/__tests__/     # Validator tests
└── evaluation/__tests__/     # Evaluation framework tests
```

## Common Maintenance Scenarios

### 1. Adding a New Task

**When**: You want to create a new grep-impossible task

**Steps**:
1. Choose appropriate category (see Task Categories below)
2. Create task file: `tasks/{category}/{task-name}.ts`
3. Follow task template from existing tasks
4. Apply anti-keyword pattern (see Design Patterns)
5. Create objective success criteria
6. Add to appropriate tier suite: `benchmarks/tier{1,2,3}-{name}.ts`
7. Create test file: `tasks/{category}/__tests__/{task-name}.test.ts`
8. Run validation: `pnpm test search-optimization/validation`
9. Update task count in documentation

**Template Structure**:
```typescript
import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_NAME: SearchTask = {
  id: 'tier1-task-name',
  name: 'Human-Readable Task Name',
  category: 'relationship-discovery', // or other category
  difficulty: 'hard',

  description: 'Conceptual description without keywords...',

  searchTarget: {
    type: 'pattern',
    pattern: /relevant.*pattern/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Task prompt for agent...',
    validator: {
      type: 'explanation',
      mentionsFiles: ['file1.ts', 'file2.ts'],
      mentionsPattern: /expected.*concepts/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.25,  // <0.30 for Tier 1
  expectedSearchSuccess: 0.75, // >0.70 for Tier 1

  successValidator: createTaskValidator({
    searchTarget: { type: 'pattern', pattern: /relevant.*pattern/i },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['file1.ts', 'file2.ts'],
        mentionsPattern: /expected.*concepts/i,
      },
    },
  }),

  basedOnRealScenario: true,
}
```

**Reference**: See `docs/search-optimization/task-design-guide.md` for comprehensive guidance.

### 2. Updating Existing Task

**When**: A task fails validation, needs refinement, or codebases changes break it

**Steps**:
1. Run validation to identify issue: `pnpm search-optimize:validate {task-id}`
2. Check which quality dimension failed (see Troubleshooting)
3. Update task definition in `tasks/{category}/{task-name}.ts`
4. Common fixes:
   - **Construct validity failure**: Adjust `expectedGrepSuccess` down
   - **Discriminant validity failure**: Improve anti-keyword pattern
   - **Ecological validity failure**: Add `basedOnRealScenario` context
   - **Reliability failure**: Make success criteria more deterministic
5. Re-run validation: `pnpm search-optimize:validate {task-id}`
6. Update tests if behavior changed
7. Document reason for change in commit message

**Reference**: See `docs/search-optimization/validation-guide.md` section "Troubleshooting Guide by Failure Type".

### 3. Adding a New Task Category

**When**: You identify a new pattern of grep-impossible tasks that doesn't fit existing categories

**Current Categories**:
1. relationship-discovery
2. conceptual-similarity
3. architectural-understanding
4. negative-space
5. cross-cutting-concerns
6. ambiguity-resolution

**Steps**:
1. Document category definition and examples
2. Add category to `types.ts` in `TaskCategory` type
3. Create directory: `tasks/{new-category}/`
4. Implement 2-3 initial tasks demonstrating the pattern
5. Add category to task design guide documentation
6. Update framework README with new category count
7. Add category-specific validation if needed

**Validation**: New categories should demonstrate:
- Distinct pattern not covered by existing categories
- Grep-impossible characteristics
- Generalization across multiple codebases
- Real-world developer scenarios

### 4. Extending to New Programming Language

**When**: You want to validate framework on Python, Rust, Go, Java, etc.

**Steps**:
1. Choose representative codebase in target language
2. Identify 5-10 tasks that should generalize (see Generalization Guide)
3. Adapt task descriptions and success criteria to target language idioms
4. Run cross-project validation suite
5. Document language-specific adaptations
6. Calculate generalization percentage
7. Add language support to documentation

**Generalization Priorities** (most → least generalizable):
1. Relationship Discovery tasks (universal concept)
2. Architectural Understanding tasks (language-agnostic patterns)
3. Negative Space tasks (structure-based, not syntax-based)
4. Conceptual Similarity tasks (requires language-specific idioms)
5. Ambiguity Resolution tasks (heavily language/framework dependent)

**Reference**: See `docs/search-optimization/benchmark-usage.md` section "Cross-Project Validation".

### 5. Updating Validation Criteria

**When**: Quality dimension thresholds need adjustment based on empirical data

**Current Thresholds**:
- Construct Validity: Grep success < 30% (Tier 1), 30-60% (Tier 2)
- Discriminant Validity: Search advantage >40% (p < 0.05)
- Ecological Validity: basedOnRealScenario = true + practitioner survey score >3.5/5
- Test-Retest Reliability: Correlation >0.7 across runs
- Statistical Power: Minimum 5 iterations (10 for publication)

**Steps**:
1. Analyze validation results across all tasks
2. Identify systematic threshold issues
3. Update thresholds in `validation/task-validator.ts`
4. Update documentation to reflect new thresholds
5. Re-validate all existing tasks
6. Document rationale for threshold changes

**Warning**: Threshold changes affect all tasks. Always document reasoning and re-validate entire suite.

## Design Patterns and Best Practices

### Anti-Keyword Pattern

Make tasks grep-resistant by describing concepts rather than using obvious keywords:

**Bad** (keyword-heavy):
```typescript
description: 'Find the retry logic with exponential backoff mechanism'
```

**Good** (conceptual):
```typescript
description: 'Find code that re-attempts failed operations with increasing delays between attempts'
```

**Technique**: Replace technical terms with descriptive phrases, use synonyms, focus on behavior rather than names.

### Objective Success Criteria

Prefer automated validation over subjective judgment:

**Hierarchy** (best → worst):
1. **Code Changes**: Agent modifies files and tests pass
2. **File Identification**: Agent identifies specific files by path
3. **Pattern Matching**: Agent mentions specific functions/patterns
4. **Explanation**: Agent explanation includes required concepts

**Example**:
```typescript
validator: {
  type: 'explanation',
  mentionsFiles: ['src/auth/jwt.ts', 'src/middleware/authenticate.ts'],
  mentionsPattern: /token.*validation|verify.*signature/i,
}
```

### Real-World Grounding

Base all tasks on actual development scenarios:

**Sources**:
- Pull requests (code review tasks)
- Bug reports (debugging tasks)
- Refactoring discussions (architectural tasks)
- Onboarding questions (understanding tasks)

**Documentation**:
```typescript
basedOnRealScenario: true,
scenarioDescription: 'From PR #123: Understanding OAuth flow for rate limiting',
```

## Troubleshooting Guide

### Task Validation Failures

**Symptom**: Task fails construct validity (grep succeeds too often)

**Diagnosis**: Grep success rate > 30% for Tier 1 task

**Fix**:
1. Check if task description contains keywords
2. Apply stronger anti-keyword pattern
3. Consider if task truly requires semantic understanding
4. May need to move to Tier 2 (grep-hard) if grep has 30-60% success

---

**Symptom**: Task fails discriminant validity (search doesn't outperform grep)

**Diagnosis**: Search success rate not significantly higher than grep (p >= 0.05)

**Fix**:
1. Task may not require semantic understanding
2. Check if success criteria are too lenient (both grep and search pass)
3. Refine success criteria to require deeper understanding
4. Consider removing task if truly not search-appropriate

---

**Symptom**: Task fails ecological validity (unrealistic scenario)

**Diagnosis**: Practitioners rate task relevance <3.5/5 or no real-world basis

**Fix**:
1. Add `basedOnRealScenario` with link to PR/issue
2. Rephrase task as actual developer would encounter it
3. Remove artificial constraints
4. Test task description with developer not familiar with framework

---

**Symptom**: Task fails reliability (inconsistent results)

**Diagnosis**: Test-retest correlation <0.7 across runs

**Fix**:
1. Success criteria may be too subjective
2. Add more specific file/pattern requirements
3. Reduce dependence on explanation quality
4. Use code changes or file identification instead

---

**Symptom**: Task fails statistical power (insufficient sample size)

**Diagnosis**: Need more iterations to achieve p < 0.05 significance

**Fix**:
1. Increase iterations (minimum 5, recommended 10)
2. Use mock mode for fast iteration during development
3. Budget for additional LLM API costs for real validation

### Implementation Issues

**Symptom**: Task times out (exceeds maxTimeSeconds)

**Fix**:
1. Increase `maxTimeSeconds` (default: 300)
2. Check if task is too complex (split into smaller tasks)
3. Verify search indexing is complete and up-to-date

---

**Symptom**: Validator incorrectly passes/fails

**Fix**:
1. Check validator logic in `validators.ts`
2. Test validator independently with known good/bad outputs
3. Add test cases for edge cases
4. Consider making criteria more explicit

---

**Symptom**: Search tool not being selected

**Fix**:
1. Verify task truly requires semantic understanding
2. Check tool descriptions are current
3. Confirm task doesn't accidentally hint at grep usage
4. May be correct behavior if task is grep-solvable

## Framework Extension Points

### Custom Validators

Add new validator types in `validators.ts`:

```typescript
export function createCustomValidator(
  config: CustomValidatorConfig
): TaskValidator {
  return (result: TaskResult) => {
    // Custom validation logic
    return {
      success: boolean,
      confidence: number,
      reason: string,
    }
  }
}
```

### Custom Metrics

Add new metrics in `evaluation/metrics.ts`:

```typescript
export interface CustomMetrics {
  // Add new metric fields
}

export function calculateCustomMetrics(
  results: TaskResult[]
): CustomMetrics {
  // Calculation logic
}
```

### Custom Baseline Tools

Add new baseline comparison tools in `evaluation/baseline-runner.ts`:

```typescript
export async function runCustomBaseline(
  task: SearchTask,
  options: BaselineOptions
): Promise<BaselineResult> {
  // Custom tool execution
}
```

## Testing Strategy

### Unit Tests
Location: `**/__tests__/*.test.ts`

Run: `pnpm test search-optimization`

Coverage: 95%+ for validation infrastructure

### Integration Tests
Location: `benchmarks/__tests__/`

Run: `pnpm test search-optimization/benchmarks`

Coverage: All tier suites executable

### Validation Tests
Location: `validation/__tests__/`

Run: `pnpm test search-optimization/validation`

Coverage: All five quality dimensions

### Manual Testing Checklist
- [ ] New task passes all 5 quality dimensions
- [ ] Task description uses anti-keyword pattern
- [ ] Success criteria are objective and automatable
- [ ] Based on real-world scenario
- [ ] Grep baseline confirms difficulty tier
- [ ] Search provides significant advantage (p < 0.05)
- [ ] Results consistent across multiple runs
- [ ] Documentation updated with task count

## Performance Considerations

### Cost Management

**Single Task Validation**: $0.30-0.75 (5 iterations)
**Tier 1 Suite**: $12-20 (10 tasks)
**Full Three-Tier Suite**: $45-75 (35 tasks)

**Cost Reduction Strategies**:
1. Use mock mode during development (free)
2. Batch validations to reduce API overhead
3. Cache baseline results (grep doesn't change)
4. Run full validation only before releases

### Execution Time

**Single Task**: 5-10 minutes (5 iterations)
**Tier 1 Suite**: 1-2 hours (10 tasks)
**Full Suite**: 4-6 hours (35 tasks)

**Optimization**:
1. Parallel task execution where possible
2. Mock mode for rapid iteration
3. Incremental validation (only changed tasks)

## Known Issues and Limitations

### 1. Single Codebase Focus

**Issue**: Current tasks primarily validated on CrewChief TypeScript codebase

**Impact**: Generalization to other codebases requires validation

**Mitigation**: Cross-project validation documented in benchmark usage guide

**Future**: Expand validation to Python, Rust, Go codebases

### 2. LLM Agent Specific

**Issue**: Framework assumes LLM-based agents with tool calling

**Impact**: Not tested with human developers or other agent architectures

**Mitigation**: Task design based on real developer workflows should transfer

**Future**: Study with human developers for validation

### 3. Cost Constraints

**Issue**: Full validation requires LLM API calls ($45-75 for complete suite)

**Impact**: Limits large-scale experimentation and continuous validation

**Mitigation**: Mock mode for development, strategic real-mode validation

**Future**: Explore local LLM options for cost-free validation

### 4. Language Coverage

**Issue**: Tasks designed primarily for TypeScript, partial Rust/Python

**Impact**: Direct applicability limited to similar languages

**Mitigation**: Category-based design enables language adaptation

**Future**: Create language-specific adaptation guidelines

## Outstanding Work

### Near-Term (0-3 months)

1. **Expand Tier 2 Tasks**: 8 Tier 1 tasks implemented, design calls for 12 more Tier 2 tasks
2. **Expand Tier 3 Tasks**: Design calls for 15 real-world Tier 3 tasks
3. **Multi-Language Validation**: Adapt tasks to Python and Rust codebases
4. **Practitioner Survey**: Validate ecological validity with developer surveys

### Medium-Term (3-6 months)

1. **Public Benchmark Suite**: Release standardized benchmark for research community
2. **Cross-Tool Comparison**: Test framework with other semantic search implementations
3. **Automated Task Generation**: Explore LLM-based task generation with quality checks
4. **Performance Optimization**: Reduce validation execution time and cost

### Long-Term (6-12 months)

1. **Academic Publication**: Submit methodology to ICSE, FSE, or MSR
2. **Community Contributions**: Enable open-source task submissions
3. **Continuous Improvement Pipeline**: Automated validation and task evolution
4. **Human Developer Study**: Validate with actual developers, not just LLM agents

## Ownership and Contact

**Framework Maintainer**: CrewChief core team
**Documentation Owner**: See `/workspace/docs/search-optimization/`
**Implementation Owner**: See `/workspace/packages/cli/src/search-optimization/`

**Getting Help**:
1. Check documentation: `/workspace/docs/search-optimization/`
2. Review archived planning: `/workspace/.crewchief/archive/projects/TESTDES_grep-impossible-task-design/planning/`
3. Open issue with `search-optimization` tag
4. Start discussion in repository

**Contributing**:
1. Read task design guide
2. Base tasks on real scenarios
3. Apply anti-keyword pattern
4. Create objective criteria
5. Run validation before PR
6. Update documentation

## Version History

**v1.0.0** (November 7, 2025)
- Initial production release
- 35+ validated tasks across 3 tiers
- Complete documentation suite
- Five-dimension validation system
- Research report published

**Future Versions**:
- v1.1.0: Expanded Tier 2 and Tier 3 tasks
- v1.2.0: Multi-language support (Python, Rust, Go)
- v2.0.0: Public benchmark suite and cross-tool comparison

## References

### Internal Documentation
- Framework Overview: `/workspace/docs/search-optimization/README.md`
- Task Design Guide: `/workspace/docs/search-optimization/task-design-guide.md`
- Validation Guide: `/workspace/docs/search-optimization/validation-guide.md`
- Benchmark Usage: `/workspace/docs/search-optimization/benchmark-usage.md`
- Architecture: `/workspace/docs/architecture/SEARCH_EVALUATION.md`
- Research Report: `/workspace/docs/research/grep-impossible-tasks-report.md`

### Archived Planning
- Analysis: `planning/analysis.md`
- Architecture: `planning/architecture.md`
- Quality Strategy: `planning/quality-strategy.md`
- Plan: `planning/plan.md`

### External Resources
- TREC IR Benchmarks: https://trec.nist.gov/
- CheckList Methodology: https://aclanthology.org/2020.acl-main.442/
- Cohen's d Effect Size: Standard statistical reference
- Statistical Power Analysis: Standard methodology references

---

**This handoff document provides comprehensive guidance for maintaining and extending the grep-impossible task framework. For questions or clarifications, see the Getting Help section above.**
