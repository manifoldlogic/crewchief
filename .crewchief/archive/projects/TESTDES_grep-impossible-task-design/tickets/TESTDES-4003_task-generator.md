# TESTDES-4003: Implement Task Generator

**Status**: 📋 Planned
**Priority**: Medium
**Complexity**: Medium-High (6-8 hours)
**Phase**: 4 - Tier 2 & 3 Tasks
**Dependencies**: TESTDES-1001, TESTDES-3001

## Summary

Implement the task generator that systematically creates task variations from templates. This enables scaling the benchmark suite beyond hand-crafted tasks by applying the anti-keyword pattern and generating multiple difficulty variants from proven patterns.

## Background

Manual task creation is valuable for establishing patterns, but doesn't scale. The task generator enables:

1. **Systematic Coverage**: Generate variations across different targets, contexts, and difficulty levels
2. **Repeatable Process**: Apply consistent anti-keyword patterns and variation strategies
3. **Quality Assurance**: Auto-validate generated tasks using the task validator (TESTDES-3001)
4. **Rapid Iteration**: Quickly test new task patterns and refine based on validation results

From architecture.md (lines 129-199): The task generator creates systematic variations based on templates, applying anti-keyword patterns to avoid obvious searchable terms and creating easy/medium/hard variants.

From plan.md (lines 247-263): The generator uses templates for systematic task creation, variation generators for parameter substitution, and validation integration to ensure generated tasks meet quality standards.

**Key Insight**: Templates capture successful task patterns. Variations enable exploration of parameter space (different targets, contexts, difficulties) without manual creation overhead.

## Acceptance Criteria

- [ ] Template system defined for each of the 6 task categories (from TESTDES-1001)
- [ ] Variation generators implemented:
  - [ ] Parameter substitution (replace {target_function}, {scope}, {concept})
  - [ ] Target variation (different functions, classes, patterns within codebase)
  - [ ] Difficulty adjustment (easy: keyword hints, medium: conceptual, hard: inference required)
  - [ ] Context variation (code-review, debugging, refactoring scenarios)
- [ ] Anti-keyword pattern application (replace direct terms with concepts)
- [ ] Integration with task validator (TESTDES-3001) for automatic quality checking
- [ ] Template library with at least 2 templates per task category (12+ templates total)
- [ ] Generated tasks automatically validated and categorized by tier
- [ ] Documentation showing example template → variations workflow

## Technical Requirements

**Architecture**:
- TypeScript implementation in `packages/cli/src/search-optimization/generator/`
- Template-based generation with placeholder substitution
- Validation integration using task validator from TESTDES-3001
- Support for all 6 task categories from taxonomy (TESTDES-1001)

**Interfaces**:
```typescript
interface TaskTemplate {
  id: string
  category: TaskCategory  // From TESTDES-1001
  name: string
  description: string  // Template description with placeholders

  // Placeholder definitions
  placeholders: {
    [key: string]: {
      type: 'target' | 'concept' | 'scope' | 'context'
      description: string
      examples: string[]
    }
  }

  // Base task configuration
  baseConfig: {
    maxSearchAttempts: number
    maxTimeSeconds: number
    successCriteria: string[]  // Template criteria with placeholders
  }

  // Difficulty variants
  variants: {
    easy: VariantConfig    // Some keywords present
    medium: VariantConfig  // Conceptual description only
    hard: VariantConfig    // Requires inference
  }

  // Metadata
  basedOnRealScenario?: string
  antiKeywordPattern: string  // How to avoid obvious terms
}

interface VariantConfig {
  descriptionPattern: string  // With placeholders
  keywordHints: string[]      // Available keywords
  expectedGrepSuccess: number // Target success rate
  expectedSearchSuccess: number
}

interface GenerationConfig {
  template: TaskTemplate
  substitutions: Map<string, string>  // placeholder → value
  difficulty: 'easy' | 'medium' | 'hard'
  validateImmediately?: boolean  // Auto-validate with TESTDES-3001
}

interface GeneratedTask extends SearchTask {
  generatedFrom: {
    templateId: string
    substitutions: Map<string, string>
    difficulty: string
    timestamp: Date
  }
  validationResult?: ValidationResult  // From TESTDES-3001
}
```

**Template System**:

Each template captures a proven task pattern with placeholders:

```typescript
// Example: Relationship Discovery Template
const TRANSITIVE_DEPENDENCY_TEMPLATE: TaskTemplate = {
  id: 'relationship-transitive-deps',
  category: 'relationship-discovery',
  name: 'Find Transitive Dependencies',
  description: 'Find code that {dependency_relationship} on {target} {indirection_level}',

  placeholders: {
    target: {
      type: 'target',
      description: 'Function, class, or module to analyze',
      examples: ['createWorktree', 'spawnAgent', 'executeCommand', 'indexRepository']
    },
    dependency_relationship: {
      type: 'concept',
      description: 'Type of dependency relationship',
      examples: ['depends', 'could break if we change', 'is affected by', 'transitively calls']
    },
    indirection_level: {
      type: 'scope',
      description: 'How indirect the relationship is',
      examples: [
        'without importing it directly',
        'through multiple layers of abstraction',
        'indirectly',
        'via intermediate functions'
      ]
    }
  },

  baseConfig: {
    maxSearchAttempts: 5,
    maxTimeSeconds: 180,
    successCriteria: [
      'Found direct callers of {target}',
      'Identified indirect dependencies',
      'Explained dependency chain'
    ]
  },

  variants: {
    easy: {
      descriptionPattern: 'Find all code that calls {target} directly or indirectly',
      keywordHints: ['{target}', 'calls', 'uses'],
      expectedGrepSuccess: 0.5,  // Keywords help
      expectedSearchSuccess: 0.8
    },
    medium: {
      descriptionPattern: 'Find code that {dependency_relationship} {target} {indirection_level}',
      keywordHints: ['{target}'],  // Only target name
      expectedGrepSuccess: 0.3,
      expectedSearchSuccess: 0.75
    },
    hard: {
      descriptionPattern: 'What would break if we change the {target} API? Include indirect dependencies.',
      keywordHints: [],  // Pure conceptual
      expectedGrepSuccess: 0.15,
      expectedSearchSuccess: 0.7
    }
  },

  antiKeywordPattern: 'Replace function names with conceptual descriptions. Use "depends on" instead of "imports". Focus on relationships not string matching.'
}
```

**Variation Generators**:

1. **Parameter Substitution**:
```typescript
function substituteParameters(
  template: string,
  substitutions: Map<string, string>
): string {
  let result = template
  for (const [placeholder, value] of substitutions) {
    result = result.replace(new RegExp(`\\{${placeholder}\\}`, 'g'), value)
  }
  return result
}
```

2. **Target Variation**:
```typescript
async function generateTargetVariations(
  template: TaskTemplate,
  codebase: string
): Promise<GeneratedTask[]> {
  // Find targets in codebase matching placeholder type
  const targets = await findTargets(codebase, template.placeholders.target.type)

  return targets.map(target => {
    const substitutions = new Map([['target', target.name]])
    return generateTask({ template, substitutions, difficulty: 'medium' })
  })
}
```

3. **Difficulty Adjustment**:
```typescript
function generateDifficultyVariants(
  template: TaskTemplate,
  substitutions: Map<string, string>
): GeneratedTask[] {
  return ['easy', 'medium', 'hard'].map(difficulty => {
    const variant = template.variants[difficulty]
    const description = substituteParameters(variant.descriptionPattern, substitutions)

    return {
      ...createBaseTask(template, substitutions),
      description,
      difficulty,
      expectedGrepSuccess: variant.expectedGrepSuccess,
      expectedSearchSuccess: variant.expectedSearchSuccess
    }
  })
}
```

4. **Anti-Keyword Application**:
```typescript
function applyAntiKeywordPattern(
  description: string,
  pattern: string
): string {
  // Example patterns:
  // "createWorktree" → "code that creates parallel git repositories"
  // "authentication" → "verifies user identity"
  // "retry logic" → "code that re-attempts failed operations"

  // Implementation uses pattern guides from template
  return applyTransformations(description, parsePattern(pattern))
}
```

## Implementation Notes

### Generation Workflow

```typescript
async function generateTask(config: GenerationConfig): Promise<GeneratedTask> {
  // 1. Substitute placeholders
  const description = substituteParameters(
    config.template.variants[config.difficulty].descriptionPattern,
    config.substitutions
  )

  // 2. Apply anti-keyword pattern
  const refinedDescription = applyAntiKeywordPattern(
    description,
    config.template.antiKeywordPattern
  )

  // 3. Create task
  const task: GeneratedTask = {
    id: generateTaskId(config.template.id, config.substitutions),
    name: generateTaskName(config.template, config.substitutions),
    category: config.template.category,
    difficulty: config.difficulty,
    description: refinedDescription,
    ...config.template.baseConfig,
    generatedFrom: {
      templateId: config.template.id,
      substitutions: config.substitutions,
      difficulty: config.difficulty,
      timestamp: new Date()
    }
  }

  // 4. Validate if requested
  if (config.validateImmediately) {
    task.validationResult = await validateTask({
      task,
      tier: inferTier(config.difficulty),
      iterations: 5
    })
  }

  return task
}
```

### Template Library Structure

Organize templates by category:

```
generator/
├── templates/
│   ├── relationship-discovery/
│   │   ├── transitive-dependencies.ts
│   │   ├── call-chain-analysis.ts
│   │   └── impact-analysis.ts
│   ├── conceptual-similarity/
│   │   ├── pattern-matching.ts
│   │   ├── similar-implementations.ts
│   │   └── retry-patterns.ts
│   ├── architectural-understanding/
│   │   ├── data-flow.ts
│   │   ├── initialization-sequence.ts
│   │   └── system-interactions.ts
│   ├── negative-space/
│   │   ├── missing-error-handling.ts
│   │   └── unprotected-operations.ts
│   ├── ambiguity-resolution/
│   │   ├── multiple-implementations.ts
│   │   └── context-disambiguation.ts
│   └── cross-cutting/
│       ├── scattered-concerns.ts
│       └── cross-file-patterns.ts
├── variations/
│   ├── parameter-substitution.ts
│   ├── target-finder.ts
│   ├── difficulty-adjuster.ts
│   └── anti-keyword.ts
├── index.ts
└── __tests__/
    └── generator.test.ts
```

### Example Generation Flow

```typescript
// Start with template
const template = TRANSITIVE_DEPENDENCY_TEMPLATE

// Define substitutions
const substitutions = new Map([
  ['target', 'createWorktree'],
  ['dependency_relationship', 'could break if we change'],
  ['indirection_level', 'without importing it directly']
])

// Generate all difficulty variants
const tasks = generateDifficultyVariants(template, substitutions)

// Validate generated tasks
for (const task of tasks) {
  const validation = await validateTask({
    task,
    tier: 'tier1-impossible',
    iterations: 3  // Quick validation
  })

  if (validation.passed) {
    console.log(`✅ ${task.id}: PASSED`)
  } else {
    console.log(`❌ ${task.id}: FAILED - ${validation.recommendations}`)
  }
}
```

### Integration with Validator

```typescript
// Auto-validation during generation
async function generateAndValidate(
  template: TaskTemplate,
  codebase: string
): Promise<GeneratedTask[]> {
  // 1. Generate variations
  const tasks = await generateAllVariations(template, codebase)

  // 2. Validate in parallel
  const validated = await Promise.all(
    tasks.map(async task => {
      const validation = await validateTask({
        task,
        tier: inferTier(task.difficulty),
        iterations: 5
      })

      return {
        ...task,
        validationResult: validation
      }
    })
  )

  // 3. Filter to only passing tasks
  return validated.filter(t => t.validationResult?.passed)
}
```

### Template Best Practices

From architecture.md (lines 166-198) and quality-strategy.md:

1. **Start with Real Scenarios**: Base templates on actual code reviews, debugging sessions
2. **Apply Anti-Keyword Pattern**: Replace direct terms with conceptual descriptions
3. **Create Difficulty Variants**: Easy (keywords), Medium (conceptual), Hard (inference)
4. **Validate Grep-Impossibility**: Generated tasks should defeat grep at intended difficulty
5. **Objective Criteria**: Use binary checks, avoid subjective judgment
6. **Multiple Targets**: Each template should work across different code targets

## Files to Create/Modify

**New Files**:
- `packages/cli/src/search-optimization/generator/templates/relationship-discovery/transitive-dependencies.ts`
- `packages/cli/src/search-optimization/generator/templates/relationship-discovery/call-chain-analysis.ts`
- `packages/cli/src/search-optimization/generator/templates/conceptual-similarity/pattern-matching.ts`
- `packages/cli/src/search-optimization/generator/templates/conceptual-similarity/similar-implementations.ts`
- `packages/cli/src/search-optimization/generator/templates/architectural-understanding/data-flow.ts`
- `packages/cli/src/search-optimization/generator/templates/architectural-understanding/initialization-sequence.ts`
- `packages/cli/src/search-optimization/generator/templates/negative-space/missing-error-handling.ts`
- `packages/cli/src/search-optimization/generator/templates/ambiguity-resolution/multiple-implementations.ts`
- `packages/cli/src/search-optimization/generator/templates/cross-cutting/scattered-concerns.ts`
- `packages/cli/src/search-optimization/generator/variations/parameter-substitution.ts`
- `packages/cli/src/search-optimization/generator/variations/target-finder.ts`
- `packages/cli/src/search-optimization/generator/variations/difficulty-adjuster.ts`
- `packages/cli/src/search-optimization/generator/variations/anti-keyword.ts`
- `packages/cli/src/search-optimization/generator/types.ts`
- `packages/cli/src/search-optimization/generator/index.ts`
- `packages/cli/src/search-optimization/generator/__tests__/generator.test.ts`
- `packages/cli/src/search-optimization/generator/__tests__/variations.test.ts`
- `packages/cli/src/search-optimization/generator/__tests__/templates.test.ts`

**Updated Files**:
- `packages/cli/src/search-optimization/index.ts` - Export generator functions

## Dependencies

**Required Tickets**:
- TESTDES-1001: Task taxonomy (provides category definitions)
- TESTDES-3001: Task validator (validates generated tasks)

**Existing Code**:
- `packages/cli/src/search-optimization/taxonomy/categories.ts` - Task categories
- `packages/cli/src/search-optimization/validation/task-validator.ts` - Validation logic
- `packages/cli/src/search-optimization/tasks/` - Hand-crafted tasks to extract patterns from

## Agent Assignments

**Primary Agent**: general-purpose
**Responsibilities**: TypeScript implementation, template design, variation generators, validator integration

**Supporting Agents**:
- unit-test-runner: Execute and validate tests
- verify-ticket: Check acceptance criteria
- commit-ticket: Create conventional commit

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Generated tasks too similar | Limited coverage | Diverse placeholder examples, target variation |
| Templates too rigid | Can't capture nuance | Support custom transformation functions |
| Validation too slow | Generation bottleneck | Cache validation results, parallelize |
| Anti-keyword pattern inconsistent | Quality variance | Document patterns, provide examples |
| Template proliferation | Hard to maintain | Start small (2 per category), iterate based on usage |

## Testing Strategy

**Unit Tests**:
- Parameter substitution with various placeholders
- Difficulty variant generation
- Anti-keyword pattern application
- Template validation (well-formed)

**Integration Tests**:
- Generate task from template end-to-end
- Auto-validation integration
- Multiple difficulty variants from one template
- Target variation across codebase

**Validation Tests**:
- Generated tasks pass task validator (TESTDES-3001)
- Difficulty calibration matches expectations
- Anti-keyword pattern actually defeats grep
- Template reuse across different targets

## Success Metrics

- [ ] 12+ templates created (2 per category minimum)
- [ ] Can generate 3 difficulty variants from single template
- [ ] Generated tasks pass validation >80% of time
- [ ] Target variation produces 5+ unique tasks per template
- [ ] Anti-keyword pattern reduces grep success by >30%
- [ ] Documentation enables contributor to create new template

## References

**Code References**:
- `/workspace/packages/cli/src/search-optimization/taxonomy/categories.ts` - Task categories (TESTDES-1001)
- `/workspace/packages/cli/src/search-optimization/validation/task-validator.ts` - Validation (TESTDES-3001)

**Planning References**:
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:129-199` - Task generator architecture
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:166-198` - Example task generation
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:366-488` - Task design patterns
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/plan.md:247-263` - Task generator deliverable
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md:369-401` - Failure types to avoid
- `.crewchief/projects/TESTDES_grep-impossible-task-design/tickets/TESTDES_TICKET_INDEX.md:123-128` - Ticket context

## Notes

The task generator transforms task creation from artisanal (hand-crafted) to systematic (template-based). This is critical for scaling beyond the initial 8-10 hand-crafted tasks to 50+ tasks across all categories.

**Key Principles**:
1. **Templates capture patterns**: Extract successful patterns from hand-crafted tasks
2. **Variations enable exploration**: Test different targets, contexts, difficulties
3. **Validation ensures quality**: Auto-validate prevents bad tasks entering suite
4. **Anti-keyword prevents gaming**: Force conceptual understanding, not string matching

**From architecture.md (lines 149-165)**: Generation strategy starts with real scenarios, applies anti-keyword pattern, creates variations, and validates grep-impossibility. If grep succeeds >30%, task is too easy.

The generator is the bridge between manual task design (Phases 1-3) and scaled benchmark creation (Phases 4-5). It enables rapid iteration and systematic exploration of the task design space.
