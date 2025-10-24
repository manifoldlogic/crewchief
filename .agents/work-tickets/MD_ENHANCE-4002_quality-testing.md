# Ticket: MD_ENHANCE-4002: Quality Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- parser-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Compare old regex parser output vs new tree-sitter parser output, test search quality improvements, verify heading hierarchies are correct, and run performance benchmarks to ensure no regression. This validates that the migration achieved its goals and maintains system performance.

## Background
The MD_ENHANCE project aims to improve documentation indexing quality. We need to validate that the new parser is actually better: more accurate, maintains hierarchy, improves search results, and doesn't degrade performance. This ticket ensures we deliver on the success metrics.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 86-96, 98-103

## Acceptance Criteria
- [ ] Parser accuracy measured at >99% (correctly identifies all markdown elements)
- [ ] Hierarchy tracking validated at 100% (all parent paths correct)
- [ ] Code block detection at 100% (all code blocks found with correct languages)
- [ ] Search relevance improved (A/B test queries before/after)
- [ ] No performance regression (indexing time within 10% of baseline)
- [ ] Query performance acceptable (<100ms for typical searches)
- [ ] Comprehensive test report generated

## Technical Requirements
- Create test suite comparing old vs new parser on same documents
- Measure parse accuracy: % of markdown elements correctly identified
- Validate hierarchy: manually verify parent paths for representative docs
- Benchmark search quality: run test queries, compare result relevance
- Performance testing: measure indexing time for 100+ markdown files
- Stress testing: parse large documents (>10k lines)
- Edge case testing: malformed markdown, empty files, Unicode content
- Generate detailed test report with metrics and comparisons

Success Metrics Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 98-103

## Implementation Notes

### Accuracy Testing
```rust
#[test]
fn test_parser_accuracy() {
    let test_docs = load_test_documents();

    for doc in test_docs {
        let expected = doc.expected_elements; // Manually verified
        let actual = new_parser.parse(&doc.content);

        let accuracy = calculate_accuracy(expected, actual);
        assert!(accuracy > 0.99, "Parser accuracy below 99%");
    }
}
```

### Hierarchy Validation
Test cases:
- Simple nesting: h1 > h2 > h3
- Sibling headings: h1 > h2 > h2 > h2
- Level jumps: h1 > h4 (skipping h2, h3)
- Complex documents: mixture of all patterns

Expected output:
```
✓ All parent paths correct (152/152)
✓ No orphaned headings
✓ Nesting depth accurate
```

### Search Quality Testing
A/B test queries:
1. "authentication example" → Should find code blocks in auth section
2. "database setup" → Should find setup section with hierarchy
3. "API reference" → Should return API docs with parent context

Metrics:
- Precision: % of results that are relevant
- Recall: % of relevant docs that are found
- MRR: Mean reciprocal rank of first relevant result

### Performance Benchmarks
Measure:
- Time to index 100 markdown files
- Time to index single large file (10k lines)
- Query response time (average, p95, p99)
- Memory usage during parsing
- Database query performance with new metadata

Baseline (regex parser):
- 100 files: ~2.5s
- Large file: ~150ms
- Query p95: ~45ms

Target (tree-sitter parser):
- 100 files: <3.0s (within 20%)
- Large file: <200ms (within 33%)
- Query p95: <50ms (similar or better)

### Test Report Template
```markdown
# MD_ENHANCE Quality Testing Report

## Parser Accuracy
- Elements detected: 1,247 / 1,256 (99.3%)
- Headings: 100%
- Code blocks: 100%
- Tables: 98.5%
- Lists: 99.1%

## Hierarchy Tracking
- Correct parent paths: 152 / 152 (100%)
- Nesting depth accurate: ✓
- Section boundaries: ✓

## Search Quality
- Precision improved: 78% → 89%
- Recall improved: 82% → 91%
- MRR improved: 0.73 → 0.85

## Performance
- Indexing 100 files: 2.5s → 2.8s (+12%)
- Large file parsing: 150ms → 175ms (+17%)
- Query p95: 45ms → 42ms (-7%)

## Conclusion
✅ All success metrics met
✅ No performance regression
✅ Search quality significantly improved
```

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 98-109

## Dependencies
- MD_ENHANCE-4001 (Migration Script) - MUST be completed to have new data
- All Phase 1-3 tickets - Parser implementation must be complete

## Risk Assessment
- **Risk**: Test coverage insufficient to catch edge cases
  - **Mitigation**: Use real project docs as test cases, crowd-source edge cases, review tree-sitter-markdown test suite

- **Risk**: Performance regression not detected in testing but appears in production
  - **Mitigation**: Test with production-scale data, monitor after deployment, implement rollback plan

- **Risk**: Search quality improvements subjective and hard to measure
  - **Mitigation**: Use quantitative metrics (precision/recall), A/B test with real users, collect feedback

## Files/Packages Affected
- `crates/maproom/tests/integration/quality_test.rs` - New comprehensive test suite
- `crates/maproom/tests/integration/performance_test.rs` - Performance benchmarks
- `crates/maproom/tests/fixtures/reference_docs/` - Manually verified test documents
- `crates/maproom/benches/parser_bench.rs` - Criterion benchmarks
- `docs/MD_ENHANCE_TEST_REPORT.md` - Generated test report
- `scripts/run-quality-tests.sh` - Test execution script
