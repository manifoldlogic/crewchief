# Ticket: LOCAL-2003: Update EmbeddingConfig validation for Ollama

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the EmbeddingConfig validation logic in the Rust codebase to handle Ollama-specific requirements: no API key needed for local providers, 768-dimension vectors for nomic-embed-text model, and local endpoint validation with provider-specific defaults.

## Background
With the addition of Ollama as an embedding provider (LOCAL-2001), the existing validation logic in EmbeddingConfig needs to be updated to handle provider-specific requirements. Unlike cloud providers (OpenAI, Cohere), Ollama runs locally and doesn't require an API key. Additionally, the nomic-embed-text model produces 768-dimensional embeddings, which must be enforced at configuration time to prevent runtime errors.

The current validation logic assumes all providers require API keys and doesn't differentiate between cloud and local providers. This creates a poor user experience and prevents Ollama from being usable. This ticket addresses the validation layer to make Ollama a first-class provider.

Validation is critical - it prevents runtime errors from misconfiguration and provides clear feedback to users during setup.

## Acceptance Criteria
- [ ] `validate()` method updated to check provider type before requiring API key
- [ ] API key is NOT required for Ollama and Local providers
- [ ] API key IS required for OpenAI and Cohere providers
- [ ] nomic-embed-text model enforces exactly 768 dimensions (error if not 768)
- [ ] New `api_endpoint_url()` method returns correct default for each provider:
  - OpenAI: https://api.openai.com/v1/embeddings
  - Ollama: http://localhost:11434/api/embeddings
  - Local: http://localhost:8080/embeddings
- [ ] Custom endpoints can override defaults via `api_endpoint` config field
- [ ] Validation errors have clear, actionable messages (e.g., "Ollama provider with nomic-embed-text requires dimensions=768, got 512")
- [ ] All existing tests pass (no regressions)
- [ ] New unit tests for Ollama validation cases:
  - Test Ollama without API key (should pass)
  - Test Ollama with nomic-embed-text and dimensions=768 (should pass)
  - Test Ollama with nomic-embed-text and dimensions!=768 (should fail with clear error)
  - Test endpoint URL defaults for all providers
  - Test custom endpoint override

## Technical Requirements
- **File**: `crates/maproom/src/embedding/config.rs`
- **Validation Changes** (lines 829-837 in architecture doc):
  - Modify `validate()` method to switch on `self.provider`
  - For `EmbeddingProvider::OpenAI` and `EmbeddingProvider::Cohere`: require API key
  - For `EmbeddingProvider::Ollama` and `EmbeddingProvider::Local`: API key optional
- **Dimension Enforcement** (lines 841-849 in architecture doc):
  - Add dimension check for Ollama provider + nomic-embed-text model
  - Return `ConfigError` with descriptive message if dimension != 768
  - Preserve existing dimension validation for other providers
- **Endpoint URL Method** (lines 815-826 in architecture doc):
  - Add public method `api_endpoint_url() -> String`
  - Check if `self.api_endpoint` is set, return it if present
  - Otherwise return provider-specific default endpoint
  - Use match statement on `self.provider`
- **Error Handling**:
  - Use existing `ConfigError` enum from `crates/maproom/src/embedding/error.rs`
  - Add variant if needed: `InvalidDimensions { provider: String, model: String, expected: usize, got: usize }`
  - Ensure error messages guide users to correct configuration

## Implementation Notes

### API Key Validation Pattern
```rust
// In validate() method
match self.provider {
    EmbeddingProvider::OpenAI | EmbeddingProvider::Cohere => {
        if self.api_key.is_none() {
            return Err(ConfigError::MissingApiKey {
                provider: self.provider.to_string()
            });
        }
    },
    EmbeddingProvider::Ollama | EmbeddingProvider::Local => {
        // API key optional for local providers
    },
}
```

### Dimension Validation Pattern
```rust
// After API key validation in validate()
if self.provider == EmbeddingProvider::Ollama && self.model == "nomic-embed-text" {
    if self.dimensions != 768 {
        return Err(ConfigError::InvalidDimensions {
            provider: "Ollama".to_string(),
            model: "nomic-embed-text".to_string(),
            expected: 768,
            got: self.dimensions,
        });
    }
}
```

### Endpoint URL Method Pattern
```rust
pub fn api_endpoint_url(&self) -> String {
    if let Some(ref endpoint) = self.api_endpoint {
        return endpoint.clone();
    }

    match self.provider {
        EmbeddingProvider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
        EmbeddingProvider::Ollama => "http://localhost:11434/api/embeddings".to_string(),
        EmbeddingProvider::Local => "http://localhost:8080/embeddings".to_string(),
        EmbeddingProvider::Cohere => "https://api.cohere.ai/v1/embed".to_string(),
    }
}
```

### Testing Strategy
- Unit tests in `crates/maproom/src/embedding/config.rs` (in-module tests)
- Test both happy paths and error cases
- Use `assert_eq!` for success cases
- Use `assert!(matches!(err, ConfigError::...))` for error cases
- Cover all provider combinations

### References
- Ollama API documentation: https://github.com/ollama/ollama/blob/main/docs/api.md
- nomic-embed-text specifications: https://ollama.com/library/nomic-embed-text
- Rust error handling best practices: https://doc.rust-lang.org/book/ch09-00-error-handling.html

## Dependencies
- **LOCAL-2001**: Provider enum with Ollama (REQUIRED - must be completed first)
  - This ticket extends the EmbeddingProvider enum added in LOCAL-2001
  - Cannot proceed without the Ollama variant being available

## Risk Assessment
- **Risk**: Breaking existing OpenAI/Cohere validation
  - **Mitigation**: Comprehensive test coverage for all providers. Run full test suite before marking complete. Existing tests must pass without modification.

- **Risk**: Incorrect dimension enforcement breaks other Ollama models
  - **Mitigation**: Only enforce 768 dimensions for nomic-embed-text specifically. Other Ollama models use existing validation logic.

- **Risk**: Default endpoint URLs incorrect or change upstream
  - **Mitigation**: Document endpoints in code comments with links to official docs. Allow override via config. Test with actual Ollama installation in later integration tickets.

## Files/Packages Affected
- `crates/maproom/src/embedding/config.rs` - Main implementation file
  - `EmbeddingConfig::validate()` method
  - New `EmbeddingConfig::api_endpoint_url()` method
- `crates/maproom/src/embedding/error.rs` - May need new error variant
  - Potentially add `ConfigError::InvalidDimensions` if not already present
- Unit tests within `config.rs` module (new test functions)
