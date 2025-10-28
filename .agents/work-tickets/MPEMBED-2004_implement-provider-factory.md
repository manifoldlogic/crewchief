# Ticket: MPEMBED-2004: Implement provider factory with auto-detection and configuration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- provider-abstraction-architect
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create factory pattern for constructing providers from environment variables, with auto-detection of Ollama and explicit config for cloud providers.

## Background
Users shouldn't specify provider manually for a zero-config experience. The factory should auto-detect Ollama if running on localhost:11434, and fall back to explicit config via EMBEDDING_PROVIDER env var.

Configuration validation happens before returning provider to fail fast on misconfiguration.

This ticket implements the factory pattern as part of Phase 2: Provider Abstraction from the MPEMBED multi-provider embedding support plan.

## Acceptance Criteria
- [ ] `create_provider_from_env()` function auto-detects Ollama availability
- [ ] Ollama auto-detection: HTTP GET to `http://localhost:11434/api/tags` with 2s timeout
- [ ] If Ollama unavailable, check `EMBEDDING_PROVIDER` env var
- [ ] Supported provider values: "ollama", "google", "openai"
- [ ] Returns `Box<dyn EmbeddingProvider>` for dynamic dispatch
- [ ] Validates required env vars (e.g., GOOGLE_PROJECT_ID for Google)
- [ ] Returns helpful error messages on misconfiguration
- [ ] Logs selected provider (INFO level): "Using provider: ollama"

## Technical Requirements
- File location: `crates/maproom/src/embedding/factory.rs` (NEW FILE)
- Use `reqwest` for Ollama auto-detection HTTP call
- Timeout for auto-detection: 2 seconds (don't block startup)
- Environment variables checked:
  - `EMBEDDING_PROVIDER` (optional, default: auto-detect)
  - `EMBEDDING_MODEL` (optional, provider-specific defaults)
  - `EMBEDDING_API_ENDPOINT` (optional, provider-specific defaults)
  - Provider-specific vars (GOOGLE_PROJECT_ID, OPENAI_API_KEY, etc.)
- Use tracing for logging

## Implementation Notes

```rust
// crates/maproom/src/embedding/factory.rs (NEW FILE)

use std::env;
use crate::embedding::provider::EmbeddingProvider;
use crate::embedding::error::EmbeddingError;
use crate::embedding::ollama::OllamaProvider;
use crate::embedding::openai::OpenAIClient;

/// Create embedding provider from environment configuration.
///
/// # Auto-detection
/// If EMBEDDING_PROVIDER is not set, attempts to detect Ollama on localhost:11434.
///
/// # Environment Variables
/// - `EMBEDDING_PROVIDER`: Provider name (ollama, openai, google)
/// - `EMBEDDING_MODEL`: Model name (provider-specific defaults)
/// - `EMBEDDING_API_ENDPOINT`: API endpoint (provider-specific defaults)
/// - Provider-specific: OPENAI_API_KEY, GOOGLE_PROJECT_ID, etc.
pub async fn create_provider_from_env() -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    // Check explicit config first
    let explicit_provider = env::var("EMBEDDING_PROVIDER").ok();

    let provider = match explicit_provider.as_deref() {
        Some(p) => p.to_lowercase(),
        None => {
            // Auto-detect Ollama
            if is_ollama_available().await {
                tracing::info!("Ollama detected at localhost:11434");
                "ollama".to_string()
            } else {
                return Err(EmbeddingError::Configuration(
                    "No embedding provider configured. Set EMBEDDING_PROVIDER or install Ollama.".into()
                ));
            }
        }
    };

    match provider.as_str() {
        "ollama" => {
            let endpoint = env::var("EMBEDDING_API_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:11434/api/embed".to_string());
            let model = env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "nomic-embed-text".to_string());

            tracing::info!("Using provider: ollama (model: {})", model);
            Ok(Box::new(OllamaProvider::new(endpoint, model)?))
        }
        "openai" => {
            let config = EmbeddingConfig::from_env()?;
            let client = OpenAIClient::new(config)?;
            tracing::info!("Using provider: openai");
            Ok(Box::new(client))
        }
        _ => Err(EmbeddingError::Configuration(
            format!("Unknown provider: {}. Supported: ollama, openai", provider)
        ))
    }
}

/// Check if Ollama is available on localhost.
async fn is_ollama_available() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}
```

**Key Design Decisions**:
- Explicit config via EMBEDDING_PROVIDER bypasses auto-detection
- Auto-detection only checks Ollama (fastest local option)
- Timeout ensures startup isn't blocked by network issues
- Helpful error messages guide users on configuration

## Dependencies
- MPEMBED-2001 (trait definition)
- MPEMBED-2002 (Ollama provider)
- MPEMBED-2003 (OpenAI provider)

## Risk Assessment
- **Risk**: Auto-detection adds 2s startup delay if Ollama not running
  - **Mitigation**: Timeout set to 2s, explicit config bypasses auto-detection
- **Risk**: Auto-detection false positive if port 11434 is occupied by non-Ollama service
  - **Mitigation**: Check for `/api/tags` endpoint specific to Ollama

## Files/Packages Affected
- crates/maproom/src/embedding/factory.rs (create)
- crates/maproom/src/embedding/mod.rs (modify - add `pub mod factory;`)
