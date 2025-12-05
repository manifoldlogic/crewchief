# Analysis: Ollama Dimension Inference

## Problem Definition

The `EmbeddingConfig::from_env()` function has a critical bug where it fails to infer embedding dimensions from Ollama model names, causing dimension mismatches when users rely on auto-detection without explicit configuration.

**Specific Issue:**
```rust
// Current behavior in config.rs:105-124
pub fn from_env() -> Result<Self, EmbeddingError> {
    let mut config = Self::default();  // Sets dimension: 1536 (OpenAI default)

    // Load provider
    if let Ok(provider) = env::var("MAPROOM_EMBEDDING_PROVIDER") {
        config.provider = provider.parse()?;
    }

    // Load dimension - ONLY updates if explicitly set
    if let Ok(dim) = env::var("MAPROOM_EMBEDDING_DIMENSION") {
        config.dimension = dim.parse()?;
    }
    // BUG: No dimension inference from model name!
}
```

**Failure Scenario:**
```bash
# User auto-detects Ollama with mxbai-embed-large
# No explicit MAPROOM_EMBEDDING_DIMENSION set
$ crewchief-maproom generate-embeddings --repo myrepo

# Result:
# - Model: mxbai-embed-large (produces 1024-dim vectors)
# - Dimension: 1536 (hardcoded default, not updated)
# - Error: "Dimension mismatch in batch at index 0: expected 1536 dimensions but got 1024"
```

## Context

### Zero-Config Philosophy
Maproom implements a "zero-config" experience for Ollama embeddings where users can start indexing without setting any environment variables. This works through:
1. Auto-detection of Ollama at localhost:11434
2. Default model selection (mxbai-embed-large)
3. **MISSING:** Dimension inference from model name

### Recent Changes
The codebase recently transitioned from nomic-embed-text (768-dim) to mxbai-embed-large (1024-dim) as the default model, but the dimension default remained at 1536 (OpenAI's dimension).

**Important Note on Defaults:** The `EmbeddingConfig::default()` uses OpenAI defaults (1536-dim, "text-embedding-3-small"), but the factory defaults to Ollama with "mxbai-embed-large" when auto-detecting. This creates a mismatch - the config layer has the OpenAI model, but inference needs to see the Ollama model that will eventually be used. The fix must handle model defaulting in the config layer before inference runs.

## Existing Solutions

### Industry Patterns
Most embedding providers handle dimension inference automatically:
- **OpenAI SDK:** Infers dimensions from model name
- **Cohere SDK:** Model metadata includes dimension
- **LangChain:** Maintains model dimension mapping

### Codebase Patterns

**Validation Logic (config.rs:265-286):**
The codebase already has dimension validation that knows expected dimensions:
```rust
if self.provider == Provider::Ollama {
    match self.model.as_str() {
        "nomic-embed-text" if self.dimension != 768 => {
            tracing::warn!("nomic-embed-text typically uses 768 dimensions, got {}.", self.dimension);
        }
        "mxbai-embed-large" if self.dimension != 1024 => {
            tracing::warn!("mxbai-embed-large typically uses 1024 dimensions, got {}.", self.dimension);
        }
        _ => {}
    }
}
```

This validation logic proves the codebase already understands the model-to-dimension mapping. The bug is that this knowledge isn't used proactively during configuration loading.

**Factory Pattern (factory.rs:209-229):**
The factory creates OllamaProvider with explicit dimensions:
```rust
let model = env::var("MAPROOM_EMBEDDING_MODEL")
    .unwrap_or_else(|_| "mxbai-embed-large".to_string());

let config = EmbeddingConfig::from_env()?;
let dimension = config.dimension;  // BUG: Uses wrong default

let provider = OllamaProvider::new_with_config(endpoint, model, dimension, parallel_config)?;
```

## Current State

### Code Locations
1. **Bug Location:** `crates/maproom/src/embedding/config.rs:105-124`
2. **Validation Logic:** `crates/maproom/src/embedding/config.rs:265-286`
3. **Factory Usage:** `crates/maproom/src/embedding/factory.rs:197-229`
4. **OllamaProvider:** `crates/maproom/src/embedding/ollama.rs` (accepts any dimension)

### Known Model Dimensions
From grep results and validation code:
- **nomic-embed-text:** 768 dimensions
- **mxbai-embed-large:** 1024 dimensions (default)
- **Unknown models:** Should warn and require explicit configuration

### Test Coverage
Existing tests validate:
- Dimension validation warnings (config.rs:717-742)
- Dimension parameter passing (ollama.rs:1015-1086)
- Model name matching (ollama.rs:1215-1235)

Missing tests:
- Dimension inference from model name
- Zero-config workflow with auto-detected models

## Research Findings

### Key Insights

1. **The Knowledge Exists:** The codebase already knows the correct dimensions for each model (validation logic)

2. **Separation of Concerns:** The bug exists because dimension loading is separate from model loading, with no cross-validation

3. **Default Values Problem:** The OpenAI default (1536) is inappropriate for the Ollama default model (mxbai-embed-large, 1024)

4. **Zero-Config Breaks:** Users following the zero-config path encounter errors because dimension inference is missing

5. **Explicit Config Works:** Users who manually set MAPROOM_EMBEDDING_DIMENSION don't hit this bug

### Existing Infrastructure

The codebase has all the pieces needed:
- Model name parsing (from env vars or defaults)
- Dimension validation logic (knows expected values)
- Provider-aware configuration (can check provider type)

The fix just needs to connect these pieces during configuration loading.

## Constraints

### Technical Constraints
1. **Backward Compatibility:** Must not break existing configurations where users explicitly set MAPROOM_EMBEDDING_DIMENSION
2. **Custom Models:** Must handle unknown Ollama models gracefully (can't infer dimension)
3. **Provider Specificity:** Only infer for Ollama provider (OpenAI/Cohere have fixed dimensions)
4. **Validation Order:** Must preserve existing validation warnings for explicit mismatches

### Design Constraints
1. **Zero-Config Priority:** Should work with no environment variables (current user expectation)
2. **Explicit Wins:** Explicit MAPROOM_EMBEDDING_DIMENSION should override inference
3. **No External Calls:** Cannot query Ollama API for model metadata (would break offline workflows)
4. **Static Mapping:** Must maintain hardcoded model-to-dimension mapping

### Implementation Constraints
1. **Single Function Change:** Fix should be localized to `EmbeddingConfig::from_env()`
2. **No Breaking Changes:** API compatibility must be preserved
3. **Test Addition:** Must add test coverage for inference path

## Success Criteria

### Functional Success
1. **Zero-Config Works:** `mxbai-embed-large` auto-detection correctly infers 1024 dimensions
2. **Explicit Config Wins:** MAPROOM_EMBEDDING_DIMENSION always takes precedence
3. **Unknown Models Warn:** Custom models log warning and prompt for explicit dimension
4. **Backward Compatible:** Existing workflows continue working unchanged

### Test Coverage Success
1. **Inference Test:** Test that mxbai-embed-large infers 1024 without MAPROOM_EMBEDDING_DIMENSION
2. **Explicit Override:** Test that MAPROOM_EMBEDDING_DIMENSION=768 overrides inference
3. **Unknown Model:** Test that unknown-model logs warning and keeps default
4. **Nomic Legacy:** Test that nomic-embed-text infers 768 correctly

### User Experience Success
1. **No Error:** Zero-config users don't see dimension mismatch errors
2. **Clear Warnings:** Unknown model users get helpful guidance
3. **Documentation Updated:** Error messages and logs explain dimension configuration

### Edge Cases Covered
1. **Provider Changes:** If user switches from OpenAI to Ollama without clearing dimension
2. **Model Changes:** If user switches from mxbai to nomic without updating dimension
3. **Mixed Config:** If user sets model but not dimension (should infer)
4. **Custom Endpoint:** Custom Ollama endpoints work with inference

## Assumptions

1. **Model Names Stable:** "nomic-embed-text" and "mxbai-embed-large" remain standard names
2. **Dimension Stability:** Model dimensions won't change (768/1024 are fixed)
3. **Two Models Only:** Only two Ollama models are widely used and need inference
4. **Ollama-Specific:** Other providers (OpenAI, Cohere) don't need dimension inference
