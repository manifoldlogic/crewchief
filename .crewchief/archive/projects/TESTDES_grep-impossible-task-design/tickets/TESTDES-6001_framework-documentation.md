# Ticket: TESTDES-6001: Create Framework Documentation

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
Create comprehensive documentation that enables contributors to understand, use, and extend the grep-impossible task design framework. This includes three key guides: task design principles, validation methodology, and benchmark usage instructions.

## Background
The grep-impossible task design framework represents a novel approach to evaluating semantic code search tools through rigorous, scientifically-validated benchmarks. After completing Phases 1-5, we have:
- 30+ validated tasks across 3 tiers
- Systematic validation infrastructure
- Multi-tier genetic optimization integration
- Cross-project validation results

However, this framework is only valuable if others can use it. We need documentation that:
1. Teaches contributors how to create high-quality grep-impossible tasks
2. Explains the validation process and quality dimensions
3. Provides clear instructions for running benchmarks and interpreting results
4. Enables the framework to be extended to new task categories and codebases

This ticket implements Phase 6.1 of the project plan: "Framework Documentation" (plan.md lines 328-345).

Reference planning documents:
- Architecture: Task design patterns (architecture.md lines 366-487)
- Quality Strategy: Five quality dimensions and validation approach (quality-strategy.md lines 9-154)
- Plan: Documentation deliverables (plan.md lines 328-345)

## Acceptance Criteria
- [ ] `docs/search-optimization/task-design-guide.md` created with complete task design methodology
- [ ] `docs/search-optimization/validation-guide.md` created with validation procedures
- [ ] `docs/search-optimization/benchmark-usage.md` created with usage instructions
- [ ] Each guide includes practical examples and code snippets
- [ ] All 6 task categories explained with examples
- [ ] 5 quality dimensions documented with validation procedures
- [ ] Anti-keyword pattern tutorial included in task design guide
- [ ] Common pitfalls section included with mitigation strategies
- [ ] Troubleshooting sections included in all guides
- [ ] Cost considerations and best practices documented
- [ ] Integration with genetic optimizer explained in benchmark usage guide

## Technical Requirements

### 1. Task Design Guide (`task-design-guide.md`)
Must document:
- **Overview**: Purpose of grep-impossible tasks, framework philosophy
- **Six Task Categories** (from architecture.md lines 68-128):
  - Relationship Discovery (transitive dependencies, call chains)
  - Conceptual Similarity (pattern matching across terminology)
  - Ambiguity Resolution (disambiguating multiple implementation patterns)
  - Negative Space (finding absence/violations)
  - Cross-Cutting Concerns (scattered functionality)
  - Architectural Understanding (system-level flows)
- **Anti-Keyword Pattern**: Tutorial on avoiding obvious searchable terms
  - Example: "Find retry logic" → "Find code that re-attempts failed operations"
  - Why this matters: Prevents tasks from becoming grep-solvable
- **Task Design Patterns** (from architecture.md lines 366-487):
  - Transitive Relationship Query pattern
  - Conceptual Pattern Match pattern
  - Architectural Flow Trace pattern
  - Negative Constraint pattern
  - Multi-Pattern Aggregation pattern
- **Success Criteria Guidelines**: How to create objective, measurable criteria
- **Validation Checklist**: Pre-submission checks for new tasks
- **Common Pitfalls**:
  - Task too easy (grep succeeds >60%)
  - Task too hard (both grep and search fail)
  - Subjective criteria ("good explanation")
  - Coercion (hinting at tool choice)

### 2. Validation Guide (`validation-guide.md`)
Must document:
- **Overview**: Why validation matters, quality philosophy
- **Five Quality Dimensions** (from quality-strategy.md lines 9-154):
  1. **Construct Validity**: Tasks measure what they claim
     - Grep baseline validation (<30% success for "impossible")
     - Difficulty rating accuracy
  2. **Discriminant Validity**: Search vs grep perform differently
     - Statistical significance testing (p < 0.05)
     - Minimum improvement thresholds
  3. **Ecological Validity**: Tasks reflect real-world scenarios
     - Developer survey framework
     - Frequency classification
  4. **Test-Retest Reliability**: Consistent results across runs
     - Variance thresholds (<10%)
     - Determinism checks
  5. **Predictive Validity**: Benchmark scores predict real-world utility
     - Long-term validation approach
- **Running Baseline Comparisons**: Step-by-step instructions
- **Interpreting Statistical Results**: What p-values and effect sizes mean
- **Fixing Failed Tasks**: Troubleshooting guide by failure type
  - Task too easy: Add complexity, remove keywords
  - Task too hard: Simplify or add context
  - Insufficient advantage: Redesign to leverage search strengths
  - Unreliable results: Make criteria objective
  - Ecological invalid: Ground in real scenarios

### 3. Benchmark Usage Guide (`benchmark-usage.md`)
Must document:
- **Overview**: Three-tier framework (Tier 1: impossible, Tier 2: hard, Tier 3: real-world)
- **Running Individual Tasks**:
  - Grep-only execution
  - Search-available execution
  - Comparing results
- **Running Full Validation**: Complete benchmark suite execution
- **Reading Reports**: How to interpret validation reports
  - Task-by-task results
  - Suite-level statistics
  - Failure pattern analysis
- **Integration with Genetic Optimizer** (from plan.md lines 269-287):
  - Multi-tier scoring approach (40% T1 + 40% T2 + 20% T3)
  - Tool selection tracking
  - Using benchmarks to optimize tool descriptions
- **Cost Considerations**:
  - API usage estimates
  - When to run full validation vs subset
  - Manual execution guidance for expensive operations
- **Cross-Project Validation**: Testing tasks on new codebases
- **Best Practices**:
  - Iterative task refinement
  - Failure analysis workflow
  - Contributing new tasks

## Implementation Notes

### Code Examples to Include
- Complete task definition with all required fields
- Baseline runner usage
- Comparison framework usage
- Validation pipeline execution
- Report generation

### Command Examples
```typescript
// Run single task with grep-only
await runTask(task, { tools: ['grep', 'glob', 'read'] })

// Run with search available
await runTask(task, { tools: ['grep', 'glob', 'read', 'mcp__maproom__search'] })

// Validate task quality
const result = await validator.validateTask(task)

// Run full suite
const results = await runSuite(TIER1_SUITE)
```

### Cross-References
- Link to architecture document for deep dives
- Link to quality strategy for detailed validation procedures
- Link to plan document for project context
- Reference existing task implementations as examples

### Audience Considerations
- **Primary**: Contributors creating new tasks
- **Secondary**: Researchers understanding methodology
- **Tertiary**: Users interpreting benchmark results

Write for clarity and practical utility. Include "Why" explanations, not just "How" instructions.

## Dependencies
- TESTDES-1001: Task taxonomy (need to reference categories)
- TESTDES-1002: Baseline runner (need to document usage)
- TESTDES-1003: Comparison framework (need to document usage)
- TESTDES-2001, 2002, 2003: Example tasks to reference
- TESTDES-2004: Tier 1 suite (benchmark structure)
- TESTDES-3001: Task validator (validation procedures)
- TESTDES-3002: Ecological validation (survey framework)
- TESTDES-3003: Report generator (reading reports)
- TESTDES-4001, 4002: Additional tiers (complete framework)
- TESTDES-5001: Multi-tier optimizer (integration guide)

All previous phases must be complete to document the full framework.

## Risk Assessment
- **Risk**: Documentation becomes stale as framework evolves
  - **Mitigation**: Date-stamp documentation, create update process, version alongside code
- **Risk**: Too technical for new contributors
  - **Mitigation**: Start with conceptual overview, progressive disclosure, include beginner examples
- **Risk**: Examples become outdated
  - **Mitigation**: Reference actual code files, automated example validation
- **Risk**: Missing key information that blocks contributors
  - **Mitigation**: Review with fresh eyes, test with new contributor, gather feedback

## Files/Packages Affected
- **New files**:
  - `docs/search-optimization/task-design-guide.md` (~500-800 lines)
  - `docs/search-optimization/validation-guide.md` (~400-600 lines)
  - `docs/search-optimization/benchmark-usage.md` (~300-500 lines)
- **Updated files**:
  - `docs/search-optimization/README.md` (if exists, add index)
  - Main project `README.md` (add link to documentation)
