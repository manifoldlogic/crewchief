# Ticket: AGENTOPT-0001: Create Test Query Set (100 Queries)

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
Create a comprehensive 100-query test set with gold standards for validating tool description variants. This foundational dataset enables empirical testing of AI agent query optimization.

## Background
This ticket implements Phase 0, Step 1 from the AGENTOPT project plan. The data-driven optimization approach requires a representative test set of queries that AI agents typically ask when searching code. This test set will be used across all variant testing experiments to ensure consistent, statistical measurement of query transformation quality.

The test set must cover diverse query patterns:
- Natural language questions (40 queries) - "How does X work?"
- Simple keyword queries (30 queries) - "error handling"
- Complex multi-word queries (20 queries) - "cart checkout validation"
- Edge cases (10 queries) - camelCase, file paths, special characters

Each query requires gold standard expected results to enable automated success measurement.

## Acceptance Criteria
- [ ] 100 total queries created across 4 categories (40 NL, 30 simple, 20 complex, 10 edge)
- [ ] Each query has defined expected terms, minimum result count, and gold standard files
- [ ] Test query set saved in JSON format at `.crewchief/projects/AGENTOPT_ai-agent-query-optimization/test-queries.json`
- [ ] Query set validated by running sample queries against current maproom index
- [ ] Documentation added explaining query selection rationale and expected usage

## Technical Requirements
- JSON structure matching schema in architecture.md lines 952-975
- Each query must include: id, category, query, expected_terms, min_results, gold_standard_files, notes
- Query diversity: Cover authentication, error handling, database, API, UI, utils, config, tests
- Real-world queries: Based on actual developer search patterns
- Edge case coverage: Include queries that currently fail to identify improvement opportunities

## Implementation Notes
1. Analyze current maproom search logs (if available) for common query patterns
2. Draft queries across categories ensuring diversity:
   - Natural language: Use "how", "what", "where", "when" patterns
   - Simple: 2-3 word technical terms
   - Complex: 3-5 word specific searches
   - Edge cases: Test boundary conditions
3. For each query, manually search current codebase to determine:
   - Expected search terms that SHOULD work
   - Minimum expected result count
   - Gold standard files that SHOULD appear in results
4. Validate 10% of queries (10 random samples) against live maproom index
5. Document query selection methodology in test-queries.json header

Example structure:
```json
{
  "metadata": {
    "version": "1.0",
    "created": "2025-01-20",
    "total_queries": 100,
    "purpose": "Variant testing for AI agent query optimization"
  },
  "test_queries": [
    {
      "id": "NL-001",
      "category": "natural_language",
      "query": "How does authentication work?",
      "expected_terms": ["authentication", "auth"],
      "min_results": 3,
      "gold_standard_files": ["auth.ts", "middleware/auth.ts"],
      "notes": "Common natural language pattern"
    }
  ]
}
```

## Dependencies
- Access to crewchief codebase for query validation
- Maproom MCP tool available for testing queries

## Risk Assessment
- **Risk**: Test queries not representative of real usage
  - **Mitigation**: Review search logs if available, consult with developers on common search patterns
- **Risk**: Gold standards become outdated as codebase evolves
  - **Mitigation**: Version the test set, include codebase commit hash in metadata
- **Risk**: Query set too small to detect statistical differences
  - **Mitigation**: 100 queries provides n≥100 for statistical validity (p<0.05)

## Files/Packages Affected
- `.crewchief/projects/AGENTOPT_ai-agent-query-optimization/test-queries.json` (create)
