# Quality Strategy: Maproom Skill Progressive Disclosure (MPRSKL)

## Testing Philosophy

This ticket combines Rust code changes (bug fix, error messages) with documentation restructuring (skills). Testing strategy reflects both:

1. **Rust code**: Comprehensive unit and integration tests with existing coverage maintained
2. **Documentation**: Manual validation and agent testing
3. **Integration**: End-to-end verification of zero-config workflow

## Coverage Requirements

**Minimum Thresholds:**
- Line coverage: Must meet or exceed existing thresholds in `crates/maproom/`
- Branch coverage: All new conditional logic must have explicit tests

**Existing test infrastructure:**
- Framework: `#[tokio::test]` for async, `#[test]` for sync
- Serial tests: `#[serial]` for env var manipulation tests
- Location: Inline `mod tests` blocks in each source file

## Test Types

### Unit Tests

**Scope:** `EmbeddingConfig::from_env_with_provider()` and related config logic

**Tools:** Rust std test, serial_test crate

**Coverage Target:** 100% of new code paths

**What to Test:**

1. **from_env_with_provider() basic behavior**
   - `from_env_with_provider(None)` returns same result as `from_env()`
   - `from_env_with_provider(Some(Provider::Ollama))` sets provider correctly
   - Provider override is applied before env var loading

2. **Dimension inference with provider override**
   - Provider::Ollama + no MAPROOM_EMBEDDING_DIMENSION -> infers from model
   - Provider::Ollama + mxbai-embed-large -> dimension 1024
   - Provider::Ollama + nomic-embed-text -> dimension 768
   - Provider::Ollama + unknown model -> keeps default with warning

3. **Env var override precedence**
   - Provider override + explicit env var -> env var wins
   - Provider override + no env var -> override applies
   - Dimension inference respects explicit MAPROOM_EMBEDDING_DIMENSION

4. **Backward compatibility**
   - All existing `from_env()` tests must continue passing
   - No behavioral changes when provider_override is None

**Note:** These test cases validate the `from_env_with_provider()` method added in MPRSKL.1001. This method does not exist in the current codebase but is required to fix the zero-config Ollama dimension mismatch bug verified by user testing (December 2025).

**Example Test Cases:**

```rust
#[test]
#[serial]
fn test_from_env_with_provider_ollama_infers_dimension() {
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");

    let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama)).unwrap();

    assert_eq!(config.provider, Provider::Ollama);
    assert_eq!(config.model, "mxbai-embed-large"); // Default for Ollama
    assert_eq!(config.dimension, 1024); // Inferred from model
}

#[test]
#[serial]
fn test_from_env_with_provider_none_same_as_from_env() {
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");
    env::set_var("MAPROOM_EMBEDDING_DIMENSION", "512");

    let with_none = EmbeddingConfig::from_env_with_provider(None).unwrap();
    let plain = EmbeddingConfig::from_env().unwrap();

    assert_eq!(with_none.provider, plain.provider);
    assert_eq!(with_none.dimension, plain.dimension);

    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");
}

#[test]
#[serial]
fn test_from_env_with_provider_env_var_overrides_override() {
    // Env var should win over programmatic override
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");

    let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama)).unwrap();

    // Env var "openai" should override programmatic Provider::Ollama
    assert_eq!(config.provider, Provider::OpenAI);

    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
}
```

### Integration Tests

**Scope:** Factory/config coordination, auto-detection flow

**Approach:** Test full `create_provider_from_env()` flow with mocked or real Ollama

**Key Test:**

```rust
#[tokio::test]
#[serial]
async fn test_auto_detected_ollama_uses_correct_dimension() {
    // Clean environment
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

    // This test requires Ollama running, so mark as #[ignore] for CI
    // Or mock the detection endpoint

    let result = create_provider_from_env().await;

    match result {
        Ok(provider) => {
            assert_eq!(provider.provider_name(), "ollama");
            assert_eq!(provider.dimension(), 1024); // mxbai-embed-large default
        }
        Err(_) => {
            // Ollama not available - acceptable in CI
            // This is why the test should be #[ignore] by default
        }
    }
}
```

### Documentation Tests

**Scope:** Skill file structure, content accuracy, link validity

**Approach:** Manual validation checklist + optional agent testing

**Checklist:**
- [ ] SKILL.md line count < 50
- [ ] SKILL.md contains YAML frontmatter
- [ ] All reference links in SKILL.md resolve to existing files
- [ ] cli-reference.md documents all crewchief-maproom commands
- [ ] troubleshooting.md covers dimension mismatch error
- [ ] No broken internal links

## Critical Paths

The following paths MUST have comprehensive test coverage:

### 1. Zero-Config Ollama Flow (Primary Fix Target)

**Happy Path:**
- Ollama running at localhost:11434
- No env vars set
- Factory detects Ollama, passes Provider::Ollama to config
- Config infers dimension 1024 for mxbai-embed-large
- Provider created successfully

**Error Cases:**
- Ollama not running -> clear error message
- Model mismatch -> warning logged
- Dimension mismatch -> enhanced error message

**Edge Cases:**
- Custom Ollama endpoint
- Non-standard model name
- Explicit env var overrides auto-detection

### 2. Backward Compatibility

**Happy Path:**
- Explicit `MAPROOM_EMBEDDING_PROVIDER=ollama` works unchanged
- Explicit `MAPROOM_EMBEDDING_DIMENSION` overrides inference
- OpenAI provider configuration unchanged

**Error Cases:**
- Invalid provider name
- Missing API key for OpenAI/Google

### 3. Config Precedence

**Test Matrix:**

| Provider Override | MAPROOM_EMBEDDING_PROVIDER | Result |
|------------------|---------------------------|--------|
| None | Not set | Default (OpenAI) |
| None | "ollama" | Ollama |
| Some(Ollama) | Not set | Ollama |
| Some(Ollama) | "openai" | OpenAI (env wins) |

## Negative Testing Requirements

### Invalid Inputs and Malformed Data

- Invalid provider name: "invalid_provider" -> descriptive error
- Invalid dimension: "not_a_number" -> parse error with field name
- Invalid model name with dimension inference: Unknown model -> warning + default

### Error Handling Paths

- Network failure during Ollama detection -> fallback error message
- Database connection failure during scan -> clear error
- Embedding generation failure -> warning, scan continues

### Configuration Failures

- Missing OpenAI API key -> specific error with instructions
- Missing Google credentials -> specific error with instructions
- Invalid credentials file -> file validation error

## Test Data Strategy

**Environment Variable Tests:**
- Use `#[serial]` to prevent race conditions
- Clean up env vars in test teardown
- Document env vars used in each test

**Mock Data:**
- No external service mocking needed for config tests
- Factory tests may need Ollama mock or `#[ignore]`

## Quality Gates

Before verification:

- [ ] All existing tests pass (`cargo test -p crewchief-maproom`)
- [ ] New unit tests for `from_env_with_provider()` pass
- [ ] Integration test for auto-detection (if Ollama available)
- [ ] Coverage thresholds maintained
- [ ] No new clippy warnings (`cargo clippy -p crewchief-maproom`)
- [ ] Code formatted (`cargo fmt -- --check`)

Documentation quality:

- [ ] SKILL.md under 50 lines
- [ ] All reference files exist
- [ ] All links valid
- [ ] Content matches current CLI behavior

## Test File Locations

| Test Type | Location |
|-----------|----------|
| Config unit tests | `crates/maproom/src/embedding/config.rs` (inline `mod tests`) |
| Factory integration tests | `crates/maproom/src/embedding/factory.rs` (inline `mod tests`) |
| Error tests | `crates/maproom/src/embedding/error.rs` |
| CLI behavior | `crates/maproom/src/main.rs` (inline `mod tests`) |

## CI Integration

Existing CI workflow runs:
- `cargo test` - all unit and integration tests
- `cargo clippy` - lint checks
- `cargo fmt -- --check` - format checks

No CI changes needed for this ticket. New tests follow existing patterns.
