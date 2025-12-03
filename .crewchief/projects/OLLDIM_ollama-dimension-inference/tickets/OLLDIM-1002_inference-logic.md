# Ticket: [OLLDIM-1002]: Inference Logic in from_env()

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-developer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add dimension inference logic to `EmbeddingConfig::from_env()` that detects when Ollama provider is used without explicit dimension configuration, infers dimension from model name, and logs appropriate debug/warning messages.

## Background
With the helper function from OLLDIM-1001 in place, this ticket integrates it into the config loading flow. The inference logic must:
1. Default the model to "mxbai-embed-large" if Ollama provider has OpenAI default model
2. Only run for Ollama provider
3. Only run if dimension is not explicitly configured
4. Use inferred dimension or default with warning
5. Allow explicit dimension to override inference

Reference: Phase 1 Deliverable 2 from plan.md

## Acceptance Criteria
- [ ] Model defaulting occurs before inference for zero-config case
- [ ] Inference only runs for Ollama provider (Provider::Ollama check)
- [ ] Inference skipped if MAPROOM_EMBEDDING_DIMENSION environment variable is set
- [ ] Correct dimension inferred for known models (768 for nomic, 1024 for mxbai)
- [ ] Warning logged for unknown models with actionable guidance
- [ ] Debug log on successful inference showing model and dimension
- [ ] Debug log on model defaulting
- [ ] Explicit dimension override works (overrides inference)
- [ ] Code comments explain OpenAI-centric defaults and precedence
- [ ] All 6 integration unit tests pass

## Technical Requirements
- Location: `crates/maproom/src/embedding/config.rs` lines ~115-124 (after model loading, before dimension loading)
- Must call `infer_ollama_dimension()` helper function
- Must use `tracing::debug!()` for successful inference
- Must use `tracing::warn!()` for unknown models
- Must track explicit dimension separately from default
- Must maintain backward compatibility (explicit config unchanged)

## Implementation Notes

**Exact implementation provided in plan.md lines 62-109:**

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

**Unit tests required (plan.md lines 149-257):**

All tests must use `#[serial]` attribute and clean up environment variables. Required tests:

1. `test_from_env_infers_dimension_mxbai` - mxbai model infers 1024
2. `test_from_env_infers_dimension_nomic` - nomic model infers 768
3. `test_from_env_explicit_dimension_overrides_inference` - explicit wins over inferred
4. `test_from_env_unknown_model_keeps_default` - unknown model warns and uses default
5. `test_from_env_inference_only_for_ollama` - non-Ollama providers unaffected
6. `test_from_env_zero_config_ollama` - true zero-config with model defaulting

See plan.md lines 149-257 for complete test implementations.

## Dependencies
- **Prerequisite**: OLLDIM-1001 (helper function must exist)
- Uses `tracing` crate (already in dependencies)
- Uses `std::env` (standard library)

## Risk Assessment
- **Risk**: Incorrect precedence order (explicit vs inferred vs default)
  - **Mitigation**: Unit tests verify all three precedence scenarios. Code comments explain ordering.

- **Risk**: Logging creates too much noise
  - **Mitigation**: Use `debug!` level for successful inference (only visible with RUST_LOG). Use `warn!` only for actionable problems.

- **Risk**: Breaks existing explicit configuration
  - **Mitigation**: Explicit dimension is checked first and overrides everything. Backward compatibility test included.

## Files/Packages Affected
- `crates/maproom/src/embedding/config.rs` (modify from_env() method around lines 115-124)
- `crates/maproom/src/embedding/config.rs` (add 6 unit tests in existing `#[cfg(test)] mod tests` block)

## Verification Notes
The verify-ticket agent should confirm:
1. Inference logic is in correct location (after model loading, before dimension loading)
2. Model defaulting happens before inference
3. All conditional checks are correct (is_none, Provider::Ollama)
4. Logging uses appropriate levels (debug vs warn)
5. All 6 unit tests exist and pass
6. Tests use `#[serial]` attribute
7. Tests clean up environment variables
8. Code comments explain precedence and OpenAI defaults
9. No clippy warnings
10. Code formatted with `cargo fmt`
