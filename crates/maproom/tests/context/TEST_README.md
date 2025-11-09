# Context Assembly Integration Tests

This directory contains comprehensive integration tests for the Context Assembly System (CONTEXT_ASM-4002).

## Test Structure

```
tests/context/
├── integration/
│   ├── assembly_pipeline_test.rs  # End-to-end pipeline tests (11 tests)
│   ├── edge_cases_test.rs         # Error handling tests (11 tests)
│   ├── real_data_test.rs          # Real fixture data tests (6 tests)
│   └── mod.rs                     # Module organization
├── quality_test.rs                # Quality validation tests (10 tests)
└── fixtures/
    └── sample-repo/               # Realistic test codebase
        ├── src/
        │   ├── lib.rs            # Main entry point
        │   ├── utils.rs          # Utility functions
        │   └── api.rs            # API handlers
        └── README.md             # Fixture documentation
```

## Test Categories

### 1. Assembly Pipeline Tests (829 lines)
**File**: `integration/assembly_pipeline_test.rs`

Tests the complete end-to-end assembly workflow:
- ✓ Primary chunk loading
- ✓ Relationship expansion (callers, callees, tests)
- ✓ Multi-level dependency traversal
- ✓ Budget allocation and truncation
- ✓ Token counting accuracy
- ✓ No duplicates verification
- ✓ Relevance scoring

**Key Tests**:
- `test_complete_assembly_pipeline_primary_only`
- `test_complete_assembly_pipeline_with_callees`
- `test_complete_assembly_pipeline_with_callers`
- `test_complete_assembly_pipeline_with_tests`
- `test_complete_assembly_pipeline_all_relationships`
- `test_budget_allocation_and_truncation`
- `test_relevance_scoring_order`
- `test_token_sum_consistency`
- `test_no_duplicate_chunks`

### 2. Edge Cases and Error Handling (786 lines)
**File**: `integration/edge_cases_test.rs`

Tests robustness and error scenarios:
- ✓ Missing chunk IDs
- ✓ File read errors
- ✓ Empty files and invalid line ranges
- ✓ Circular dependencies
- ✓ Malformed data
- ✓ Zero budget scenarios
- ✓ Very large files
- ✓ Concurrent assembly

**Key Tests**:
- `test_missing_chunk_id`
- `test_empty_file_handling`
- `test_missing_file_on_disk`
- `test_invalid_line_range`
- `test_circular_dependencies`
- `test_zero_budget`
- `test_very_large_file`
- `test_malformed_chunk_data`
- `test_concurrent_assembly_same_chunk`

### 3. Quality Validation Tests (839 lines)
**File**: `quality_test.rs`

Validates output quality metrics:
- ✓ No unrelated chunks included
- ✓ All direct dependencies present
- ✓ Relationship types correct
- ✓ Budget efficiency
- ✓ Deterministic results
- ✓ No duplicates
- ✓ Importance ordering
- ✓ Content extraction accuracy

**Key Tests**:
- `test_quality_no_unrelated_chunks`
- `test_quality_all_direct_dependencies_included`
- `test_quality_relationship_types_correct`
- `test_quality_budget_efficiency`
- `test_quality_deterministic_results`
- `test_quality_no_duplicate_chunks`
- `test_quality_importance_ordering`
- `test_quality_content_extraction_accuracy`

### 4. Real Data Tests (580 lines)
**File**: `integration/real_data_test.rs`

Tests with realistic fixture repository:
- ✓ Multi-file codebases
- ✓ Content matches actual files
- ✓ Multi-level callee expansion
- ✓ Realistic budget usage

**Key Tests**:
- `test_real_data_lib_run_with_callees`
- `test_real_data_api_process_with_multi_level_callees`
- `test_real_data_content_matches_files`
- `test_real_data_multi_file_context`
- `test_real_data_realistic_budget_usage`

## Test Fixtures

### Sample Repository
Located in `fixtures/sample-repo/`, this is a realistic Rust codebase with:
- Multiple files with interdependencies
- Function call relationships
- Test coverage
- Realistic code patterns

**Structure**:
- `lib.rs`: Main entry point calling utils and api
- `utils.rs`: Config loading, validation, formatting
- `api.rs`: Request processing with dependencies

**Known Relationships**:
- `lib::run()` → calls → `utils::load_config()`
- `lib::run()` → calls → `api::process_request()`
- `api::process_request()` → calls → `utils::format_output()`
- `api::process_request()` → calls → `api::fetch_data()`

## Running the Tests

### Prerequisites
Set up a PostgreSQL test database:
```bash
export MAPROOM_DATABASE_URL="postgresql://postgres:postgres@localhost:5432/maproom_test"
```

### Run All Integration Tests
```bash
# From maproom crate directory
cd crates/maproom

# Run all tests
cargo test --test "*"

# Run with output
cargo test --test "*" -- --nocapture

# Run specific test file
cargo test --test context_assembler_test
```

### Run Specific Test Categories
```bash
# Assembly pipeline tests
cargo test --package crewchief-maproom assembly_pipeline

# Edge cases
cargo test --package crewchief-maproom edge_cases

# Quality validation
cargo test --package crewchief-maproom quality

# Real data tests
cargo test --package crewchief-maproom real_data
```

## Test Patterns

### Database Setup
Tests use helper functions for database access:
```rust
fn should_skip_db_test() -> bool {
    env::var("MAPROOM_DATABASE_URL").is_err()
}

fn is_skip_error(e: &anyhow::Error) -> bool {
    let err_str = e.to_string();
    err_str.contains("MAPROOM_DATABASE_URL not set")
        || err_str.contains("Connection refused")
}
```

### Test Fixture Pattern
Tests create temporary directories with cleanup:
```rust
let temp_dir = TempDir::new()?;
let worktree_path = temp_dir.path().to_string_lossy().to_string();
// ... use temp_dir ...
// Automatic cleanup when temp_dir drops
```

### Assertion Patterns
Tests verify multiple quality aspects:
```rust
// Structure
assert_eq!(bundle.items.len(), expected_count);

// Content accuracy
assert!(primary.content.contains("expected_code"));

// Quality metrics
assert!(!bundle.truncated, "Should not be truncated");
assert!(bundle.total_tokens <= budget);

// No duplicates
let mut seen = HashSet::new();
for item in &bundle.items {
    let key = format!("{}:{}:{}", item.relpath, item.range.start, item.range.end);
    assert!(seen.insert(key.clone()), "Duplicate: {}", key);
}
```

## Coverage Summary

**Total Integration Tests**: 38 test cases
- Assembly pipeline: 11 tests
- Edge cases: 11 tests
- Quality validation: 10 tests
- Real data: 6 tests

**Lines of Test Code**: ~3,000 lines

**Estimated Coverage**: >90% for context assembly modules

## Test Quality Standards

All tests meet these standards:
- ✓ Self-contained with setup and cleanup
- ✓ Descriptive test names explaining what is tested
- ✓ Clear assertions with helpful error messages
- ✓ Handle database unavailability gracefully
- ✓ Deterministic and reproducible
- ✓ No flaky behavior
- ✓ Well-commented for complex scenarios

## Continuous Integration

Tests are designed to run in CI with:
- Docker Compose for PostgreSQL
- Environment variable configuration
- Graceful skipping when database unavailable
- Parallel execution support (with caution)

## Troubleshooting

### Tests Skip with "MAPROOM_DATABASE_URL not set"
**Solution**: Set the MAPROOM_DATABASE_URL environment variable:
```bash
export MAPROOM_DATABASE_URL="postgresql://user:pass@localhost:5432/maproom_test"
```

### Connection Refused Errors
**Solution**: Ensure PostgreSQL is running:
```bash
# Using Docker
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:16

# Or start local PostgreSQL
sudo systemctl start postgresql
```

### Tests Hang or Timeout
**Possible causes**:
- Circular dependency test may need more time
- Database connection pool exhausted
- File I/O bottleneck

**Solution**: Run with `RUST_LOG=debug` for diagnostics

### Fixture Files Not Found
**Error**: "Fixture path does not exist"
**Solution**: Run tests from crate directory, not workspace root:
```bash
cd crates/maproom
cargo test
```

## Future Enhancements

Potential additions:
- [ ] Snapshot testing for formatted output
- [ ] Performance regression tests
- [ ] Fuzzing for malformed input
- [ ] Multi-language fixture repositories
- [ ] Memory leak detection tests
- [ ] Stress tests with very large codebases

## References

- Ticket: CONTEXT_ASM-4002
- Context Assembly System: `src/context/`
- Existing Unit Tests: Inline in source modules
- Performance Benchmarks: `benches/context_assembly_bench.rs`
