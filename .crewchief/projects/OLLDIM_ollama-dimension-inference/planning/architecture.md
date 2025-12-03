# Architecture: Ollama Dimension Inference

## High-Level Overview

Add dimension inference logic to `EmbeddingConfig::from_env()` that detects Ollama model names and sets appropriate dimensions when not explicitly configured. The solution maintains backward compatibility while fixing zero-config workflows.

## Key Design Decisions

### 1. Inference Location
**Decision:** Add inference logic in `EmbeddingConfig::from_env()` after model loading, before validation.

**Rationale:**
- Single point of configuration assembly
- Access to both provider and model information
- Occurs before validation warnings
- No API changes required

**Alternative Rejected:** Factory-level inference would require changing multiple call sites and break encapsulation.

### 2. Explicit Config Priority
**Decision:** Only infer dimension if `MAPROOM_EMBEDDING_DIMENSION` environment variable is NOT set.

**Rationale:**
- Preserves explicit user configuration
- Backward compatible with existing workflows
- Clear precedence: explicit > inferred > default

**Implementation:**
```rust
// Track whether dimension was explicitly set
let explicit_dimension = env::var("MAPROOM_EMBEDDING_DIMENSION").ok();

// ... load model ...

// Infer dimension if not explicit and provider is Ollama
if explicit_dimension.is_none() && config.provider == Provider::Ollama {
    config.dimension = infer_dimension_from_model(&config.model);
}
```

### 3. Model-to-Dimension Mapping
**Decision:** Use static const mapping in config.rs with hardcoded known models.

**Rationale:**
- Simple, fast lookup
- No external dependencies
- Easy to maintain and extend
- Co-located with validation logic

**Model Mapping:**
```rust
/// Infer embedding dimension from Ollama model names.
/// Uses prefix matching to handle model tags (e.g., "mxbai-embed-large:latest").
fn infer_ollama_dimension(model: &str) -> Option<usize> {
    if model.starts_with("nomic-embed-text") {
        Some(768)
    } else if model.starts_with("mxbai-embed-large") {
        Some(1024)
    } else {
        None  // Unknown model
    }
}
```

**Rationale for Prefix Matching:** Users may specify model tags like `mxbai-embed-large:latest` or version-specific tags. Using `starts_with()` ensures inference works correctly regardless of tag specification.

### 4. Unknown Model Handling
**Decision:** Log warning and keep default dimension for unknown models.

**Rationale:**
- Non-breaking for custom Ollama models
- Provides clear guidance to users
- Fails safely (validation will catch mismatch later)

**Warning Message:**
```
WARN: Unknown Ollama model 'custom-model'. Cannot infer embedding dimension.
Please set MAPROOM_EMBEDDING_DIMENSION explicitly for custom models.
Defaulting to 1536 dimensions - this may cause errors if incorrect.
```

### 5. Validation Preservation
**Decision:** Keep existing validation warnings unchanged.

**Rationale:**
- Validation catches inference errors
- Warnings help users verify configuration
- Existing test coverage preserved

## Component Design

### Modified Component: EmbeddingConfig::from_env()

**Current Flow:**
```
1. Create default config (dimension: 1536)
2. Load provider from env
3. Load model from env
4. Load dimension from env (if set)
5. Load other config...
6. Return config
```

**New Flow:**
```
1. Create default config (dimension: 1536, model: "text-embedding-3-small")
2. Load provider from env
3. Load model from env (if set)
4. If provider is Ollama AND model is still OpenAI default:
   - Set model to "mxbai-embed-large" (Ollama default)
   - This ensures inference sees the correct model
5. Check if dimension explicitly set
6. If NOT explicit AND provider is Ollama:
   - Infer dimension from model name
   - Log inference decision
   - Set config.dimension
7. Load dimension from env (overrides inference if set)
8. Load other config...
9. Return config
```

**Critical Addition:** Step 4 handles the zero-config case where the config has OpenAI defaults but the factory will use Ollama. By defaulting the model in the config layer (before inference), we ensure inference sees the model that will actually be used.

**Key Change Location:**
```rust
// Around line 115, after provider loading
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

// Track whether dimension was explicitly set (for inference decision)
let explicit_dimension = env::var("MAPROOM_EMBEDDING_DIMENSION").ok();

// NEW: Infer dimension for Ollama if not explicitly set
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

### Helper Function: infer_ollama_dimension()

**Signature:**
```rust
fn infer_ollama_dimension(model: &str) -> Option<usize>
```

**Location:** `crates/maproom/src/embedding/config.rs` (module-private helper)

**Implementation:**
```rust
/// Infer embedding dimension from known Ollama model names.
///
/// Uses prefix matching to handle model tags (e.g., "mxbai-embed-large:latest").
/// Returns None for unknown models, allowing caller to handle appropriately.
/// This enables zero-config workflows where dimension is automatically determined
/// from the model name without requiring explicit MAPROOM_EMBEDDING_DIMENSION.
///
/// # Supported Models
///
/// - `nomic-embed-text*`: 768 dimensions
/// - `mxbai-embed-large*`: 1024 dimensions
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

## Data Flow

### Successful Zero-Config Flow
```
User runs: crewchief-maproom generate-embeddings --repo test
    ↓
factory::create_provider_from_env()
    ↓
EmbeddingConfig::from_env()
    ↓
1. No MAPROOM_EMBEDDING_PROVIDER → defaults to auto-detect (later resolves to Ollama)
2. Provider set to Ollama (auto-detection)
3. No MAPROOM_EMBEDDING_MODEL → model still "text-embedding-3-small" (OpenAI default)
4. Model defaulting: Provider is Ollama, model is OpenAI default → set to "mxbai-embed-large"
5. No MAPROOM_EMBEDDING_DIMENSION → triggers inference
6. infer_ollama_dimension("mxbai-embed-large") → Some(1024)
7. config.dimension = 1024
    ↓
OllamaProvider::new_with_config(..., model="mxbai-embed-large", dimension=1024)
    ↓
Embedding generation succeeds ✓
```

### Explicit Override Flow
```
User sets: MAPROOM_EMBEDDING_DIMENSION=768
User sets: MAPROOM_EMBEDDING_MODEL=mxbai-embed-large
    ↓
EmbeddingConfig::from_env()
    ↓
1. Model loaded: "mxbai-embed-large"
2. Inference would suggest: 1024
3. MAPROOM_EMBEDDING_DIMENSION is set → skip inference
4. Explicit dimension loaded: 768
5. config.dimension = 768 (explicit wins)
    ↓
Validation warning: "mxbai-embed-large typically uses 1024, got 768"
    ↓
OllamaProvider created with dimension=768 (explicit config honored)
```

### Unknown Model Flow
```
User sets: MAPROOM_EMBEDDING_MODEL=custom-model
    ↓
EmbeddingConfig::from_env()
    ↓
1. Model loaded: "custom-model"
2. infer_ollama_dimension("custom-model") → None
3. Warning logged: "Unknown model, set dimension explicitly"
4. config.dimension = 1536 (default unchanged)
    ↓
If actual dimension differs → validation error later
User sees helpful message about setting dimension explicitly
```

## Integration Points

### 1. Factory Integration
**Location:** `crates/maproom/src/embedding/factory.rs:209-214`

**Current Code:**
```rust
let config = EmbeddingConfig::from_env()?;
let dimension = config.dimension;  // Now correctly inferred
```

**Change Required:** None - factory automatically benefits from inference.

### 2. Validation Integration
**Location:** `crates/maproom/src/embedding/config.rs:265-286`

**Behavior:** Unchanged - validation still warns on mismatches (including inference errors).

### 3. OllamaProvider Integration
**Location:** `crates/maproom/src/embedding/ollama.rs`

**Change Required:** None - already accepts any dimension parameter.

## Technology Choices

### Why Not Query Ollama API?
**Rejected:** Dynamic dimension detection via Ollama API calls.

**Reasons:**
1. **Performance:** Adds network latency to every config load
2. **Offline Support:** Breaks workflows without Ollama running
3. **Reliability:** Network failures would prevent configuration
4. **Simplicity:** Static mapping is faster and more predictable

### Why Not Configuration File?
**Rejected:** Move model dimensions to external config file.

**Reasons:**
1. **Complexity:** Adds file I/O and parsing overhead
2. **Deployment:** Complicates binary distribution
3. **Maintenance:** Two sources of truth for same information
4. **Zero-Config:** Contradicts zero-config philosophy

### Why Not Trait Method?
**Rejected:** Add `default_dimension()` method to Provider trait.

**Reasons:**
1. **Wrong Abstraction:** Dimension is model-specific, not provider-specific
2. **API Changes:** Would require updating all provider implementations
3. **Scope Creep:** OllamaProvider supports multiple dimensions per provider

## Error Handling

### Inference Errors
**Scenario:** Unknown model name.
**Handling:** Log warning, keep default, let validation catch mismatches.

### Parse Errors
**Scenario:** Invalid MAPROOM_EMBEDDING_DIMENSION value.
**Handling:** Existing error handling unchanged (returns ConfigError).

### Provider Mismatch
**Scenario:** Ollama model name with non-Ollama provider.
**Handling:** No interference - inference only triggers for Ollama provider.

## Performance Considerations

### Inference Cost
- **Operation:** String comparison against 2-3 known values
- **Cost:** Negligible (< 1 microsecond)
- **Frequency:** Once per process startup
- **Impact:** Zero measurable performance impact

### Memory Impact
- **Added Code:** ~20 lines (helper function + inference logic)
- **Runtime Memory:** None (no new data structures)
- **Binary Size:** Negligible increase

## Testing Strategy

### Unit Tests (config.rs)
1. `test_infer_dimension_mxbai_embed_large()` - Infers 1024
2. `test_infer_dimension_nomic_embed_text()` - Infers 768
3. `test_infer_dimension_unknown_model()` - Returns None
4. `test_explicit_dimension_overrides_inference()` - Explicit wins
5. `test_inference_only_for_ollama()` - OpenAI ignored

### Integration Tests (factory.rs)
1. `test_zero_config_infers_dimension()` - End-to-end zero-config
2. `test_explicit_dimension_honored()` - Explicit config preserved

### Regression Tests
Existing tests should pass unchanged (dimension inference doesn't break existing behavior).

## Migration Strategy

### Rollout Plan
**Phase 1:** Ship fix in next release (no migration needed - backward compatible)

### User Communication
**Release Notes:**
```
### Bug Fix: Ollama Dimension Inference

Maproom now automatically infers embedding dimensions from Ollama model names:
- mxbai-embed-large: 1024 dimensions (default)
- nomic-embed-text: 768 dimensions

This fixes dimension mismatch errors in zero-config setups. Existing explicit
MAPROOM_EMBEDDING_DIMENSION configurations continue to work unchanged.
```

### Breaking Changes
**None** - This is a bug fix that makes zero-config work as intended.

## Risks and Mitigations

### Risk: Incorrect Inference
**Scenario:** Model dimensions change upstream in Ollama.
**Likelihood:** Very low (model dimensions are fixed at training time).
**Mitigation:** Validation warnings catch mismatches, user can override explicitly.

### Risk: Model Name Variations
**Scenario:** Users specify "mxbai-embed-large:latest" or "mxbai-embed-large@sha256:...".
**Likelihood:** Medium (Ollama supports tags).
**Mitigation:** Use `.starts_with()` or `.contains()` for flexible matching.

### Risk: Custom Models
**Scenario:** User runs custom fine-tuned model with standard name.
**Likelihood:** Low (users typically rename custom models).
**Mitigation:** Warning message guides users to set dimension explicitly.

### Risk: Validation Confusion
**Scenario:** Inference sets wrong dimension, validation warns but user ignores.
**Likelihood:** Low (validation happens before generation).
**Mitigation:** Clear error messages during actual embedding generation.

## Future Enhancements

### Potential Extensions (Out of Scope)
1. **Dynamic Model Detection:** Query Ollama for available models and dimensions
2. **Model Registry:** External file with community-maintained model mappings
3. **Provider-Level Defaults:** Per-provider default dimensions in configuration
4. **Automatic Testing:** Integration tests against real Ollama instance

These are intentionally excluded to keep the fix focused and minimize risk.

## Code Reuse

### Leveraged Existing Code
1. **Validation Logic:** Reuse model name strings from validation (config.rs:268-281)
2. **Provider Enum:** Use existing Provider::Ollama type check
3. **Error Types:** Use existing ConfigError for consistency

### No New Dependencies
This fix requires zero new external dependencies - pure Rust standard library.
