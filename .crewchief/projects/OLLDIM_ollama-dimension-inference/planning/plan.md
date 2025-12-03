# Execution Plan: Ollama Dimension Inference

## Overview

Single-phase implementation to fix dimension inference bug in EmbeddingConfig::from_env(). This is a focused bug fix with minimal scope and clear deliverables.

---

## Phase 1: Dimension Inference Implementation

**Objective:** Fix EmbeddingConfig::from_env() to infer dimensions from Ollama model names.

**Estimated Effort:** 2-3 hours (small bug fix)

### Deliverables

#### 1. Helper Function: infer_ollama_dimension()
**File:** `crates/maproom/src/embedding/config.rs`
**Location:** After RetryConfig impl, before tests

**Implementation:**
```rust
/// Infer embedding dimension from known Ollama model names.
///
/// Uses prefix matching to handle model tags (e.g., "mxbai-embed-large:latest").
/// Returns the expected dimension for well-known models, or None for unknown models.
/// This enables zero-config workflows where dimension is automatically determined
/// from the model name without requiring explicit MAPROOM_EMBEDDING_DIMENSION.
///
/// # Supported Models
///
/// - `nomic-embed-text*`: 768 dimensions (matches tags like "nomic-embed-text:latest")
/// - `mxbai-embed-large*`: 1024 dimensions (matches tags like "mxbai-embed-large:v1")
///
/// # Returns
///
/// - `Some(dimension)` for known models
/// - `None` for unknown models (caller should warn and use default)
fn infer_ollama_dimension(model: &str) -> Option<usize> {
    if model.starts_with("nomic-embed-text") {
        Some(768)
    } else if model.starts_with("mxbai-embed-large") {
        Some(1024)
    } else {
        None
    }
}
```

**Acceptance Criteria:**
- [ ] Function compiles and passes clippy
- [ ] Docstring explains purpose and usage
- [ ] Returns correct dimensions for known models
- [ ] Returns None for unknown models
- [ ] Uses prefix matching to handle model tags

#### 2. Inference Logic in from_env()
**File:** `crates/maproom/src/embedding/config.rs`
**Location:** Lines ~115-124 (after model loading, before dimension loading)

**Implementation:**
```rust
// Load model from environment if specified
if let Ok(model) = env::var("MAPROOM_EMBEDDING_MODEL") {
    config.model = model;
}

// NEW: Default to Ollama model if provider is Ollama and model is still OpenAI default
// This ensures inference sees the correct model in zero-config scenarios
// Note: Config defaults are OpenAI-centric (dimension: 1536, model: "text-embedding-3-small")
// but factory defaults to Ollama with mxbai-embed-large when auto-detecting
if config.provider == Provider::Ollama && config.model == "text-embedding-3-small" {
    config.model = "mxbai-embed-large".to_string();
    tracing::debug!("Defaulting to mxbai-embed-large for Ollama provider");
}

// Track whether dimension was explicitly set (clearer than checking is_err() later)
let explicit_dimension = env::var("MAPROOM_EMBEDDING_DIMENSION").ok();

// NEW: Infer dimension for Ollama if not explicitly configured
// This fixes the bug where zero-config setups use wrong default dimension
// Precedence: explicit > inferred > default
if explicit_dimension.is_none() && config.provider == Provider::Ollama {
    if let Some(inferred_dim) = infer_ollama_dimension(&config.model) {
        tracing::debug!(
            "Inferred dimension {} for Ollama model '{}'",
            inferred_dim,
            config.model
        );
        config.dimension = inferred_dim;
    } else {
        tracing::warn!(
            "Unknown Ollama model '{}'. Cannot infer embedding dimension. \
             Please set MAPROOM_EMBEDDING_DIMENSION explicitly for custom models. \
             Defaulting to {} dimensions - this may cause errors if incorrect.",
            config.model,
            config.dimension
        );
    }
}

// Apply explicit dimension if provided (overrides inference)
if let Some(dim_str) = explicit_dimension {
    config.dimension = dim_str.parse().map_err(|_| ConfigError::InvalidValue {
        field: "EMBEDDING_DIMENSION".to_string(),
        reason: "Must be a positive integer".to_string(),
    })?;
}
```

**Acceptance Criteria:**
- [ ] Model defaulting occurs before inference for zero-config case
- [ ] Inference only runs for Ollama provider
- [ ] Inference skipped if MAPROOM_EMBEDDING_DIMENSION set
- [ ] Correct dimension inferred for known models
- [ ] Warning logged for unknown models
- [ ] Debug log on successful inference and model defaulting
- [ ] Explicit dimension overrides inference
- [ ] Code comments explain OpenAI-centric defaults and precedence

#### 3. Unit Tests
**File:** `crates/maproom/src/embedding/config.rs`
**Location:** In existing `#[cfg(test)] mod tests` block

**Test 1: Helper Function**
```rust
#[test]
fn test_infer_ollama_dimension_known_models() {
    assert_eq!(infer_ollama_dimension("nomic-embed-text"), Some(768));
    assert_eq!(infer_ollama_dimension("mxbai-embed-large"), Some(1024));
}

#[test]
fn test_infer_ollama_dimension_with_tags() {
    // Test prefix matching for model tags
    assert_eq!(infer_ollama_dimension("nomic-embed-text:latest"), Some(768));
    assert_eq!(infer_ollama_dimension("mxbai-embed-large:latest"), Some(1024));
    assert_eq!(infer_ollama_dimension("mxbai-embed-large:v1"), Some(1024));
}

#[test]
fn test_infer_ollama_dimension_unknown_model() {
    assert_eq!(infer_ollama_dimension("custom-model"), None);
    assert_eq!(infer_ollama_dimension("unknown"), None);
}
```

**Test 2: Inference Integration**
```rust
#[test]
#[serial]
fn test_from_env_infers_dimension_mxbai() {
    // Setup: Ollama provider with mxbai model, no explicit dimension
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
    env::set_var("MAPROOM_EMBEDDING_MODEL", "mxbai-embed-large");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

    let config = EmbeddingConfig::from_env().unwrap();

    assert_eq!(config.provider, Provider::Ollama);
    assert_eq!(config.model, "mxbai-embed-large");
    assert_eq!(config.dimension, 1024); // Inferred

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
}

#[test]
#[serial]
fn test_from_env_infers_dimension_nomic() {
    // Setup: Ollama provider with nomic model, no explicit dimension
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
    env::set_var("MAPROOM_EMBEDDING_MODEL", "nomic-embed-text");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

    let config = EmbeddingConfig::from_env().unwrap();

    assert_eq!(config.dimension, 768); // Inferred

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
}

#[test]
#[serial]
fn test_from_env_explicit_dimension_overrides_inference() {
    // Setup: Explicit dimension should win even if inference would differ
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
    env::set_var("MAPROOM_EMBEDDING_MODEL", "mxbai-embed-large");
    env::set_var("MAPROOM_EMBEDDING_DIMENSION", "768");

    let config = EmbeddingConfig::from_env().unwrap();

    assert_eq!(config.dimension, 768); // Explicit wins over inferred 1024

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");
}

#[test]
#[serial]
fn test_from_env_unknown_model_keeps_default() {
    // Setup: Unknown model should log warning and keep default
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
    env::set_var("MAPROOM_EMBEDDING_MODEL", "custom-model");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

    let config = EmbeddingConfig::from_env().unwrap();

    assert_eq!(config.dimension, 1536); // Default unchanged

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
}

#[test]
#[serial]
fn test_from_env_inference_only_for_ollama() {
    // Setup: Non-Ollama provider should not trigger inference
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");
    env::set_var("MAPROOM_EMBEDDING_MODEL", "mxbai-embed-large");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

    let config = EmbeddingConfig::from_env().unwrap();

    assert_eq!(config.dimension, 1536); // Default unchanged (not Ollama)

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
}

#[test]
#[serial]
fn test_from_env_zero_config_ollama() {
    // Setup: True zero-config - no env vars, provider will be Ollama
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");

    // Simulate the case where provider is set to Ollama but no explicit model
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");

    let config = EmbeddingConfig::from_env().unwrap();

    assert_eq!(config.provider, Provider::Ollama);
    assert_eq!(config.model, "mxbai-embed-large"); // Defaulted in config
    assert_eq!(config.dimension, 1024); // Inferred from defaulted model

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
}
```

**Acceptance Criteria:**
- [ ] All tests use `#[serial]` attribute (tests modify env vars)
- [ ] Tests clean up environment variables
- [ ] All assertions pass
- [ ] Tests cover happy path and edge cases
- [ ] Tests verify model tag handling with prefix matching
- [ ] Tests verify zero-config scenario with model defaulting

#### 4. Integration Test
**File:** `crates/maproom/src/embedding/factory.rs`
**Location:** In existing `#[cfg(test)] mod tests` block

```rust
#[tokio::test]
#[serial]
async fn test_zero_config_infers_dimension_mxbai() {
    // Clean environment
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");
    env::remove_var("MAPROOM_EMBEDDING_API_ENDPOINT");

    // Set up minimal config for Ollama
    env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");
    env::set_var("MAPROOM_EMBEDDING_MODEL", "mxbai-embed-large");

    let result = create_provider_from_env().await;

    // Cleanup
    env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
    env::remove_var("MAPROOM_EMBEDDING_MODEL");

    assert!(result.is_ok(), "Provider creation should succeed");
    let provider = result.unwrap();
    assert_eq!(provider.provider_name(), "ollama");
    assert_eq!(provider.dimension(), 1024); // Correctly inferred
}
```

**Acceptance Criteria:**
- [ ] Test passes with real provider creation
- [ ] Dimension correctly flows through factory
- [ ] Test is marked with `#[serial]`

### Agent Assignments

**rust-developer:**
- Implement `infer_ollama_dimension()` helper function
- Add inference logic to `EmbeddingConfig::from_env()`
- Add logging (debug on success, warn on unknown model)

**unit-test-runner:**
- Create and run all unit tests in config.rs
- Create and run integration test in factory.rs
- Verify test coverage with `cargo test -p crewchief-maproom`

**verify-ticket:**
- Run existing test suite to ensure no regressions
- Verify backward compatibility (explicit config still works)
- Check that validation warnings still fire appropriately

**commit-ticket:**
- Create commit with message: "fix(embedding): infer Ollama dimensions from model name"
- Include ticket reference and bug description

### Dependencies
- None (self-contained change in single module)

### Risk Factors
- **Low Risk:** Minimal code change (~30 lines)
- **High Test Coverage:** 7+ tests covering all paths
- **Backward Compatible:** Only affects zero-config workflows

---

## Success Metrics

### Code Quality
- [ ] All tests pass: `cargo test -p crewchief-maproom`
- [ ] No clippy warnings: `cargo clippy -p crewchief-maproom`
- [ ] Formatted correctly: `cargo fmt --check`

### Functionality
- [ ] Zero-config with mxbai-embed-large uses 1024 dimensions
- [ ] Zero-config with nomic-embed-text uses 768 dimensions
- [ ] Explicit MAPROOM_EMBEDDING_DIMENSION overrides inference
- [ ] Unknown models log warning and use default
- [ ] Non-Ollama providers unaffected

### User Experience
- [ ] No dimension mismatch errors in zero-config setups
- [ ] Clear warning messages for unknown models
- [ ] Debug logs help troubleshoot configuration

### Documentation
- [ ] Helper function has clear docstring
- [ ] Inline comments explain inference logic
- [ ] Warning message provides actionable guidance

---

## Testing Checklist

### Unit Tests (config.rs)
- [ ] `test_infer_ollama_dimension_known_models` - Helper function correctness
- [ ] `test_infer_ollama_dimension_with_tags` - Prefix matching for model tags
- [ ] `test_infer_ollama_dimension_unknown_model` - Unknown model handling
- [ ] `test_from_env_infers_dimension_mxbai` - mxbai inference
- [ ] `test_from_env_infers_dimension_nomic` - nomic inference
- [ ] `test_from_env_explicit_dimension_overrides_inference` - Explicit wins
- [ ] `test_from_env_unknown_model_keeps_default` - Unknown model behavior
- [ ] `test_from_env_inference_only_for_ollama` - Provider specificity
- [ ] `test_from_env_zero_config_ollama` - Zero-config with model defaulting

### Integration Tests (factory.rs)
- [ ] `test_zero_config_infers_dimension_mxbai` - End-to-end zero-config

### Regression Tests
- [ ] Existing config tests pass unchanged
- [ ] Existing factory tests pass unchanged
- [ ] Existing ollama tests pass unchanged
- [ ] Validation tests still warn on mismatches

### Manual Testing
- [ ] Zero-config: No env vars → works with mxbai-embed-large at 1024 dim
- [ ] Explicit config: MAPROOM_EMBEDDING_DIMENSION=768 → honored
- [ ] Model switch: Change to nomic-embed-text → infers 768
- [ ] Unknown model: custom-model → warns and uses default

---

## Rollback Plan

### If Tests Fail
1. Revert changes to config.rs
2. Keep tests for future attempts
3. Document failure reason

### If Integration Issues
1. Add feature flag for inference (disabled by default)
2. Roll out incrementally
3. Gather feedback before enabling by default

### If User Complaints
1. Dimension mismatch errors still possible (unknown models)
2. Warning messages guide users to explicit configuration
3. Explicit config always available as escape hatch

---

## Post-Implementation

### Verification Steps
1. Run full test suite: `cargo test -p crewchief-maproom`
2. Test zero-config workflow manually
3. Verify logs show inference decisions
4. Check validation still warns on mismatches

### Documentation Updates
**File:** `crates/maproom/CLAUDE.md`
**Add section:**
```markdown
## Embedding Dimension Configuration

Maproom automatically infers embedding dimensions for known Ollama models:
- `mxbai-embed-large*`: 1024 dimensions (default, matches tags like `:latest`)
- `nomic-embed-text*`: 768 dimensions (matches tags like `:latest`)

To override automatic inference or configure custom models:
```bash
export MAPROOM_EMBEDDING_DIMENSION=512
```

Explicit configuration always takes precedence over inference.

## After Upgrading to Dimension Inference

If you previously experienced dimension mismatch errors:
1. The fix is automatic - no configuration changes needed
2. Existing embeddings are dimension-tagged and remain valid
3. New embeddings will use correct inferred dimensions
4. No regeneration required

Zero-config workflows now work correctly:
```bash
# No environment variables needed for Ollama with standard models
crewchief-maproom generate-embeddings --repo myrepo
# Automatically uses mxbai-embed-large at 1024 dimensions
```
```

### Release Notes
```markdown
### Bug Fix: Ollama Dimension Inference (OLLDIM)

Fixed dimension mismatch errors in zero-config Ollama setups. Maproom now
automatically infers embedding dimensions from model names:
- mxbai-embed-large → 1024 dimensions
- nomic-embed-text → 768 dimensions

Explicit MAPROOM_EMBEDDING_DIMENSION configuration continues to work unchanged.
```

---

## Timeline

**Total Estimated Time:** 2-3 hours

- **Implementation:** 1 hour
  - Helper function: 15 minutes
  - Inference logic: 30 minutes
  - Logging: 15 minutes

- **Testing:** 1-1.5 hours
  - Unit tests: 45 minutes
  - Integration test: 15 minutes
  - Manual verification: 15 minutes

- **Documentation:** 30 minutes
  - Code comments: 15 minutes
  - CLAUDE.md update: 15 minutes
