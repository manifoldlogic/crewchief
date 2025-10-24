# Ticket: CONTEXT_ASM-4002: Comprehensive Testing Suite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- mcp-context-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Create a comprehensive testing suite for the Context Assembly System including unit tests for all modules, integration tests with real codebase data, performance benchmarks, and quality validation tests to ensure >90% test coverage and verify system meets performance targets.

## Background
The Context Assembly System is a critical component that assembles contextual information for AI assistants. It must operate reliably with high performance (p95 <120ms) while maintaining quality and relevance. A comprehensive testing suite is essential to:
- Validate correctness of assembly logic across all modules
- Ensure performance meets production requirements
- Verify context quality and relevance
- Enable confident refactoring and maintenance
- Catch regressions early in development

This work is part of Phase 4 (Week 6, Task 2) of the CONTEXT_ASM project plan.

## Acceptance Criteria
- [ ] Test coverage exceeds 90% for all context assembly modules
- [ ] Unit tests implemented for all core modules (assembler, graph, budget, formatter, etc.)
- [ ] Integration tests passing with real codebase data
- [ ] Performance benchmarks documented showing p95 latency <120ms for assembly operations
- [ ] Quality validation tests verify context relevance and completeness
- [ ] All tests passing in CI/CD pipeline
- [ ] Test documentation created explaining test structure and how to run tests

## Technical Requirements
- **Unit Tests**: Test each module in isolation
  - `assembler.rs` - Assembly logic and orchestration
  - `graph.rs` - Dependency graph construction and traversal
  - `budget.rs` - Token budget management and allocation
  - `formatter.rs` - Content formatting and template rendering
  - `relationship_queries.rs` - Relationship query execution
  - `content_provider.rs` - Content retrieval and caching

- **Integration Tests**: End-to-end testing with real data
  - Test with actual codebase repositories
  - Verify complete assembly pipeline
  - Test with various context types (function, class, file, etc.)
  - Test error handling and edge cases

- **Performance Benchmarks**: Criterion-based benchmarks
  - Assembly operations for different context sizes
  - Graph construction and traversal
  - Budget calculation and allocation
  - Content formatting
  - Target: p95 latency <120ms for typical assembly operations

- **Quality Validation Tests**: Verify output quality
  - Context relevance scoring
  - Completeness verification
  - Relationship accuracy
  - Budget utilization efficiency

## Implementation Notes

### Testing Framework
- Use Rust's built-in `#[cfg(test)]` for unit tests
- Use `tests/` directory for integration tests
- Use Criterion.rs for performance benchmarks
- Create test fixtures with representative data

### Test Organization
```
crates/maproom/
├── src/
│   └── context/
│       ├── assembler.rs (with inline unit tests)
│       ├── graph.rs (with inline unit tests)
│       └── ...
├── tests/
│   └── context/
│       ├── integration/
│       │   ├── assembly_pipeline_test.rs
│       │   ├── real_data_test.rs
│       │   └── edge_cases_test.rs
│       ├── quality_test.rs
│       └── fixtures/
│           ├── sample_repo/
│           └── test_data.json
└── benches/
    └── context_benchmarks.rs
```

### Test Data Strategy
- Create minimal representative test fixtures
- Use snapshot testing for output validation where appropriate
- Mock database queries for unit tests
- Use test database with real schema for integration tests

### Performance Benchmark Scenarios
1. Small context: Single function with 2-3 dependencies
2. Medium context: Class with 5-7 related items
3. Large context: Module with 15-20 related items
4. Maximum budget: Assembly with budget constraints

### Quality Metrics to Test
- Context includes all direct dependencies
- Relationship types are correctly identified
- Budget allocation follows priority rules
- Formatted output is valid and parseable
- No duplicate content in assembly

## Dependencies
- **CONTEXT_ASM-4001**: Complete implementation of Context Assembly System (must be implemented to test)
- Existing Maproom test infrastructure
- Test database setup and fixtures

## Risk Assessment
- **Risk**: Real data integration tests may be flaky or environment-dependent
  - **Mitigation**: Use controlled test fixtures and clearly documented test database setup; provide scripts for test data initialization

- **Risk**: Performance benchmarks may vary significantly across hardware
  - **Mitigation**: Document benchmark environment specs; focus on relative performance and regression detection rather than absolute numbers

- **Risk**: Achieving >90% test coverage may reveal gaps in implementation
  - **Mitigation**: Work closely with mcp-context-engineer to address any discovered issues; may require refactoring for testability

- **Risk**: Quality validation metrics may be subjective or hard to quantify
  - **Mitigation**: Define clear, measurable quality criteria upfront; use heuristics like "all direct dependencies present" rather than subjective "goodness" scores

## Files/Packages Affected

### Files to Create
- `crates/maproom/tests/context/integration/assembly_pipeline_test.rs` - End-to-end assembly tests
- `crates/maproom/tests/context/integration/real_data_test.rs` - Tests with real codebase data
- `crates/maproom/tests/context/integration/edge_cases_test.rs` - Error handling and edge cases
- `crates/maproom/tests/context/quality_test.rs` - Quality validation tests
- `crates/maproom/tests/context/fixtures/` - Test data and fixtures directory
- `crates/maproom/benches/context_benchmarks.rs` - Performance benchmarks

### Files to Modify
- `crates/maproom/src/context/assembler.rs` - Add inline unit tests
- `crates/maproom/src/context/graph.rs` - Add inline unit tests
- `crates/maproom/src/context/budget.rs` - Add inline unit tests
- `crates/maproom/src/context/formatter.rs` - Add inline unit tests
- `crates/maproom/src/context/relationship_queries.rs` - Add inline unit tests
- `crates/maproom/src/context/content_provider.rs` - Add inline unit tests
- `crates/maproom/Cargo.toml` - Add test dependencies (criterion, etc.)
- `.github/workflows/` - CI configuration for running tests and benchmarks
