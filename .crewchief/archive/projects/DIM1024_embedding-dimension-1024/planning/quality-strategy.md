# Quality Strategy: embedding dimension 1024

## Testing Philosophy

**Confidence over coverage**: Test critical paths and edge cases, not ceremonial 100% coverage. Focus on scenarios that could break production: migration failures, dimension mismatches, data corruption, backward compatibility breaks.

**Test pyramid**:
- **Unit tests** (majority): Fast, focused, test individual functions
- **Integration tests** (moderate): Test database interactions and multi-component flows
- **E2E tests** (few): Test complete workflows (embedding generation → storage → search)

**Pragmatic approach**: Skip tests for trivial getters/setters. Test business logic, error handling, and edge cases.

## Test Types

### Unit Tests

**Scope:**
- Dimension mapping functions (get_vec_table_name)
- Constant validation (SUPPORTED_DIMENSIONS contains 1024)
- Configuration parsing (dimension from env vars)
- Validation logic (allow 1024 for Ollama)
- Conditional sanitization (model-based branching)
- Error messages (include 1024 in unsupported dimension errors)

**Tools:**
- Rust built-in test framework (`cargo test`)
- `#[cfg(test)]` modules inline with source
- Mock database connections (in-memory SQLite)

**Coverage Target:**
- Critical business logic: 100% (dimension mapping, validation)
- Supporting code: 80%+ (utilities, helpers)
- Trivial code: Skip (simple getters, constructors)

**Example Tests:**
```rust
#[test]
fn test_1024_dimension_mapping() {
    assert_eq!(get_vec_table_name(1024).unwrap(), "vec_code_1024");
}

#[test]
fn test_supported_dimensions_includes_1024() {
    assert!(SUPPORTED_DIMENSIONS.contains(&1024));
}

#[test]
fn test_config_allows_ollama_1024() {
    let config = EmbeddingConfig {
        provider: Provider::Ollama,
        model: "mxbai-embed-large".to_string(),
        dimension: 1024,
        // ...
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_conditional_sanitization_skips_mxbai() {
    let provider = OllamaProvider::new(
        "http://localhost:11434".to_string(),
        "mxbai-embed-large".to_string(),
        1024,
    ).unwrap();
    // Verify sanitization not applied (test with | character)
}
```

### Integration Tests

**Scope:**
- Migration #10 applies successfully (creates vec_code_1024 table)
- Migration #10 is idempotent (safe to run twice)
- 1024-dim embeddings store in code_embeddings table
- 1024-dim embeddings sync to vec_code_1024 table
- Vector search uses correct table for 1024-dim queries
- Mixed dimensions coexist (768, 1024, 1536 in same database)

**Approach:**
- Use in-memory SQLite database (fast, isolated)
- Register sqlite-vec extension in setup
- Run all migrations to simulate real database
- Test cross-component interactions

**Example Tests:**
```rust
#[test]
fn test_migration_10_creates_vec_code_1024() {
    let mut conn = setup_test_db();
    let mut runner = MigrationRunner::new(&mut conn);
    runner.migrate().unwrap();

    let exists: bool = conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='vec_code_1024'",
        [],
        |_| Ok(true),
    ).unwrap_or(false);
    assert!(exists);
}

#[test]
fn test_mixed_dimensions_storage() {
    let conn = setup_test_db();

    // Store embeddings of all three dimensions
    let embed_768: Vec<f32> = (0..768).map(|i| i as f32).collect();
    let embed_1024: Vec<f32> = (0..1024).map(|i| i as f32).collect();
    let embed_1536: Vec<f32> = (0..1536).map(|i| i as f32).collect();

    upsert_embedding(&conn, "blob_768", &embed_768, "nomic-embed-text").unwrap();
    upsert_embedding(&conn, "blob_1024", &embed_1024, "mxbai-embed-large").unwrap();
    upsert_embedding(&conn, "blob_1536", &embed_1536, "text-embedding-3-small").unwrap();

    // Verify all stored correctly
    assert_eq!(get_embedding(&conn, "blob_768").unwrap().unwrap().len(), 768);
    assert_eq!(get_embedding(&conn, "blob_1024").unwrap().unwrap().len(), 1024);
    assert_eq!(get_embedding(&conn, "blob_1536").unwrap().unwrap().len(), 1536);
}
```

### End-to-End Tests

**Scope:** Critical paths only (not all possible combinations).

**Critical Paths:**

1. **Full embedding workflow with mxbai-embed-large**
   - Configure: MAPROOM_EMBEDDING_MODEL=mxbai-embed-large, MAPROOM_EMBEDDING_DIMENSION=1024
   - Generate embedding for test text
   - Verify 1024-dim vector returned
   - Store in database
   - Sync to vec_code_1024
   - Search with query
   - Verify results returned

2. **Mixed dimension search**
   - Database has 768, 1024, 1536-dim embeddings
   - Query with 1024-dim embedding
   - Results only include 1024-dim chunks (not 768 or 1536)

3. **Problematic character handling**
   - Text with |, [], (), Unicode symbols
   - Embed with nomic-embed-text (sanitized)
   - Embed with mxbai-embed-large (raw)
   - Verify mxbai results preserve characters

**Note**: E2E tests require Ollama running locally, marked with `#[ignore]` to skip in CI.

## Critical Paths (MUST Test)

1. **Migration #10 idempotency**: Running twice doesn't error or duplicate data
2. **Dimension mapping correctness**: 1024 → vec_code_1024 (not vec_code or vec_code_768)
3. **Backward compatibility**: Existing 768 and 1536-dim embeddings still work after changes
4. **Configuration validation**: Ollama + 1024 passes validation (not rejected)
5. **Search correctness**: Vector search with 1024-dim query returns 1024-dim results
6. **Sanitization conditional**: nomic-embed-text uses sanitization, mxbai-embed-large doesn't
7. **Error messages**: Unsupported dimension error lists [768, 1024, 1536]

## Test Data Strategy

### Dimension Test Data

Use predictable, reproducible embeddings for testing:

```rust
// 768-dim: [0.0, 1.0, 2.0, ..., 767.0]
let embed_768: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();

// 1024-dim: [0.0, 1.0, 2.0, ..., 1023.0]
let embed_1024: Vec<f32> = (0..1024).map(|i| i as f32 / 1024.0).collect();

// 1536-dim: [0.0, 1.0, 2.0, ..., 1535.0]
let embed_1536: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
```

**Rationale**: Simple patterns are easy to verify (length check, value spot-check).

### Problematic Character Test Data

Test strings that crash nomic-embed-text:

```rust
const PROBLEMATIC_CHARS: &str = "| table | with | pipes\n[x] checkbox [link](url)\n→ ← ↔ arrows\n├── tree\n";
```

**Expected behavior**:
- nomic-embed-text: Characters replaced before embedding
- mxbai-embed-large: Characters preserved in raw text

### Migration Test Data

Test with actual database state:
- Fresh database (no migrations)
- Partially migrated database (migrations 1-9, not 10)
- Fully migrated database (migrations 1-10)

**Verify**: Migration #10 succeeds in all cases.

## Quality Gates

Before ticket verification passes, ALL must be true:

### Code Quality
- [ ] `cargo clippy` passes (no warnings)
- [ ] `cargo fmt --check` passes (code formatted)
- [ ] No compiler warnings (`cargo build --release`)

### Test Suite
- [ ] All unit tests pass (`cargo test --lib`)
- [ ] All integration tests pass (`cargo test --test '*'`)
- [ ] E2E tests pass OR marked with `#[ignore]` (if Ollama unavailable)
- [ ] No flaky tests (run 3x to confirm stability)

### Functionality
- [ ] Migration #10 applies successfully (check schema_migrations table)
- [ ] 1024-dim embeddings can be stored and retrieved
- [ ] Vector search with 1024-dim query returns results
- [ ] Existing 768/1536 embeddings still work

### Documentation
- [ ] Code comments explain non-obvious logic (conditional sanitization)
- [ ] Test function names are descriptive (`test_1024_dim_embedding_storage`)
- [ ] Error messages are clear ("Unsupported embedding dimension: 512. Supported dimensions: [768, 1024, 1536]")

## Test Automation

### Continuous Integration

**Pre-commit** (local):
```bash
cargo fmt --check && cargo clippy && cargo test
```

**CI pipeline** (GitHub Actions):
1. Lint: `cargo clippy -- -D warnings`
2. Format: `cargo fmt --check`
3. Build: `cargo build --release`
4. Test: `cargo test` (skip `#[ignore]` tests)

### Local Testing Workflow

**Fast feedback loop** (< 10 seconds):
```bash
# Run only dimension-related tests
cargo test dimension
```

**Full test suite** (< 2 minutes):
```bash
# Run all tests (unit + integration)
cargo test -p crewchief-maproom
```

**E2E with Ollama** (requires setup):
```bash
# Pull models first
ollama pull nomic-embed-text
ollama pull mxbai-embed-large

# Run ignored tests
cargo test -- --ignored
```

## Known Test Limitations

### 1. E2E tests require Ollama

**Limitation**: Cannot run full E2E tests in CI without Ollama installed.

**Mitigation**: Mark E2E tests with `#[ignore]`, run manually before release.

**Acceptance**: Unit and integration tests provide high confidence; E2E is validation, not primary testing.

### 2. Vector search quality is hard to test

**Limitation**: Difficult to write deterministic tests for "relevance" of search results.

**Mitigation**: Test structural correctness (right dimension, returns results, no crashes), not subjective quality.

**Acceptance**: Quality regression testing done manually with real-world queries.

### 3. Performance tests are environment-dependent

**Limitation**: Search latency varies by hardware, other load, etc.

**Mitigation**: Benchmark locally, document results in planning docs, skip automated performance gates.

**Acceptance**: Performance is monitored post-deployment, not blocking for MVP.

## Regression Testing

**After each ticket**, run regression suite to verify no breaks:

```bash
# Verify existing dimensions still work
cargo test test_768_dim
cargo test test_1536_dim

# Verify migrations still idempotent
cargo test test_migration_idempotent

# Verify mixed dimensions
cargo test test_mixed_dimensions
```

**Target**: Zero regressions (all existing tests pass after each change).

## Test Metrics (Not Goals, But Tracked)

Track these for visibility, not as pass/fail gates:

- **Test count**: ~30-40 tests total (unit + integration)
- **Test runtime**: < 2 minutes for full suite
- **Coverage**: 85%+ on dimension-related code (aspirational, not enforced)
- **Flakiness**: 0% (no flaky tests tolerated)

**Philosophy**: Better to have 10 meaningful tests than 100 trivial tests for coverage.

## Handoff to Implementation

Tests will be written during implementation, not before. **Test-driven development NOT required** - pragmatic approach:

1. Implement feature
2. Write unit tests for edge cases
3. Write integration test for happy path
4. Verify quality gates pass
5. Move to verification

This approach is faster and avoids speculative tests for code that might change.
