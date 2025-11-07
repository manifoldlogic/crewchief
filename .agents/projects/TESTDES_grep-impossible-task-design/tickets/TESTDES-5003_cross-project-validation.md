# Ticket: TESTDES-5003: Implement Cross-Project Validation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- technical-researcher
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Validate that grep-impossible tasks generalize beyond CrewChief to other codebases. Select 3 diverse codebases (different languages, domains, sizes), adapt a subset of ~10 tasks for cross-project testing, run validation, and measure generalization metrics. Create research document analyzing task transferability, variance in grep vs search advantage, and codebase-specific patterns.

## Background
The TESTDES framework creates grep-impossible tasks based on CrewChief's codebase. To prove these tasks have real-world utility, we must demonstrate they generalize to other projects. Generalization validation is a critical quality dimension (quality-strategy.md Section 3: Ecological Validity) that proves tasks aren't artificially tailored to one codebase.

This cross-project validation provides empirical evidence that:
1. Task categories reflect universal code search patterns
2. Grep vs search performance gaps hold across different codebases
3. The taxonomy is reusable for evaluating semantic search on any project

**Reference**: See plan.md "Phase 5: Integration & Optimization" (lines 400-457) and quality-strategy.md "Ecological Validity" for validation criteria.

## Acceptance Criteria
- [ ] 3 diverse codebases selected (documented in research report)
  - [ ] Different primary languages (TypeScript, Python, Rust minimum)
  - [ ] Different domains (CLI tool, web framework, systems programming)
  - [ ] Different sizes (small <10k LOC, medium 10-50k LOC, large >50k LOC)
- [ ] Subset of ~10 tasks adapted for cross-project testing
  - [ ] Tasks span all 6 categories from taxonomy
  - [ ] Mix of Tier 1 (grep-impossible) and Tier 2 (grep-hard) tasks
- [ ] Validation executed on all 3 codebases
  - [ ] Grep baseline run for each task on each codebase
  - [ ] Search-enabled run for each task on each codebase
- [ ] Generalization metrics calculated
  - [ ] Task success rate across codebases (mean, variance, range)
  - [ ] Grep vs search advantage per codebase (consistency check)
  - [ ] Task transferability score (which tasks work universally vs codebase-specific)
- [ ] Research document created at `docs/research/cross-project-validation.md`
  - [ ] Methodology: codebase selection criteria and task adaptation process
  - [ ] Results: performance metrics for each codebase
  - [ ] Analysis: what patterns generalize vs what's codebase-specific
  - [ ] Recommendations: how to adapt tasks for new codebases
- [ ] Limitations identified and documented
  - [ ] Language-specific patterns (e.g., Python decorators vs TypeScript decorators)
  - [ ] Domain-specific patterns (e.g., CLI vs web framework architecture)
  - [ ] Size-related patterns (e.g., small projects lack transitive dependencies)

## Technical Requirements
- TypeScript implementation in `packages/cli/src/search-optimization/validation/cross-project.ts`
- Cross-project runner that:
  - Accepts codebase configuration (path, language, description)
  - Adapts tasks for target codebase (modify queries, adjust success criteria)
  - Executes adapted tasks with both grep-only and search-enabled configurations
  - Collects metrics per codebase, per task, per tool configuration
- Integration with existing evaluation infrastructure:
  - Reuse `baseline-runner.ts` for grep-only execution
  - Reuse `comparison.ts` for side-by-side evaluation
  - Extend `metrics.ts` to calculate generalization statistics
- Codebase selection criteria:
  - **TypeScript codebase**: Medium-sized CLI tool or framework (different from CrewChief)
  - **Python codebase**: Web framework or data processing library (FastAPI, Django, pandas-like)
  - **Rust codebase**: Systems tool or networking library (tokio-based, async runtime)
- Task adaptation strategy:
  - Keep core task structure (category, difficulty, success criteria)
  - Modify queries to match target codebase concepts
  - Adjust file paths and expected results
  - Document adaptations in task metadata
- Generalization metrics:
  - **Task Success Rate**: Mean success across 3 codebases (variance as stability indicator)
  - **Grep vs Search Gap**: Difference in success rate (consistency check: should be similar across codebases)
  - **Transferability Score**: Binary (task works on all 3) or continuous (% of codebases where task succeeds)
- Research document structure:
  ```markdown
  # Cross-Project Validation Report

  ## Executive Summary
  ## Methodology
  ### Codebase Selection
  ### Task Adaptation Process
  ## Results
  ### Per-Codebase Performance
  ### Generalization Metrics
  ### Statistical Analysis
  ## Analysis
  ### What Generalizes
  ### What's Codebase-Specific
  ### Language-Specific Patterns
  ### Domain-Specific Patterns
  ## Recommendations
  ### Adapting Tasks for New Codebases
  ### Universal vs Specific Tasks
  ## Limitations
  ## Future Work
  ```

## Implementation Notes

### Codebase Selection Strategy
Select codebases that are:
1. **Publicly available**: Open-source projects we can index
2. **Well-structured**: Clear architecture, good documentation
3. **Active**: Recent commits, maintained projects
4. **Representative**: Cover common development scenarios

**Suggested candidates**:
- **TypeScript**: `commander.js` (CLI framework), `fastify` (web framework), `type-graphql` (GraphQL library)
- **Python**: `fastapi` (web framework), `click` (CLI framework), `httpx` (HTTP client)
- **Rust**: `clap` (CLI parser), `tokio` (async runtime), `axum` (web framework)

### Task Adaptation Process
For each task:
1. **Review core concept**: What is this task fundamentally testing?
2. **Map to target codebase**: Find equivalent concepts (e.g., "worktree management" → "request routing" for web framework)
3. **Adapt query**: Modify semantic search query to match target domain
4. **Adjust success criteria**: Update expected file paths, function names
5. **Document mapping**: Record original task → adapted task in metadata

Example:
```typescript
// Original (CrewChief)
{
  query: "How does worktree creation work?",
  expectedFiles: ["worktree/create.ts"],
  category: "architectural-understanding"
}

// Adapted (FastAPI)
{
  query: "How does request routing work?",
  expectedFiles: ["routing.py", "applications.py"],
  category: "architectural-understanding",
  adaptationNotes: "Mapped worktree creation to request routing as core architectural concept"
}
```

### Metrics Calculation
```typescript
interface CrossProjectMetrics {
  taskId: string
  category: string

  // Per-codebase results
  codebaseResults: {
    codebaseName: string
    grepSuccess: number
    searchSuccess: number
    grepSearchGap: number
  }[]

  // Generalization metrics
  meanGrepSuccess: number
  meanSearchSuccess: number
  successVariance: number  // Low variance = good generalization
  consistentGap: boolean   // Search advantage similar across codebases
  transferabilityScore: number  // 0.0-1.0 (% of codebases where task succeeds)
}
```

### Research Document Insights
The technical-researcher agent should analyze:
1. **Universal patterns**: Which task categories work everywhere?
   - Hypothesis: Relationship discovery, architectural understanding generalize well
   - Hypothesis: Negative space tasks may be codebase-specific (depend on conventions)
2. **Language effects**: Do Python codebases favor different task types than TypeScript?
3. **Size effects**: Do small codebases lack enough complexity for grep-impossible tasks?
4. **Domain effects**: Do web frameworks vs CLI tools have different search patterns?

### Cost Management
Cross-project validation is expensive (10 tasks × 3 codebases × 2 tool configs × 5 runs = 300 LLM calls).

**Mitigation**:
- Start with 1 run per configuration (10 × 3 × 2 = 60 calls) to validate approach
- If results are promising, expand to 3-5 runs for statistical significance
- Document API costs in research report for future planning
- Consider using smaller model (Claude Haiku) for validation runs

## Dependencies
- **TESTDES-5001**: Multi-tier optimizer must be complete (provides task suites to validate)
- **TESTDES-3001**: Task validator must be complete (validates adapted tasks)
- **TESTDES-2004**: Tier 1 benchmark suite must exist (source of tasks to adapt)

## Risk Assessment
- **Risk**: Selected codebases aren't diverse enough
  - **Mitigation**: Use explicit selection criteria (language, domain, size), document rationale in research report

- **Risk**: Task adaptation changes task difficulty (no longer grep-impossible)
  - **Mitigation**: Validate adapted tasks with task-validator.ts before running cross-project evaluation

- **Risk**: Results show poor generalization (tasks only work on CrewChief)
  - **Mitigation**: This is a valid research finding. Document which tasks generalize vs which don't. Adjust taxonomy based on results.

- **Risk**: API costs exceed budget
  - **Mitigation**: Start with single-run validation (60 calls ~$20-30), expand only if initial results are promising. Use cost tracking in runner.

- **Risk**: Codebase indexing fails (maproom doesn't support target language)
  - **Mitigation**: Choose codebases with languages maproom supports (TypeScript, Python, Rust confirmed). Test indexing before full validation run.

## Files/Packages Affected
**Files to Create**:
- `packages/cli/src/search-optimization/validation/cross-project.ts` - Cross-project validation runner
- `docs/research/cross-project-validation.md` - Research document (by technical-researcher)

**Files to Modify**:
- `packages/cli/src/search-optimization/validation/index.ts` - Export cross-project runner
- `packages/cli/src/search-optimization/evaluation/metrics.ts` - Add generalization metrics

**Files to Reference**:
- `packages/cli/src/search-optimization/evaluation/baseline-runner.ts` - Grep-only execution
- `packages/cli/src/search-optimization/evaluation/comparison.ts` - Side-by-side comparison
- `packages/cli/src/search-optimization/validation/task-validator.ts` - Validate adapted tasks
- `packages/cli/src/search-optimization/benchmarks/tier1-impossible.ts` - Source tasks for adaptation
