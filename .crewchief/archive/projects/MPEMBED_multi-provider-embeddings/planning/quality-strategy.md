# MPEMBED: Multi-Provider Embedding Support - Quality Strategy

## Testing Philosophy

**Pragmatic MVP approach:**
- Tests exist to **prevent rework and backtracking**, not for ceremonial coverage
- Focus on **integration boundaries** where provider abstraction meets reality
- Prioritize **regression prevention** for dimension handling (the current blocker)
- Use **property-based testing** for invariants that must hold across all providers
- Avoid **unit testing internal provider details** (let API contracts be the tests)

**Anti-patterns to avoid:**
- Testing implementation details (e.g., "does OllamaProvider use reqwest?")
- Exhaustive mocking (e.g., mocking every HTTP call in provider implementations)
- Test-for-test's-sake (e.g., testing that dimension() returns 768 for Ollama)
- Elite test signaling (e.g., 95% coverage metrics without practical value)

## Risk-Based Test Prioritization

### Critical Risks (Must Test)

**Risk 1: Dimension mismatch at database insertion**
- **Why critical**: This is the current blocker; regression would be catastrophic
- **Test strategy**: Integration tests that verify embeddings actually persist

**Risk 2: Provider switching breaks existing embeddings**
- **Why critical**: Users with OpenAI embeddings must not lose data
- **Test strategy**: Migration tests with fixture data

**Risk 3: Search returns wrong/no results after provider change**
- **Why critical**: Core functionality; silent failures are dangerous
- **Test strategy**: End-to-end search tests with multiple providers

### Medium Risks (Should Test)

**Risk 4: Google Vertex AI authentication failures in production**
- **Why medium**: Complex auth, but fails fast with clear errors
- **Test strategy**: Integration test with service account credentials

**Risk 5: Ollama auto-detection false positives/negatives**
- **Why medium**: Affects UX, but users can override with config
- **Test strategy**: Unit tests for detection logic

### Low Risks (Minimal Testing)

**Risk 6: Provider-specific API changes**
- **Why low**: Caught immediately in integration tests
- **Test strategy**: Rely on provider SDK contracts

**Risk 7: Performance regressions in COALESCE queries**
- **Why low**: Measurable with benchmarks, not critical for MVP
- **Test strategy**: Performance benchmarks (not CI-blocking)

## Test Levels

### 1. Unit Tests (Selective)

**What to unit test:**
- Provider factory logic (correct provider from env vars)
- Column selection logic (dimension → column name mapping)
- Provider detection (Ollama availability check)

**What NOT to unit test:**
- Individual provider implementations (HTTP clients, JSON parsing)
- EmbeddingService caching logic (already tested in existing codebase)
- Database query construction (covered by integration tests)

**Example test:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_selection_768_dim() {
        let (code_col, text_col) = select_columns_for_dimension(768);
        assert_eq!(code_col, "code_embedding_ollama");
        assert_eq!(text_col, "text_embedding_ollama");
    }

    #[test]
    fn test_column_selection_1536_dim() {
        let (code_col, text_col) = select_columns_for_dimension(1536);
        assert_eq!(code_col, "code_embedding");
        assert_eq!(text_col, "text_embedding");
    }

    #[test]
    #[should_panic(expected = "Invalid dimension")]
    fn test_column_selection_invalid_dim() {
        select_columns_for_dimension(512);
    }
}
```

### 2. Integration Tests (Primary Focus)

**Critical integration tests:**

#### Test A: Ollama embeddings persist to correct columns

```rust
#[tokio::test]
async fn test_ollama_embeddings_persist_768_dim() {
    // Setup: Ollama provider + test database
    let provider = OllamaProvider::new(
        "http://localhost:11434/api/embed",
        "nomic-embed-text",
    ).unwrap();
    let pool = setup_test_db().await;

    // Generate embedding
    let embedding = provider.embed("fn hello() {}".to_string()).await.unwrap();
    assert_eq!(embedding.len(), 768);

    // Insert into database
    upsert_embeddings(&pool, 1, Some(embedding.clone()), None, 768).await.unwrap();

    // Verify it's in the right column
    let row: (Option<Vec<f32>>, Option<Vec<f32>>) = sqlx::query_as(
        "SELECT code_embedding_ollama, code_embedding FROM maproom.chunks WHERE id = 1"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(row.0.is_some()); // 768-dim column populated
    assert!(row.1.is_none());  // 1536-dim column empty
    assert_eq!(row.0.unwrap().len(), 768);
}
```

#### Test B: Google embeddings reuse Ollama columns

```rust
#[tokio::test]
async fn test_google_embeddings_share_768_columns() {
    let provider = GoogleProvider::new(
        "test-project",
        "us-central1",
        "text-embedding-gecko@003",
        "RETRIEVAL_DOCUMENT",
    ).await.unwrap();
    let pool = setup_test_db().await;

    let embedding = provider.embed("class Foo {}".to_string()).await.unwrap();
    assert_eq!(embedding.len(), 768);

    upsert_embeddings(&pool, 2, Some(embedding), None, 768).await.unwrap();

    let row: (Option<Vec<f32>>,) = sqlx::query_as(
        "SELECT code_embedding_ollama FROM maproom.chunks WHERE id = 2"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(row.0.is_some());
    assert_eq!(row.0.unwrap().len(), 768);
}
```

#### Test C: OpenAI embeddings preserve existing behavior

```rust
#[tokio::test]
async fn test_openai_embeddings_use_original_columns() {
    let config = EmbeddingConfig {
        provider: "openai".to_string(),
        api_key: env::var("OPENAI_API_KEY").unwrap(),
        model: "text-embedding-3-small".to_string(),
        // ... other config
    };
    let provider = Box::new(OpenAIClient::new(config).unwrap());
    let pool = setup_test_db().await;

    let embedding = provider.embed("interface User {}".to_string()).await.unwrap();
    assert_eq!(embedding.len(), 1536);

    upsert_embeddings(&pool, 3, Some(embedding), None, 1536).await.unwrap();

    let row: (Option<Vec<f32>>, Option<Vec<f32>>) = sqlx::query_as(
        "SELECT code_embedding_ollama, code_embedding FROM maproom.chunks WHERE id = 3"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(row.0.is_none());  // 768-dim column empty
    assert!(row.1.is_some()); // 1536-dim column populated
    assert_eq!(row.1.unwrap().len(), 1536);
}
```

#### Test D: Migration preserves existing OpenAI embeddings

```rust
#[tokio::test]
async fn test_migration_preserves_existing_embeddings() {
    let pool = setup_test_db_with_openai_embeddings().await;

    // Pre-migration: Verify existing embeddings
    let count_before: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Run migration 0015 (add 768-dim columns)
    run_migration(&pool, "0015_add_ollama_columns.sql").await.unwrap();

    // Post-migration: Verify embeddings still exist
    let count_after: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count_before.0, count_after.0);

    // Verify new columns exist and are empty
    let new_columns: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding_ollama IS NOT NULL"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(new_columns.0, 0);
}
```

### 3. End-to-End Tests

**Scenario 1: Scan → Embed → Search (Ollama)**

```rust
#[tokio::test]
async fn test_e2e_ollama_scan_and_search() {
    // Setup: Test repository with TypeScript file
    let temp_dir = setup_test_repo_with_file("src/hello.ts", "export function hello() {}");
    let pool = setup_test_db().await;

    // Scan with Ollama embeddings
    env::set_var("EMBEDDING_PROVIDER", "ollama");
    let scan_result = run_scan_command(&temp_dir, "--generate-embeddings=true").await.unwrap();

    assert_eq!(scan_result.files_processed, 1);
    assert_eq!(scan_result.chunks_created, 1);
    assert_eq!(scan_result.embeddings_generated, 1);

    // Search for the function
    let search_results = hybrid_search(&pool, "hello function", 768, 10).await.unwrap();

    assert!(!search_results.is_empty());
    assert_eq!(search_results[0].symbol_name, Some("hello".to_string()));
}
```

**Scenario 2: Mixed embeddings (OpenAI + Ollama)**

```rust
#[tokio::test]
async fn test_e2e_mixed_embeddings_search() {
    let pool = setup_test_db().await;

    // Insert chunk with OpenAI embedding (1536-dim)
    let openai_vec = vec![0.1; 1536];
    upsert_embeddings(&pool, 1, Some(openai_vec), None, 1536).await.unwrap();

    // Insert another chunk with Ollama embedding (768-dim)
    let ollama_vec = vec![0.2; 768];
    upsert_embeddings(&pool, 2, Some(ollama_vec), None, 768).await.unwrap();

    // Search with 768-dim query (Ollama)
    let query_vec = vec![0.2; 768];
    let results = hybrid_search(&pool, "test", query_vec, 768, 10).await.unwrap();

    // Should return Ollama chunk (chunk 2), not OpenAI chunk (dimension mismatch)
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].chunk_id, 2);
}
```

### 4. Property-Based Tests

**Invariant 1: Embedding dimension always matches provider**

```rust
#[quickcheck]
fn prop_provider_dimension_matches_embedding(provider_name: ProviderName) -> bool {
    let provider = create_test_provider(provider_name);
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let embedding = provider.embed("test".to_string()).await.unwrap();
        embedding.len() == provider.dimension()
    })
}
```

**Invariant 2: Column selection is deterministic and reversible**

```rust
#[quickcheck]
fn prop_column_selection_deterministic(dimension: u16) -> TestResult {
    if dimension != 768 && dimension != 1536 {
        return TestResult::discard();
    }

    let (code_col, text_col) = select_columns_for_dimension(dimension as usize);
    let inferred_dim = infer_dimension_from_column(&code_col);

    TestResult::from_bool(inferred_dim == dimension as usize)
}
```

### 5. Contract Tests

**Provider contract: All providers must satisfy interface**

```rust
#[async_trait]
pub trait EmbeddingProviderContractTest {
    async fn create_provider() -> Box<dyn EmbeddingProvider>;

    #[tokio::test]
    async fn test_embed_returns_correct_dimension() {
        let provider = Self::create_provider().await;
        let embedding = provider.embed("test".to_string()).await.unwrap();
        assert_eq!(embedding.len(), provider.dimension());
    }

    #[tokio::test]
    async fn test_embed_batch_maintains_order() {
        let provider = Self::create_provider().await;
        let texts = vec!["first".to_string(), "second".to_string(), "third".to_string()];
        let embeddings = provider.embed_batch(texts.clone()).await.unwrap();
        assert_eq!(embeddings.len(), 3);
        // Verify embeddings correspond to inputs (semantic similarity)
    }

    #[tokio::test]
    async fn test_provider_name_is_lowercase() {
        let provider = Self::create_provider().await;
        assert_eq!(provider.provider_name(), provider.provider_name().to_lowercase());
    }
}

// Apply contract to each provider
impl EmbeddingProviderContractTest for OllamaProvider { /* ... */ }
impl EmbeddingProviderContractTest for GoogleProvider { /* ... */ }
impl EmbeddingProviderContractTest for OpenAIClient { /* ... */ }
```

## Test Data Strategy

**Fixtures:**
- **Small repository**: 5 TypeScript files, ~20 chunks (for fast tests)
- **Existing OpenAI embeddings**: Fixture database with 100 chunks, all with 1536-dim embeddings
- **Mixed embeddings**: Fixture with 50 chunks (768-dim) + 50 chunks (1536-dim)

**Test doubles:**
- **Mock Ollama server**: HTTP server returning fake 768-dim embeddings
- **Mock Google auth**: Service account credentials that work in test environment
- **Mock OpenAI client**: Already exists in codebase

**Real provider tests (optional, CI-gated):**
- Run against real Ollama (if available on CI machine)
- Run against real Google Vertex AI (if CI has credentials)
- Run against real OpenAI (if CI has API key)

## Continuous Integration Strategy

**Pre-commit checks:**
- Rust lint (cargo clippy)
- Rust format (cargo fmt --check)
- Unit tests (fast, no DB or network)

**CI pipeline (on PR):**
1. Run all unit tests
2. Spin up PostgreSQL with pgvector (Docker)
3. Run integration tests (with test database)
4. Run E2E tests (with mock providers)
5. Optional: Run real provider tests if credentials available

**CI pipeline (on main merge):**
- Same as PR pipeline
- Additionally: Run performance benchmarks, report to dashboard

## Manual Testing Checklist

Before marking implementation complete:

**Provider functionality:**
- [ ] Ollama embeddings generate and persist (768-dim)
- [ ] Google embeddings generate and persist (768-dim)
- [ ] OpenAI embeddings generate and persist (1536-dim)
- [ ] Search works with each provider independently
- [ ] Search works with mixed embeddings (768 + 1536)

**Zero-config experience:**
- [ ] `npx -y @crewchief/maproom-mcp` works with Ollama running on localhost
- [ ] Error message is clear if no provider available
- [ ] `EMBEDDING_PROVIDER=google` switches to Google successfully

**Migration safety:**
- [ ] Existing OpenAI embeddings survive migration
- [ ] Adding 768-dim columns doesn't break existing search
- [ ] Switching from OpenAI to Ollama doesn't lose file indexes

**Performance:**
- [ ] Ollama: 1,000 chunks in <5 minutes
- [ ] Search latency with COALESCE: <100ms for 25K chunks
- [ ] Index build time: <2 minutes for 25K chunks

## Quality Gates

**Code review requirements:**
- Two approvers (one familiar with database, one with embeddings)
- No `unwrap()` in production code (use proper error handling)
- All public APIs documented with rustdoc comments

**Merge requirements:**
- All CI checks pass
- Integration tests pass for all three providers
- Migration test passes (existing embeddings preserved)
- Manual testing checklist complete

**Post-merge monitoring:**
- No regressions in search latency (measure p95)
- No increase in failed embedding generation (monitor error logs)
- User reports of dimension mismatch (should be zero)

## Known Limitations (Acceptable for MVP)

**Not testing:**
- Embedding quality differences between providers (subjective, needs eval harness)
- Cost tracking accuracy for Google Vertex AI (depends on billing API)
- Concurrent embedding generation race conditions (existing codebase already handles)
- Ollama model download failures (out of scope, user responsibility)

**Edge cases deliberately ignored:**
- Switching between Ollama and Google (both use same columns, latest wins)
- Having 3 different dimension embeddings simultaneously (not supported)
- Provider APIs changing response schemas (would break immediately, caught in integration tests)

## Test Maintenance Strategy

**When to update tests:**
- New provider added: Implement contract tests
- Database schema change: Update migration tests
- Search algorithm change: Update E2E search tests
- Bug discovered: Add regression test

**When to delete tests:**
- Provider removed (e.g., deprecate OpenAI support)
- Implementation detail tests that don't prevent regressions
- Duplicate tests that verify the same invariant

## Success Metrics

**Test effectiveness:**
- Zero dimension mismatch errors in production after launch
- Search functionality regression detected within 1 CI run
- Provider switching works first try for 95% of users

**Test efficiency:**
- Unit tests: <5 seconds total
- Integration tests: <30 seconds total
- E2E tests: <2 minutes total
- Full CI pipeline: <5 minutes
