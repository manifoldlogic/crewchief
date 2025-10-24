# Ticket: LANG_PARSE-4002: Search Quality Validation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- search-quality-engineer
- integration-tester
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Validate search quality and accuracy across all supported languages (TypeScript, JavaScript, Python, Rust, Go). This ticket focuses on ensuring that multi-language indexing maintains or improves search quality through comprehensive testing of cross-language search queries, symbol resolution accuracy, and edge relationship correctness.

## Background
With the addition of Python, Rust, and Go language support to Maproom's indexing capabilities, we need to ensure that:
1. Cross-language search queries work correctly (e.g., searching for "authentication" returns relevant results from all languages)
2. Symbol resolution is accurate (finding the correct definition when multiple symbols exist)
3. Edge relationships (imports, function calls, type references) are correctly captured across all languages
4. Overall search quality is maintained or improved compared to TypeScript/JavaScript-only indexing

This validation is critical for the Phase 4 rollout and ensures users can confidently search across polyglot codebases.

## Acceptance Criteria
- [ ] Cross-language search queries return relevant results from all indexed languages
- [ ] Symbol resolution correctly identifies definitions across all supported languages
- [ ] Edge relationships (imports, calls, type references) are correctly captured and queryable
- [ ] Search quality metrics show no degradation compared to baseline (TypeScript/JavaScript-only)
- [ ] Documentation describes validation methodology and results
- [ ] Test suite includes automated search quality checks for all languages

## Technical Requirements
- Create comprehensive multi-language search test queries covering common programming concepts
- Implement symbol resolution accuracy tests that verify correct definition lookup
- Build edge relationship validation tests for imports, function calls, and type references
- Establish search quality metrics (precision, recall, relevance ranking)
- Compare multi-language search quality against TypeScript/JavaScript baseline
- Document test fixtures and expected results for each language
- Automate search quality regression testing

## Implementation Notes

### Test Structure
Create three main test suites:

1. **Multi-Language Search Tests** (`multi_language_search_test.rs`):
   - Test queries like "authentication", "database query", "error handling"
   - Verify results include relevant symbols from all languages
   - Check relevance ranking across languages
   - Test language-specific features (e.g., decorators, traits, interfaces)

2. **Symbol Resolution Tests** (`symbol_resolution_test.rs`):
   - Test disambiguation when multiple symbols have the same name
   - Verify correct definition is returned for symbols across files
   - Test qualified name resolution (module.Class.method)
   - Validate cross-language symbol references

3. **Edge Relationship Tests** (integrated into existing test files):
   - Verify import/dependency edges are correctly captured
   - Test function call graph accuracy
   - Validate type reference relationships
   - Check cross-file and cross-language edges

### Validation Methodology
- Use real-world codebases as test fixtures (include samples from popular projects)
- Establish baseline metrics from TypeScript/JavaScript-only indexing
- Measure precision, recall, and relevance for each language
- Document any language-specific quirks or limitations
- Create regression test suite that runs on CI

### Search Quality Metrics
- **Precision**: Percentage of returned results that are relevant
- **Recall**: Percentage of relevant results that are returned
- **Relevance Ranking**: Are most relevant results ranked higher?
- **Cross-Language Coverage**: Do queries return results from all applicable languages?

## Dependencies
- LANG_PARSE-4001 (large-scale testing and validation) - provides infrastructure for validation testing
- LANG_PARSE-1008 (Python production validation) - Python indexing must be stable
- LANG_PARSE-2003 (Rust documentation extraction) - Rust indexing must be complete
- LANG_PARSE-3003 (Go conventions) - Go indexing must be complete

## Risk Assessment
- **Risk**: Search quality may degrade with multi-language support due to increased noise
  - **Mitigation**: Establish baseline metrics and use language-specific ranking factors

- **Risk**: Edge relationships may be incomplete or incorrect across languages
  - **Mitigation**: Thorough testing with known codebases and manual verification

- **Risk**: Symbol resolution may be ambiguous when multiple languages define similar symbols
  - **Mitigation**: Use qualified names and language context for disambiguation

- **Risk**: Test fixtures may not represent real-world usage patterns
  - **Mitigation**: Use samples from popular open-source projects in each language

## Files/Packages Affected
### New Test Files
- `crates/maproom/tests/search/multi_language_search_test.rs` - Cross-language search query tests
- `crates/maproom/tests/search/symbol_resolution_test.rs` - Symbol resolution accuracy tests
- `crates/maproom/tests/fixtures/python_sample/` - Python test fixtures
- `crates/maproom/tests/fixtures/rust_sample/` - Rust test fixtures
- `crates/maproom/tests/fixtures/go_sample/` - Go test fixtures

### Documentation
- `crates/maproom/docs/search_quality_validation.md` - Validation methodology and results

### Existing Test Infrastructure
- `crates/maproom/tests/search/` - May need updates to existing search tests
- `crates/maproom/tests/integration/` - Integration test framework
