# Ticket: TESTDES-1001: Implement Task Taxonomy Infrastructure

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
Implement the foundational task taxonomy infrastructure that categorizes search tasks by characteristics predicting tool performance. Create 6 task categories (relationship-discovery, conceptual-similarity, architectural-understanding, negative-space, ambiguity-resolution, cross-cutting-concerns) and difficulty classification system (grep-impossible, grep-hard, grep-possible) with pattern templates.

## Background
The genetic optimization experiment revealed agents chose Grep over semantic search because tasks were grep-solvable. To prove semantic search provides measurable value, we need a systematic taxonomy for creating tasks that favor semantic search without coercion.

This taxonomy is the foundation for the entire TESTDES framework. It enables systematic task creation and helps identify which task types favor semantic search vs grep. The 6 categories are based on research into information retrieval evaluation and real developer workflows.

**Reference**: See architecture.md Section "Task Taxonomy" (lines 51-128) for detailed category definitions and rationale.

## Acceptance Criteria
- [x] 6 task categories defined with TypeScript types and clear descriptions
- [x] Difficulty classification enum with thresholds (impossible <30% grep success, hard 30-60%, possible >60%)
- [x] Pattern templates exported for each category showing structure and examples
- [x] Types are exported and usable by other modules
- [x] Unit tests validate category definitions

## Technical Requirements
- TypeScript implementation in `packages/cli/src/search-optimization/taxonomy/`
- Create `categories.ts` with TaskCategory interface and 6 category constants
- Create `difficulty.ts` with DifficultyLevel enum and classification logic
- Create `patterns.ts` with template types and example patterns
- Follow existing code style (ESM modules, strict typing)
- Export all types from `taxonomy/index.ts`
- Use Vitest for unit tests

## Implementation Notes
Each category should include:
- name: string identifier (e.g., "relationship-discovery")
- description: clear explanation of what this category tests
- grepDifficulty: 'impossible' | 'hard' | 'possible' | 'easy'
- searchAdvantage: 'critical' | 'significant' | 'moderate' | 'none'
- realWorldFrequency: 'common' | 'occasional' | 'rare'
- exampleScenarios: 2-3 concrete examples

**The 6 categories** (from architecture.md):
1. **Relationship Discovery** - Transitive dependencies, call chains (grep: impossible, search: critical)
2. **Conceptual Similarity** - Pattern matching across different naming (grep: hard, search: significant)
3. **Ambiguity Resolution** - Disambiguating multiple implementations (grep: hard, search: significant)
4. **Negative Space** - Finding absence of expected patterns (grep: impossible, search: critical)
5. **Cross-Cutting Concerns** - Scattered patterns across codebase (grep: hard, search: moderate)
6. **Architectural Understanding** - System-level flow and interactions (grep: impossible, search: critical)

**Difficulty thresholds**:
- grep-impossible: <30% success rate with grep
- grep-hard: 30-60% success rate with grep
- grep-possible: >60% success rate with grep

**Pattern templates** should follow the structure from architecture.md lines 369-487, showing:
- Template pattern string
- Grep approach and difficulty
- Search approach and advantage
- Success criteria structure

## Dependencies
None (foundation ticket)

## Risk Assessment
- **Risk**: Taxonomy might evolve as we create actual tasks
  - **Mitigation**: Keep categories flexible, accept that definitions may be refined based on empirical data in Phase 2

- **Risk**: Difficulty thresholds are estimates until validated
  - **Mitigation**: Document that thresholds will be validated empirically in Phase 2 when running actual grep baselines

## Files/Packages Affected
**Files to Create**:
- `packages/cli/src/search-optimization/taxonomy/categories.ts`
- `packages/cli/src/search-optimization/taxonomy/difficulty.ts`
- `packages/cli/src/search-optimization/taxonomy/patterns.ts`
- `packages/cli/src/search-optimization/taxonomy/index.ts`
- `packages/cli/src/search-optimization/taxonomy/__tests__/categories.test.ts`
