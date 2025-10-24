# Ticket: LANG_PARSE-1006: Python Parser Testing Suite

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- integration-tester
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement comprehensive testing suite for Python parser including unit tests for symbol extraction, edge case handling for malformed code, performance benchmarks, and real-world project testing with Django and Flask codebases.

## Background
As part of Phase 1, Week 2 of the LANG_PARSE project, we need to ensure the Python parser is robust, performant, and production-ready. The parser implementation (LANG_PARSE-1001 through LANG_PARSE-1004) needs thorough testing to validate correctness, handle edge cases gracefully, and meet performance targets. Testing against real-world frameworks like Django and Flask will ensure the parser works with actual production code patterns.

## Acceptance Criteria
- [ ] Unit tests achieve >90% code coverage for Python parser components
- [ ] Edge cases (malformed code, incomplete syntax, unusual decorators) are handled gracefully without panics
- [ ] Performance benchmarks show parsing speed of at least 150 files/min (within 2x of TypeScript baseline)
- [ ] Real-world Django and Flask sample projects parse successfully with all symbols extracted
- [ ] All test suites pass in CI/CD pipeline
- [ ] Performance regression tests integrated into benchmark suite

## Technical Requirements
- Create comprehensive test fixtures covering all Python symbol types (functions, classes, methods, decorators, imports)
- Implement edge case tests for:
  - Incomplete/partial Python code
  - Syntax errors and malformed code
  - Unusual decorator patterns and nested decorators
  - Complex class hierarchies and metaclasses
  - Mixed indentation and encoding issues
- Develop performance benchmarks using criterion.rs
  - Target: 150 files/min minimum parsing speed
  - Baseline comparison with TypeScript parser (within 2x)
  - Memory usage profiling
- Test with real-world projects:
  - Django sample project (core framework files)
  - Flask sample application (routing, blueprints, extensions)
- Integration tests verifying end-to-end symbol extraction and indexing

## Implementation Notes

### Test Structure
- **Unit Tests**: Located in `crates/maproom/tests/parser/python_edge_cases_test.rs`
  - Test each symbol extraction function independently
  - Cover happy path and error conditions
  - Validate AST traversal logic

- **Benchmarks**: Located in `crates/maproom/benches/python_parser_bench.rs`
  - Use criterion.rs for statistical benchmarking
  - Test single file parsing, batch parsing, and incremental updates
  - Compare against TypeScript parser baseline

- **Integration Tests**: Located in `crates/maproom/tests/integration/python_real_world_test.rs`
  - Test complete workflows from file reading to symbol extraction
  - Validate against Django and Flask codebases
  - Verify database integration and indexing

### Test Fixtures Organization
Create test fixtures directory structure:
```
crates/maproom/tests/fixtures/python/
  ├── edge_cases/
  │   ├── incomplete_syntax.py
  │   ├── malformed_decorators.py
  │   ├── unusual_classes.py
  │   └── mixed_indentation.py
  ├── django_samples/
  │   ├── models.py
  │   ├── views.py
  │   └── urls.py
  └── flask_samples/
      ├── app.py
      ├── blueprints.py
      └── extensions.py
```

### Performance Targets
- Minimum 150 files/min parsing speed
- Maximum 2x slower than TypeScript parser
- Memory usage should not exceed 500MB for 1000 files
- No memory leaks in long-running parsing sessions

### Edge Case Handling Strategy
- Parser should never panic on malformed input
- Return meaningful error messages for syntax errors
- Partial symbol extraction from incomplete code (best-effort)
- Graceful degradation when tree-sitter fails to parse

## Dependencies
- LANG_PARSE-1005 (Python parser integration - assumed complete based on ticket description)
- LANG_PARSE-1001 (Python grammar setup)
- LANG_PARSE-1002 (Python symbol extraction)
- LANG_PARSE-1003 (Python import extraction)
- LANG_PARSE-1004 (Python docstring parsing)

## Risk Assessment
- **Risk**: Real-world Django/Flask projects may use patterns not covered by current parser
  - **Mitigation**: Start with smaller sample projects, incrementally add support for complex patterns, document known limitations

- **Risk**: Performance targets may be difficult to achieve with tree-sitter overhead
  - **Mitigation**: Profile early, identify bottlenecks, consider parallel parsing for batch operations, optimize AST traversal

- **Risk**: Edge case handling may introduce performance regressions
  - **Mitigation**: Benchmark edge case paths separately, ensure fast-path remains optimized, use feature flags for expensive validation

- **Risk**: Test maintenance burden as parser evolves
  - **Mitigation**: Use property-based testing where applicable, create test helpers for common scenarios, document test rationale

## Files/Packages Affected
### New Files to Create
- `crates/maproom/tests/parser/python_edge_cases_test.rs` - Edge case unit tests
- `crates/maproom/benches/python_parser_bench.rs` - Performance benchmarks
- `crates/maproom/tests/integration/python_real_world_test.rs` - Integration tests
- `crates/maproom/tests/fixtures/python/edge_cases/*.py` - Edge case test fixtures
- `crates/maproom/tests/fixtures/python/django_samples/*.py` - Django test fixtures
- `crates/maproom/tests/fixtures/python/flask_samples/*.py` - Flask test fixtures

### Existing Files to Modify
- `crates/maproom/Cargo.toml` - Add criterion.rs benchmark dependencies
- `crates/maproom/benches/` - Add benchmark configuration
- `.github/workflows/ci.yml` - Add benchmark regression checks (if applicable)
