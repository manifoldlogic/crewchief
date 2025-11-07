# Cross-Project Validation Report

**TESTDES-5003: Validating Task Generalization Across Codebases**

*Report Date: 2025-11-07*
*Version: 1.0*
*Status: Infrastructure Complete - Awaiting Validation Runs*

---

## Executive Summary

This report documents the cross-project validation of grep-impossible search tasks to determine whether they generalize beyond the CrewChief codebase. The validation infrastructure has been implemented and is ready for execution on diverse codebases.

### Key Findings (To Be Completed)

> **Note**: This section will be populated after validation runs are executed. Current status:
> - Infrastructure: Complete ✓
> - Codebase Selection: Documented ✓
> - Task Adaptation Framework: Complete ✓
> - Validation Runs: Pending (API cost consideration)

**Expected Outcomes**:
- Demonstrate that task categories reflect universal code search patterns
- Validate grep vs search performance gaps across different languages and domains
- Identify which tasks are universally applicable vs. codebase-specific
- Provide adaptation guidelines for applying tasks to new codebases

### Research Questions

1. **Generalization**: Do grep-impossible tasks maintain their difficulty characteristics across different codebases?
2. **Consistency**: Is the search advantage (search success - grep success) consistent across codebases?
3. **Transferability**: Which task categories generalize universally vs. require codebase-specific adaptation?
4. **Patterns**: What language-specific, domain-specific, or size-related patterns affect task performance?

---

## Methodology

### 1. Codebase Selection

We selected 3 diverse codebases to represent different languages, domains, and sizes:

#### 1.1 Selection Criteria

Codebases must satisfy:
- **Publicly available**: Open-source with permissive licensing
- **Well-structured**: Clear architecture, modular design, good documentation
- **Active maintenance**: Recent commits, active community
- **Representative**: Cover common development scenarios
- **Indexable**: Compatible with maproom (tree-sitter support)

#### 1.2 Selected Codebases

| Codebase | Language | Domain | Size | Rationale |
|----------|----------|--------|------|-----------|
| **Commander.js** | TypeScript | Library | Small (<10k LOC) | Widely-used CLI framework, clean architecture, similar domain to CrewChief (CLI tools) |
| **FastAPI** | Python | Web Framework | Medium (10-50k LOC) | Modern async web framework, different domain than CrewChief, extensive use of decorators and type hints |
| **clap** | Rust | Library | Large (>50k LOC) | Feature-rich CLI parser, systems programming language, macro-heavy codebase with complex traits |

**Diversity Coverage**:
- **Languages**: TypeScript (CrewChief's language), Python (dynamic typing, different idioms), Rust (static typing, ownership model)
- **Domains**: CLI tools (Commander.js, clap), Web framework (FastAPI)
- **Sizes**: Small (Commander.js ~8k LOC), Medium (FastAPI ~20k LOC), Large (clap ~50k LOC)
- **Architectural Patterns**: Functional (Commander.js), Decorator-based (FastAPI), Trait-based (clap)

### 2. Task Selection and Adaptation

#### 2.1 Task Selection Strategy

We selected 10 tasks spanning all 6 categories from the TESTDES taxonomy:

| Category | Task Count | Examples |
|----------|-----------|----------|
| **Relationship Discovery** | 3 | Transitive dependencies, call chain tracing, API impact analysis |
| **Architectural Understanding** | 3 | Data flow analysis, initialization sequences, system interactions |
| **Negative Space** | 2 | Missing error handling, unprotected file operations |
| **Cross-Cutting Concerns** | 1 | Logging pattern identification |
| **Configuration & Environment** | 1 | Environment variable discovery |

**Selection Criteria**:
- Mix of Tier 1 (grep-impossible, expectedGrepSuccess < 0.3) and Tier 2 (grep-hard, 0.3-0.5)
- Representative of each category's core challenge
- Feasible to adapt to different codebases

#### 2.2 Task Adaptation Process

For each task, we follow a systematic adaptation process:

**Step 1: Conceptual Mapping**
- Identify the core concept being tested (e.g., "transitive dependency analysis")
- Find equivalent concepts in target codebase (e.g., "module import chains")

**Step 2: Query Adaptation**
- Translate semantic search query to target domain
- Preserve the conceptual challenge while adjusting terminology

**Step 3: Success Criteria Adjustment**
- Map expected files/functions to target codebase equivalents
- Adjust success thresholds based on codebase characteristics

**Step 4: Validation**
- Verify adapted task maintains difficulty characteristics
- Ensure task is neither trivially easy nor impossibly hard in new context

**Example Adaptation**:

```typescript
// Original Task (CrewChief)
{
  id: "arch-1001",
  category: "architectural-understanding",
  description: "Trace the data flow for worktree creation from user command to git operations",
  expectedGrepSuccess: 0.2,
  expectedSearchSuccess: 0.75
}

// Adapted Task (FastAPI)
{
  id: "arch-1001-fastapi",
  originalTaskId: "arch-1001",
  targetCodebase: "fastapi",
  category: "architectural-understanding",
  description: "Trace the data flow for request routing from HTTP reception to route handler execution",
  expectedGrepSuccess: 0.25,  // Slightly easier (smaller codebase)
  expectedSearchSuccess: 0.75,
  adaptationNotes: "Mapped worktree creation → request routing as core architectural flow",
  adaptationConfidence: 0.85
}

// Adapted Task (clap)
{
  id: "arch-1001-clap",
  originalTaskId: "arch-1001",
  targetCodebase: "clap",
  category: "architectural-understanding",
  description: "Trace the data flow for argument parsing from command invocation to value extraction",
  expectedGrepSuccess: 0.15,  // Harder (larger codebase, macro complexity)
  expectedSearchSuccess: 0.8,
  adaptationNotes: "Mapped worktree creation → arg parsing pipeline. Rust macros add complexity.",
  adaptationConfidence: 0.75
}
```

#### 2.3 Adaptation Challenges

**Challenge 1: Language-Specific Idioms**
- **Python**: Decorators, magic methods, dynamic imports
- **Rust**: Macros, traits, lifetime annotations
- **TypeScript**: Type inference, declaration merging, namespaces

**Challenge 2: Domain-Specific Architectures**
- **CLI tools**: Command-line parsing, subcommands, option handling
- **Web frameworks**: Request/response cycle, middleware, routing
- **Libraries**: Public API surface, internal implementation details

**Challenge 3: Codebase Size Effects**
- **Small codebases**: May lack complexity for certain transitive relationship tasks
- **Large codebases**: May have multiple implementations of similar concepts
- **Medium codebases**: Sweet spot for most task types

### 3. Execution Plan

#### 3.1 Validation Workflow

```
For each codebase:
  1. Index codebase with maproom
  2. Validate indexing quality (coverage, chunk granularity)
  3. For each adapted task:
     a. Run grep-only baseline (no semantic search)
     b. Run search-enabled condition
     c. Collect success metrics
     d. Record execution metadata
  4. Calculate per-codebase aggregate metrics
  5. Generate codebase-specific report

Across all codebases:
  1. Calculate generalization metrics per task
  2. Identify universal vs. specific tasks
  3. Analyze language/domain/size patterns
  4. Generate cross-project summary
```

#### 3.2 Metrics Collection

**Per-Task Metrics**:
- `grepSuccess`: Success rate with grep-only (0-1)
- `searchSuccess`: Success rate with semantic search (0-1)
- `improvement`: searchSuccess - grepSuccess
- `executionTime`: Time to complete task (seconds)
- `validationStatus`: Whether adaptation was valid

**Per-Codebase Metrics**:
- `avgGrepSuccess`: Mean grep success across all tasks
- `avgSearchSuccess`: Mean search success across all tasks
- `avgImprovement`: Mean improvement across all tasks
- `tasksDefeatingGrep`: Count where grepSuccess < 0.4

**Generalization Metrics**:
- `transferabilityScore`: 0-1 score of how well task generalizes
- `meanGrepSuccess`: Average grep success across all codebases
- `varianceGrepSuccess`: Consistency of grep difficulty
- `meanSearchAdvantage`: Average search - grep gap
- `advantageConsistency`: Whether gap is similar across codebases

#### 3.3 Cost Management

**Estimated Costs** (based on Claude API pricing):

| Configuration | Tasks | Codebases | Runs | Total Calls | Est. Cost |
|---------------|-------|-----------|------|-------------|-----------|
| **Pilot (Mock)** | 10 | 3 | 1 | 60 | $0 (mock data) |
| **Single Run** | 10 | 3 | 1 | 60 | $20-30 |
| **Statistical (3x)** | 10 | 3 | 3 | 180 | $60-90 |
| **Full (5x)** | 10 | 3 | 5 | 300 | $100-150 |

**Cost Mitigation Strategy**:
1. Start with infrastructure testing using mock data (cost: $0)
2. Run single-iteration validation to test indexing and adaptation (cost: ~$25)
3. If results promising, expand to 3-5 iterations for statistical significance
4. Consider using Claude Haiku for validation runs (60% cost reduction)

---

## Results

> **Note**: This section will be populated after validation runs are executed.

### 1. Per-Codebase Performance

#### 1.1 Commander.js (TypeScript, CLI Library, Small)

*To be completed after validation run*

**Expected Patterns**:
- Higher grep success due to smaller codebase (fewer false positives)
- Strong performance on architectural understanding tasks
- Potential struggles with transitive relationship tasks (limited depth)

**Placeholder Results**:
```
Grep Average Success:   [TBD]%
Search Average Success: [TBD]%
Improvement:            [TBD]%
Tasks Defeating Grep:   [TBD]/10
```

#### 1.2 FastAPI (Python, Web Framework, Medium)

*To be completed after validation run*

**Expected Patterns**:
- Grep challenges with decorator-heavy code
- Search advantage on cross-cutting concerns (dependency injection, middleware)
- Language-specific patterns (async/await, Pydantic models)

**Placeholder Results**:
```
Grep Average Success:   [TBD]%
Search Average Success: [TBD]%
Improvement:            [TBD]%
Tasks Defeating Grep:   [TBD]/10
```

#### 1.3 clap (Rust, CLI Library, Large)

*To be completed after validation run*

**Expected Patterns**:
- Lowest grep success (large codebase, macro complexity)
- Highest search advantage
- Strong negative space task performance (Rust's explicit error handling)

**Placeholder Results**:
```
Grep Average Success:   [TBD]%
Search Average Success: [TBD]%
Improvement:            [TBD]%
Tasks Defeating Grep:   [TBD]/10
```

### 2. Generalization Metrics

#### 2.1 Task Transferability

*To be completed after validation run*

**Universal Tasks (Transferability > 0.8)**:
- Expected: Architectural understanding, relationship discovery

**Partial Tasks (Transferability 0.4-0.8)**:
- Expected: Cross-cutting concerns (varies by language/domain)

**Limited Tasks (Transferability < 0.4)**:
- Expected: Some negative space tasks (language-specific patterns)

#### 2.2 Search Advantage Consistency

*To be completed after validation run*

**Expected Findings**:
- Search advantage should be consistent (variance < 0.05) for well-generalizing tasks
- Language-specific tasks may show higher variance
- Domain-specific tasks may have consistent advantage within similar domains

### 3. Statistical Analysis

#### 3.1 Variance Analysis

*To be completed after validation run*

**Metrics**:
- Mean success rates across codebases
- Standard deviation (consistency indicator)
- Confidence intervals
- Effect sizes

#### 3.2 Correlation Analysis

*To be completed after validation run*

**Relationships to Explore**:
- Codebase size vs. task success rate
- Language typing system (static/dynamic) vs. search advantage
- Domain similarity vs. task transferability

---

## Analysis

### 1. What Generalizes Well

*To be completed after validation run*

**Hypothesized Universal Patterns**:

1. **Architectural Understanding Tasks**
   - Reason: All codebases have architectural flows (request handling, command processing)
   - Challenge: Requires conceptual mapping between domains

2. **Relationship Discovery Tasks**
   - Reason: Transitive dependencies are universal in software
   - Challenge: Small codebases may lack sufficient depth

3. **Cross-Cutting Concerns** (Partial)
   - Reason: Logging, error handling exist everywhere
   - Challenge: Implementation patterns vary significantly

### 2. What's Codebase-Specific

*To be completed after validation run*

**Hypothesized Specific Patterns**:

1. **Negative Space Tasks**
   - Reason: Language conventions differ (e.g., Rust Result vs. Python exceptions)
   - Adaptation Required: Language-specific error handling patterns

2. **Configuration Tasks**
   - Reason: Configuration approaches vary widely (env vars, files, CLI args)
   - Adaptation Required: Domain-specific configuration patterns

### 3. Language-Specific Patterns

*To be completed after validation run*

**TypeScript**:
- Expected: Strong type inference aids semantic search
- Expected: Declaration merging creates grep challenges

**Python**:
- Expected: Decorators defeat simple grep patterns
- Expected: Dynamic typing may reduce search advantage

**Rust**:
- Expected: Macro expansion creates largest grep challenge
- Expected: Trait system ideal for semantic relationship discovery

### 4. Domain-Specific Patterns

*To be completed after validation run*

**CLI Tools (Commander.js, clap)**:
- Expected: Similar architectural patterns (command parsing, option handling)
- Expected: High task transferability between CLI codebases

**Web Frameworks (FastAPI)**:
- Expected: Different architectural patterns (middleware, routing)
- Expected: Request/response flow analogous to CLI command flow

### 5. Size-Related Patterns

*To be completed after validation run*

**Small Codebases (<10k LOC)**:
- Expected: Higher grep success (fewer false positives)
- Expected: Limited transitive relationship depth

**Medium Codebases (10-50k LOC)**:
- Expected: Balanced difficulty
- Expected: Sufficient complexity for all task types

**Large Codebases (>50k LOC)**:
- Expected: Lowest grep success
- Expected: Highest search advantage

---

## Recommendations

### 1. Adapting Tasks for New Codebases

Based on infrastructure development and adaptation process design:

#### Step-by-Step Adaptation Guide

**1. Analyze Target Codebase**
```
- Primary language and idioms
- Domain and architectural patterns
- Size and complexity
- Configuration approach
```

**2. Map Core Concepts**
```
For each task:
  - Identify the abstract concept being tested
  - Find equivalent concepts in target codebase
  - Document conceptual mapping
```

**3. Adapt Queries and Criteria**
```
- Translate semantic queries to target domain
- Adjust expected file paths
- Modify success thresholds based on codebase size
- Update validation logic
```

**4. Validate Adaptation**
```
- Use task-validator.ts to check adapted task
- Ensure difficulty characteristics are preserved
- Run pilot validation with 1-2 iterations
- Adjust based on results
```

#### Adaptation Templates

**Template 1: Architectural Understanding**
```typescript
{
  category: "architectural-understanding",

  // Generic concept
  concept: "trace data flow through system",

  // Codebase-specific mapping
  mapping: {
    "CLI tool": "command → parser → executor → output",
    "Web framework": "request → middleware → route → response",
    "Library": "public API → internal implementation → dependencies"
  },

  // Adaptation notes
  preserveDifficulty: "Ensure flow spans 3+ components",
  successCriteria: "Identify all components in flow path"
}
```

**Template 2: Relationship Discovery**
```typescript
{
  category: "relationship-discovery",

  // Generic concept
  concept: "find transitive dependencies",

  // Codebase-specific mapping
  mapping: {
    "TypeScript": "import chains across modules",
    "Python": "import statements, __init__.py aggregation",
    "Rust": "use declarations, mod hierarchy"
  },

  // Adaptation notes
  preserveDifficulty: "Ensure 3+ levels of transitivity",
  successCriteria: "Identify complete dependency chain"
}
```

### 2. Universal vs. Specific Tasks

**Use Universal Tasks When**:
- Evaluating search tools on any codebase
- Benchmarking across different languages/domains
- Comparing semantic search performance

**Use Specific Tasks When**:
- Evaluating language-specific search features
- Testing domain-specific understanding
- Measuring task realism for particular ecosystem

**Create Hybrid Suites**:
- 70% universal tasks (core semantic search capabilities)
- 30% specific tasks (codebase-relevant challenges)

### 3. Validation Best Practices

**Before Validation**:
1. Index target codebase with maproom
2. Verify indexing quality (run sample searches)
3. Adapt 2-3 pilot tasks
4. Validate adaptations with task-validator.ts

**During Validation**:
1. Start with single-run validation (cost-effective)
2. Monitor for adaptation issues (tasks too easy/hard)
3. Collect detailed execution metadata
4. Document any unexpected patterns

**After Validation**:
1. Calculate generalization metrics
2. Identify which tasks need refinement
3. Update adaptation templates
4. Share findings with research community

### 4. Cost-Effective Validation

**Strategies**:
1. Use mock data for infrastructure testing
2. Run pilot validation (1 iteration) before committing to full run
3. Use smaller models (Claude Haiku) for validation
4. Batch validations across multiple research questions
5. Consider using cached/deterministic evaluations where possible

---

## Limitations

### 1. Sample Size

**Limitation**: Only 3 codebases tested
- **Impact**: May miss language/domain patterns
- **Mitigation**: Selected codebases maximize diversity within constraint
- **Future Work**: Expand to 10+ codebases (JavaScript, Go, Java, C++)

### 2. Task Selection

**Limitation**: Only 10 tasks selected from full TESTDES suite
- **Impact**: May not represent all task difficulty levels
- **Mitigation**: Tasks span all 6 categories, mix of Tier 1 and Tier 2
- **Future Work**: Validate full suite (20+ tasks) on subset of codebases

### 3. Language-Specific Patterns

**Limitation**: Language-specific idioms may not generalize
- **Impact**: Some task categories may be language-specific
- **Examples**:
  - Python decorators have no TypeScript equivalent
  - Rust macros are unique to Rust ecosystem
  - Go's lack of generics affects relationship patterns
- **Mitigation**: Document language-specific adaptations explicitly
- **Future Work**: Create language-specific task variants

### 4. Domain-Specific Patterns

**Limitation**: CLI vs. web framework architectural differences
- **Impact**: Some architectural patterns don't map cleanly
- **Examples**:
  - CLI tools: command-line parsing, process spawning
  - Web frameworks: HTTP handling, session management
  - Libraries: API design, backward compatibility
- **Mitigation**: Create domain-specific adaptation templates
- **Future Work**: Validate within-domain generalization (multiple web frameworks)

### 5. Size-Related Patterns

**Limitation**: Small codebases may lack complexity
- **Impact**: Transitive relationship tasks may be too easy
- **Example**: Commander.js (~8k LOC) has limited dependency depth
- **Mitigation**: Adjust expected success rates for small codebases
- **Future Work**: Test on micro-projects (<1k LOC) and mega-projects (>100k LOC)

### 6. API Costs

**Limitation**: Full validation is expensive (100+ LLM calls)
- **Impact**: Limits statistical rigor (fewer iterations)
- **Mitigation**: Strategic sampling, pilot validation, mock data testing
- **Future Work**: Develop cached evaluation approach

### 7. Temporal Validity

**Limitation**: Codebases evolve over time
- **Impact**: Validation results may become outdated
- **Mitigation**: Document codebase versions, use git commit SHAs
- **Future Work**: Automated re-validation on codebase updates

### 8. Indexing Quality

**Limitation**: Maproom indexing quality varies by language
- **Impact**: Poor indexing reduces search advantage
- **Examples**:
  - TypeScript: Excellent support
  - Python: Good support
  - Rust: Good support, macro handling challenges
- **Mitigation**: Validate indexing quality before task execution
- **Future Work**: Language-specific indexing improvements

---

## Future Work

### 1. Expand Codebase Coverage

**Phase 1: Additional Languages**
- **JavaScript**: Node.js projects (e.g., Express.js)
- **Go**: Systems tools (e.g., Docker, Kubernetes)
- **Java**: Enterprise frameworks (e.g., Spring Boot)
- **C++**: Systems libraries (e.g., LLVM, V8)

**Phase 2: Additional Domains**
- **Data Processing**: pandas, polars, Apache Spark
- **Machine Learning**: scikit-learn, PyTorch
- **DevOps**: Terraform, Ansible
- **Embedded**: Zephyr, FreeRTOS

### 2. Full Suite Validation

**Current**: 10 tasks from TESTDES suite
**Target**: All 24 tasks (8 Tier 1, 8 Tier 2, 8 Tier 3)

**Benefits**:
- Complete coverage of all task categories
- Statistical validation of tier classification
- Identification of difficulty clustering patterns

### 3. Within-Domain Validation

**Hypothesis**: Tasks generalize better within same domain

**Test**:
- Compare 3 CLI tools (Commander.js, clap, yargs)
- Compare 3 web frameworks (FastAPI, Express, Axum)
- Measure within-domain vs. cross-domain transferability

### 4. Longitudinal Validation

**Hypothesis**: Task difficulty changes as codebases evolve

**Test**:
- Validate on historical codebase versions
- Track grep/search success over time
- Identify architectural changes that affect task difficulty

### 5. Automated Adaptation

**Goal**: Develop ML-based task adaptation

**Approach**:
- Train model on successful adaptations
- Input: source task + target codebase metadata
- Output: adapted task with confidence score

### 6. Community Validation

**Goal**: Open-source validation framework

**Steps**:
1. Publish validation infrastructure
2. Create contribution guidelines for new codebases
3. Aggregate results from community validations
4. Build public benchmark database

### 7. Adaptive Task Library

**Goal**: Dynamically generated tasks for any codebase

**Vision**:
```typescript
const tasks = await generateAdaptiveTasks({
  codebase: '/path/to/project',
  categories: ['architectural', 'relationship'],
  difficulty: 'grep-impossible',
  count: 10
})
```

---

## Appendix A: Codebase Details

### A.1 Commander.js

**Repository**: https://github.com/tj/commander.js
**Version**: v11.1.0 (as of validation)
**License**: MIT
**Language**: TypeScript
**LOC**: ~8,000
**Architecture**: Functional with OOP facade
**Key Features**:
- Command-line argument parsing
- Subcommand support
- Option validation
- Help text generation

**Indexing Notes**:
- Clean TypeScript, excellent tree-sitter support
- Modular structure with clear boundaries
- Extensive JSDoc documentation
- Test coverage >95%

### A.2 FastAPI

**Repository**: https://github.com/tiangolo/fastapi
**Version**: v0.104.1 (as of validation)
**License**: MIT
**Language**: Python 3.7+
**LOC**: ~20,000
**Architecture**: Decorator-based with dependency injection
**Key Features**:
- Async web framework
- Automatic OpenAPI documentation
- Pydantic model validation
- Dependency injection system

**Indexing Notes**:
- Heavy use of decorators (challenges for grep)
- Type hints throughout (aids semantic search)
- Complex dependency injection patterns
- Some dynamic code generation

### A.3 clap

**Repository**: https://github.com/clap-rs/clap
**Version**: v4.4.10 (as of validation)
**License**: MIT/Apache-2.0
**Language**: Rust 1.70+
**LOC**: ~50,000
**Architecture**: Trait-based with derive macros
**Key Features**:
- Compile-time argument parsing
- Derive macro API
- Runtime builder API
- Shell completion generation

**Indexing Notes**:
- Heavy macro usage (challenges for both grep and search)
- Trait-based abstractions (ideal for semantic search)
- Conditional compilation features
- Extensive documentation comments

---

## Appendix B: Task Adaptation Examples

### B.1 Full Adaptation: Transitive Dependencies

#### Original Task (CrewChief)
```typescript
{
  id: "rel-1001",
  category: "relationship-discovery",
  name: "Transitive Dependencies",
  description: "Find all modules that transitively depend on the worktree module",
  expectedGrepSuccess: 0.15,
  expectedSearchSuccess: 0.80,
  tier: "tier1-impossible"
}
```

#### Adapted: Commander.js
```typescript
{
  id: "rel-1001-commander",
  originalTaskId: "rel-1001",
  targetCodebase: "commander-js",
  category: "relationship-discovery",
  name: "Transitive Dependencies - Command Options",
  description: "Find all modules that transitively depend on the option parsing module",
  expectedGrepSuccess: 0.25,  // Easier: smaller codebase
  expectedSearchSuccess: 0.80,
  tier: "tier1-impossible",
  adaptationNotes: "Mapped worktree → option parsing. Smaller depth (max 3 levels vs 5).",
  adaptationConfidence: 0.90,
  conceptMapping: {
    source: "worktree management module",
    target: "option parsing module",
    rationale: "Both are core functionality with extensive internal dependencies"
  },
  challenges: [
    "Smaller codebase means less transitive depth",
    "Clearer module boundaries may make grep more effective"
  ]
}
```

#### Adapted: FastAPI
```typescript
{
  id: "rel-1001-fastapi",
  originalTaskId: "rel-1001",
  targetCodebase: "fastapi",
  category: "relationship-discovery",
  name: "Transitive Dependencies - Request Routing",
  description: "Find all modules that transitively depend on the routing module",
  expectedGrepSuccess: 0.20,
  expectedSearchSuccess: 0.85,  // Higher: better semantic structure
  tier: "tier1-impossible",
  adaptationNotes: "Mapped worktree → routing. Python imports are more explicit than TS.",
  adaptationConfidence: 0.85,
  conceptMapping: {
    source: "worktree management module",
    target: "routing module",
    rationale: "Core architectural component with cross-cutting dependencies"
  },
  challenges: [
    "Dynamic imports may hide some dependencies",
    "Decorator pattern creates implicit dependencies"
  ]
}
```

#### Adapted: clap
```typescript
{
  id: "rel-1001-clap",
  originalTaskId: "rel-1001",
  targetCodebase: "clap",
  category: "relationship-discovery",
  name: "Transitive Dependencies - Parser Core",
  description: "Find all modules that transitively depend on the parser core",
  expectedGrepSuccess: 0.10,  // Harder: large codebase, macro complexity
  expectedSearchSuccess: 0.85,
  tier: "tier1-impossible",
  adaptationNotes: "Mapped worktree → parser core. Rust macros create hidden dependencies.",
  adaptationConfidence: 0.75,  // Lower: macro expansion is complex
  conceptMapping: {
    source: "worktree management module",
    target: "parser core module",
    rationale: "Central component with extensive trait-based dependencies"
  },
  challenges: [
    "Macro expansion creates compile-time dependencies not visible in source",
    "Conditional compilation (#[cfg]) affects dependency graph",
    "Large codebase means deep transitive chains (5+ levels)"
  ]
}
```

### B.2 Adaptation Confidence Scoring

**0.9-1.0 (High Confidence)**:
- Clear conceptual mapping
- Similar architectural patterns
- Minimal adaptation required
- Expected difficulty preserved

**0.7-0.9 (Medium Confidence)**:
- Reasonable conceptual mapping
- Some architectural differences
- Moderate adaptation required
- Difficulty characteristics approximately preserved

**0.5-0.7 (Low Confidence)**:
- Loose conceptual mapping
- Significant architectural differences
- Extensive adaptation required
- Difficulty may not be preserved

**<0.5 (Very Low Confidence)**:
- Unclear mapping
- Task may not be applicable to target codebase
- Consider excluding from validation

---

## Appendix C: Infrastructure Usage

### C.1 Running Cross-Project Validation

```typescript
import {
  runCrossProjectValidation,
  SAMPLE_CODEBASES,
  type AdaptedTask
} from '@/search-optimization/validation/cross-project'

// 1. Define or load adapted tasks
const adaptedTasks: AdaptedTask[] = [
  // ... task definitions
]

// 2. Run validation (mock data for testing)
const result = await runCrossProjectValidation({
  codebases: SAMPLE_CODEBASES,
  tasks: adaptedTasks,
  iterations: 1,
  useMockData: true,  // Change to false for real execution
  parallel: false
})

// 3. Analyze results
console.log('Universal tasks:', result.summary.universalTasks)
console.log('Codebase-specific tasks:', result.summary.specificTasks)

// 4. Generate report
import { formatCrossProjectSummary } from '@/search-optimization/validation/cross-project'
const summary = formatCrossProjectSummary(result)
console.log(summary)
```

### C.2 Calculating Generalization Metrics

```typescript
import {
  calculateGeneralizationMetrics,
  calculateTransferabilityScore
} from '@/search-optimization/validation/cross-project'

// Calculate metrics for a specific task across codebases
const taskResults = codebaseResults.flatMap(cb =>
  cb.taskResults.filter(r => r.task.id === 'rel-1001')
)

const metrics = calculateGeneralizationMetrics('rel-1001', taskResults)

console.log('Transferability:', metrics.transferabilityScore)
console.log('Mean search advantage:', metrics.statistics.meanSearchAdvantage)
console.log('Advantage consistency:', metrics.consistentAdvantage)
```

### C.3 Custom Codebase Configuration

```typescript
import type { CodebaseConfig } from '@/search-optimization/validation/cross-project'

const myCodebase: CodebaseConfig = {
  id: 'my-project',
  name: 'My Project',
  language: 'typescript',
  domain: 'web-framework',
  sizeCategory: 'medium',
  repositoryUrl: 'https://github.com/org/my-project',
  description: 'Custom project for validation',
  path: '/path/to/my-project',
  worktree: 'main'
}

// Add to validation
const result = await runCrossProjectValidation({
  codebases: [...SAMPLE_CODEBASES, myCodebase],
  tasks: adaptedTasks,
  iterations: 1
})
```

---

## Appendix D: Statistical Analysis Methods

*To be completed after validation runs*

### D.1 Transferability Score Calculation

**Formula**:
```
transferability = (successCount / totalCount) * consistencyFactor

where:
  successCount = number of codebases where searchSuccess > 0.7
  totalCount = number of codebases tested
  consistencyFactor = max(0.5, 1.0 - stdDev(advantages) * 2)
```

**Interpretation**:
- 1.0: Perfect generalization (works on all codebases with consistent advantage)
- 0.5: Partial generalization (works on some codebases)
- 0.0: No generalization (fails on all codebases)

### D.2 Advantage Consistency

**Variance Threshold**: σ² < 0.05 (5%)

**Calculation**:
```
For each task:
  advantages = [searchSuccess - grepSuccess for each codebase]
  mean = sum(advantages) / count
  variance = sum((adv - mean)² for adv in advantages) / count
  isConsistent = variance < 0.05
```

### D.3 Confidence Intervals

*To be added after sufficient data collection (5+ runs per task per codebase)*

---

## Conclusion

This cross-project validation infrastructure provides a robust framework for evaluating task generalization across diverse codebases. The methodology, adaptation process, and analysis framework are ready for execution.

**Next Steps**:
1. Index selected codebases (Commander.js, FastAPI, clap)
2. Finalize task adaptations and validate with task-validator
3. Run pilot validation (1 iteration, ~$25)
4. Analyze pilot results and refine adaptations
5. Execute full validation (3-5 iterations, ~$75-150)
6. Complete results, analysis, and recommendations sections

**Infrastructure Status**: ✓ Complete and Ready for Validation

---

*End of Report*
