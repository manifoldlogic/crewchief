# Project: Grep-Impossible Task Design & Test Methodology

**Project Slug**: `TESTDES`
**Status**: Planning Complete
**Duration**: 10 weeks
**Estimated Cost**: $100-200 (API credits for validation)

## Problem Statement

The recent genetic optimization experiment revealed a critical insight: **we optimized the wrong thing**. Tool descriptions improved across 4 generations, but agents never used the search tool—not once across 40+ sessions. The problem wasn't failed optimization; it was misaligned measurement.

We optimized tool descriptions while the real question is: **"Does semantic search provide measurable value without coercing agents to use it?"**

Current task: "Find the code that creates git worktrees"
- Agent choice: Grep/Glob (rational—task is grep-solvable)
- Search usage: 0%
- Score: ~20% (because searchQuality=0 when search isn't used)

**Core Issue**: Tasks allow—and even encourage—simpler tools. To prove semantic search value, we need tasks where grep provably fails but search succeeds.

## Solution

Build a rigorous test design framework with three tiers:

**Tier 1: Grep-Impossible** (Prove Capability)
- Tasks grep cannot solve (<30% success rate)
- Examples: transitive dependencies, call chain analysis, negative space queries
- Proves: Semantic search can do things grep cannot

**Tier 2: Grep-Hard** (Prove Efficiency)
- Tasks grep struggles with (30-60% success, slow)
- Examples: conceptual similarity, ambiguity resolution, cross-cutting concerns
- Proves: Semantic search is 30-50% faster/better

**Tier 3: Real-World** (Prove Utility)
- Tasks from actual development workflows
- Natural tool selection (no coercion)
- Proves: Developers voluntarily adopt when appropriate

**Key Principles**:
- **No Coercion**: Tasks don't hint "use semantic search"
- **Objective Criteria**: Binary pass/fail, no "good explanation" fuzziness
- **Ecological Validity**: Based on real code review, debugging, refactoring
- **Scientific Rigor**: Grep baseline, statistical tests, cross-project validation

## Architecture

```
Task Design Framework
├── Taxonomy (6 categories, difficulty levels)
├── Task Generator (templates, variations, anti-keyword patterns)
├── Evaluation Pipeline
│   ├── Baseline Runner (grep-only execution)
│   ├── Comparison Framework (grep vs search)
│   └── Statistical Analysis (significance tests)
├── Validation System
│   ├── Construct Validity (tasks measure what they claim)
│   ├── Discriminant Validity (grep/search perform differently)
│   ├── Ecological Validity (realistic scenarios)
│   └── Cross-Project Generalization
└── Integration
    ├── Multi-Tier Genetic Optimizer
    ├── Benchmark Suites (Tier 1, 2, 3)
    └── Research Documentation
```

## Key Innovation

**Grep-Impossible Task Patterns**:

1. **Relationship Discovery**: "What would break if we change this API?"
   - Grep: Can find direct callers
   - Search: Finds transitive dependencies via code graph

2. **Conceptual Similarity**: "Find all retry implementations"
   - Grep: Finds "retry" keyword, misses exponential backoff, circuit breakers
   - Search: Understands conceptual similarity across different naming

3. **Architectural Flow**: "How does a request flow through the system?"
   - Grep: Manual call chain following, easy to miss steps
   - Search: Assembles complete flow with context

4. **Negative Space**: "Find endpoints without rate limiting"
   - Grep: Impossible (can't search for absence)
   - Search: Uses code graph to find unprotected endpoints

5. **Ambiguity Resolution**: "Where are database transactions managed?"
   - Grep: Finds all "transaction" mentions (ORM, decorators, manual)
   - Search: Disambiguates via context understanding

## Research Foundation

Draws on three fields:

**Information Retrieval (TREC)**:
- Query difficulty classification (easy vs hard)
- Relevance judgments (graded, user-centered)
- Adversarial test sets

**Machine Learning Evaluation**:
- Behavioral testing (CheckList methodology)
- Property-based validation
- Adversarial examples

**Software Testing**:
- Mutation testing (what if search is broken?)
- Cross-validation (generalization)
- Statistical significance

## Project Phases

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| 1. Foundation | 2 weeks | Taxonomy, baseline runner, comparison framework |
| 2. Grep-Impossible Tasks | 2 weeks | 8-10 Tier 1 tasks, suite runner |
| 3. Validation Infrastructure | 1 week | Task validator, ecological checks, reports |
| 4. Tier 2 & 3 Tasks | 2 weeks | 20 additional tasks, task generator |
| 5. Integration & Optimization | 1 week | Multi-tier optimizer, full validation |
| 6. Documentation & Research | 2 weeks | Framework docs, research report |

**Total**: 10 weeks, 30+ tasks across 3 tiers

## Success Criteria

**Phase 2 Success** (Tier 1 Complete):
- [ ] 8 grep-impossible tasks
- [ ] 80%+ defeat grep (<30% success)
- [ ] Search advantage >40%
- [ ] All tasks pass validation

**Project Success**:
- [ ] 30+ tasks across 3 tiers
- [ ] Statistical significance (p < 0.05) for grep vs search
- [ ] 60%+ cross-project generalization
- [ ] Genetic optimizer improves search usage
- [ ] Research-quality documentation

**Real-World Impact**:
- [ ] Proves semantic search provides measurable value
- [ ] Framework reusable for other tools/techniques
- [ ] Contributes novel insights to IR evaluation
- [ ] Better tool descriptions, better optimization

## Agents

- **Primary**: `general-purpose` (TypeScript implementation, task creation)
- **Support**: `technical-researcher` (survey design, research report)
- **Testing**: `unit-test-runner` (validation tests)
- **Verification**: `verify-ticket` (acceptance criteria)
- **Commit**: `commit-ticket` (conventional commits)

## Planning Documents

- **[Analysis](planning/analysis.md)**: Deep research on test design, problem space, prior art
- **[Architecture](planning/architecture.md)**: Framework design, task patterns, data model
- **[Quality Strategy](planning/quality-strategy.md)**: Validation methodology, testing approach
- **[Security Review](planning/security-review.md)**: Pragmatic security for research framework
- **[Plan](planning/plan.md)**: 6-phase implementation with detailed deliverables

## Why This Matters

The genetic optimization experiment cost ~$2.50 and taught us a $100K lesson: **optimize for the right outcome**.

Tool descriptions improved by every metric we tracked. But agents never used the tool because the tasks didn't require it. We optimized local maxima while missing global optimum.

This project ensures we measure what actually matters:
1. **Capability**: Can semantic search solve problems grep cannot?
2. **Efficiency**: Is semantic search faster/better for complex queries?
3. **Utility**: Do developers voluntarily adopt when appropriate?

If we build this framework correctly, we'll have:
- **Scientific validation** of semantic search value
- **Systematic methodology** for tool evaluation
- **Research contributions** to IR and developer tools
- **Practical impact** on tool adoption and optimization

## Next Steps

1. **Review planning docs**: Understand approach and rationale
2. **Create tickets**: `/create-project-tickets TESTDES`
3. **Review tickets**: `/review-tickets TESTDES`
4. **Execute**: `/work-on-project TESTDES` or ticket-by-ticket

## Learning Summary

**From Genetic Optimization**:
- Tool descriptions: Optimized ✓
- Search usage: 0% (agents chose Grep)
- Scores: ~20% (because searchQuality=0)
- Lesson: We measured improvement, not value

**The Insight**:
Agents made the CORRECT choice. For "find worktree code", Grep IS better. The task was grep-solvable, so they solved it with grep.

To prove search is valuable, we need tasks where:
- Grep objectively fails (too slow, too noisy, or impossible)
- Search measurably succeeds (faster, more accurate, possible at all)
- Task is realistic (developers actually encounter this)
- Victory is clear (objective criteria, reproducible)

**This project delivers that proof.**
