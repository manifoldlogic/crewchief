# Quality Strategy: Provider Configuration Fixes

## Testing Philosophy

**Goal**: Prevent regression and verify fixes work correctly without exhaustive ceremony.

**Approach**: Focused tests that caught the original bugs + integration tests for the happy path.

**Not Testing**: Edge cases that don't matter for MVP (rate limits, network errors, etc.)

## What We're Testing

### 1. Rust Configuration Loading (Unit Tests)

**Purpose**: Verify endpoint resolution logic is correct for each provider.

**Tests Needed**:

```rust
#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_openai_uses_default_endpoint() {
        // No EMBEDDING_API_ENDPOINT set
        env::remove_var("EMBEDDING_API_ENDPOINT");
        env::set_var("EMBEDDING_PROVIDER", "openai");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.openai.com/v1/embeddings"
        );
    }

    #[test]
    fn test_openai_ignores_ollama_endpoint() {
        // THIS IS THE BUG TEST - verify fix
        env::set_var("EMBEDDING_API_ENDPOINT", "http://localhost:11434/api/embed");
        env::set_var("EMBEDDING_PROVIDER", "openai");

        let config = EmbeddingConfig::from_env().unwrap();
        // Should ignore Ollama endpoint for OpenAI provider
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.openai.com/v1/embeddings"
        );
    }

    #[test]
    fn test_openai_accepts_custom_openai_endpoint() {
        // Allow explicit OpenAI endpoint override
        env::set_var("EMBEDDING_API_ENDPOINT", "https://api.openai.com/v2/embeddings");
        env::set_var("EMBEDDING_PROVIDER", "openai");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(
            config.api_endpoint_url(),
            "https://api.openai.com/v2/embeddings"
        );
    }

    #[test]
    fn test_ollama_uses_custom_endpoint() {
        env::set_var("EMBEDDING_API_ENDPOINT", "http://custom:8080/api/embed");
        env::set_var("EMBEDDING_PROVIDER", "ollama");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(
            config.api_endpoint_url(),
            "http://custom:8080/api/embed"
        );
    }

    #[test]
    fn test_ollama_uses_default_if_no_override() {
        env::remove_var("EMBEDDING_API_ENDPOINT");
        env::set_var("EMBEDDING_PROVIDER", "ollama");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(
            config.api_endpoint_url(),
            "http://localhost:11434/api/embed"
        );
    }

    #[test]
    fn test_google_ignores_embedding_api_endpoint() {
        env::set_var("EMBEDDING_API_ENDPOINT", "http://localhost:11434/api/embed");
        env::set_var("EMBEDDING_PROVIDER", "google");
        env::set_var("GOOGLE_VERTEX_REGION", "us-central1");

        let config = EmbeddingConfig::from_env().unwrap();
        // Should use region-based URL, not EMBEDDING_API_ENDPOINT
        assert!(config.api_endpoint_url().contains("us-central1"));
        assert!(!config.api_endpoint_url().contains("11434"));
    }
}
```

**Why These Tests Matter**:
- `test_openai_ignores_ollama_endpoint` - Catches the exact bug we had
- `test_openai_accepts_custom_openai_endpoint` - Proves override still works
- `test_ollama_uses_custom_endpoint` - Verifies Ollama override not broken
- Other tests - Verify each provider's default behavior

**What We're NOT Testing**:
- API key validation (covered elsewhere)
- Network requests (mocked in integration tests)
- Invalid provider names (existing validation)
- Parallel processing config (not changed)

### 2. Database Migration (Manual Test)

**Purpose**: Verify `updated_at` column addition works on fresh and existing databases.

**Test Plan**:

```bash
# Test 1: Fresh database (should work)
docker compose down -v
docker compose up -d postgres
node bin/cli.cjs setup --provider=openai
# Verify: setup succeeds

# Test 2: Scan without embeddings
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Verify: chunks inserted, no embeddings

# Test 3: Generate embeddings (tests updated_at)
# Already happens during scan with --generate-embeddings
# Verify: No "column updated_at does not exist" errors

# Test 4: Check database
docker exec maproom-postgres psql -U maproom -d maproom \
  -c "SELECT chunk_id, updated_at FROM maproom.chunks LIMIT 5;"
# Verify: updated_at column exists and has timestamps
```

**Success Criteria**:
- ✅ Fresh database: `updated_at` column created
- ✅ Existing database: Migration adds column without data loss
- ✅ Embedding updates succeed
- ✅ Timestamps update on re-indexing

**Not Testing**:
- Trigger edge cases (UPDATE vs INSERT distinction)
- Timezone handling (default TIMESTAMPTZ is fine)
- Historical data migration (we don't have production data)

### 3. CLI Workaround Removal (Integration Test)

**Purpose**: Verify OpenAI works WITHOUT explicit endpoint in CLI.

**Test Plan**:

```bash
# Setup with OpenAI
export OPENAI_API_KEY="sk-..."
node bin/cli.cjs setup --provider=openai

# Verify setup output
# Should NOT show: EMBEDDING_API_ENDPOINT: https://api.openai.com/v1/embeddings
# Should show: EMBEDDING_PROVIDER: openai

# Scan and generate embeddings
node bin/cli.cjs scan /workspace/packages/maproom-mcp

# Verify scan output
# Should show: Generated: N (N > 0)
# Should NOT show: Failed to generate code embeddings
# Should NOT show: Connection refused errors
```

**Success Criteria**:
- ✅ Setup completes without workaround code
- ✅ Scan generates embeddings
- ✅ No connection errors to localhost:11434
- ✅ Cost shows API calls made to OpenAI

**What We're NOT Testing**:
- Google Vertex AI (nice to have, not critical path)
- Ollama with custom endpoint (existing feature)
- Error recovery (not changed by fixes)

### 4. Environment Variable Precedence (Manual Verification)

**Purpose**: Verify environment variable contract is clear.

**Test Scenarios**:

```bash
# Scenario 1: Clean environment (should work)
unset EMBEDDING_API_ENDPOINT
export EMBEDDING_PROVIDER=openai
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Expected: Uses https://api.openai.com/v1/embeddings

# Scenario 2: Wrong endpoint for provider (should ignore)
export EMBEDDING_API_ENDPOINT=http://localhost:11434/api/embed
export EMBEDDING_PROVIDER=openai
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Expected: Ignores Ollama endpoint, uses OpenAI default

# Scenario 3: Correct custom endpoint (should use)
export EMBEDDING_API_ENDPOINT=https://api.openai.com/v2/embeddings
export EMBEDDING_PROVIDER=openai
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Expected: Uses custom OpenAI endpoint (if valid)

# Scenario 4: Ollama with custom endpoint (should work)
export EMBEDDING_API_ENDPOINT=http://remote-host:11434/api/embed
export EMBEDDING_PROVIDER=ollama
node bin/cli.cjs scan /workspace/packages/maproom-mcp
# Expected: Uses custom Ollama endpoint
```

**Success Criteria**:
- ✅ Scenarios match expected behavior
- ✅ Error messages are clear for wrong configurations
- ✅ Logs show which endpoint is being used

## Test Execution Order

1. **First**: Run Rust unit tests
   - Fastest feedback
   - Catches config logic bugs

2. **Second**: Manual database migration test
   - Requires Docker
   - One-time verification

3. **Third**: Integration test without workaround
   - End-to-end verification
   - Requires OpenAI API key

4. **Fourth**: Environment precedence verification
   - Optional but recommended
   - Builds confidence in fix

## What Success Looks Like

**Before Fix**:
```
$ node bin/cli.cjs scan /workspace/packages/maproom-mcp
...
[ERROR] Failed to generate code embeddings: Connection refused (localhost:11434)
Generated: 0, Failed: 854
```

**After Fix**:
```
$ node bin/cli.cjs scan /workspace/packages/maproom-mcp
...
📊 Embedding Generation Summary:
   Processed 854 chunks in 25s
   Provider: openai (1536 dimensions)
   Generated: 854, Cached: 0, Failed: 0
   API calls: 18, Tokens: 95000, Cost: $0.0019
```

## Regression Prevention

**Add to CI** (future):
```yaml
# .github/workflows/test.yml
- name: Test provider endpoint resolution
  run: cargo test config_tests::test_openai_ignores_ollama_endpoint
```

**Add to documentation**:
```markdown
## Environment Variables

- `EMBEDDING_PROVIDER`: Required. One of: openai, google, ollama
- `EMBEDDING_API_ENDPOINT`: Optional override. Only used for:
  - Ollama: Custom Ollama server location
  - Local: Required for local provider
  - Cloud providers: Ignored unless domain matches provider
```

## Limitations

**Not Testing**:
- Performance under load (not changed)
- Concurrent requests (not changed)
- Network failures and retries (existing error handling)
- Rate limiting (OpenAI SDK handles)
- Token counting accuracy (provider responsibility)
- Cost calculation edge cases (informational only)

**Why**: These tests would be ceremonial - they don't add confidence in the fix and would slow development.

## Risk Mitigation

**If tests fail**:
1. Check environment variables are set correctly
2. Verify Docker containers are running
3. Check API keys are valid
4. Review Rust compiler output for type errors
5. Rollback: Keep workaround in CLI until Rust fix validated

**If production breaks**:
1. Rollback to previous version (workaround in place)
2. Check logs for actual error
3. Verify database migration ran
4. Test with fresh database to isolate issue
