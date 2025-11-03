# Ticket: PROVFIX-1001: Fix Rust Endpoint Resolution Bug in EmbeddingConfig

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose (or rust-specialist if available)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix critical bug in `/workspace/crates/maproom/src/embedding/config.rs` where `EmbeddingConfig::from_env()` incorrectly loads `EMBEDDING_API_ENDPOINT` environment variable unconditionally, causing cloud providers (OpenAI, Cohere) to inherit Ollama's endpoint from Docker Compose defaults.

## Background
During implementation of provider selection for Maproom MCP, a critical bug was discovered:
- OpenAI provider configured correctly but attempts to connect to `http://localhost:11434/api/embed` (Ollama endpoint)
- Root cause: Line 167 in `config.rs` loads `EMBEDDING_API_ENDPOINT` without provider validation
- Docker Compose sets default `EMBEDDING_API_ENDPOINT=http://ollama:11434`
- All providers inherit this endpoint regardless of their actual provider type
- Results in "Connection refused" errors for cloud providers

This bug is detailed in `.agents/projects/PROVFIX_maproom-provider-fixes/planning/analysis.md` section "1. Critical: Rust Endpoint Resolution Bug".

The fix implements provider-aware endpoint loading with domain validation to ensure endpoints match the configured provider.

This is Phase 1, Ticket 1 of the PROVFIX implementation plan.

## Acceptance Criteria
- [x] `EmbeddingConfig::from_env()` validates endpoint domain matches provider
- [x] OpenAI provider ignores Ollama endpoints in environment
- [x] OpenAI provider accepts custom OpenAI endpoints (https://api.openai.com/*)
- [x] Ollama provider continues to accept custom endpoints
- [x] Google provider ignores `EMBEDDING_API_ENDPOINT` entirely (uses region-based URL)
- [x] Clear precedence: explicit parameter > validated env var > provider default

## Technical Requirements

### Core Changes to `EmbeddingConfig::from_env()`

1. **Load provider first** before any endpoint logic:
   ```rust
   // Parse provider before touching endpoint variables
   let config = EmbeddingConfig {
       provider: env::var("EMBEDDING_PROVIDER")
           .ok()
           .and_then(|s| s.parse().ok())
           .unwrap_or_default(),
       // ... other fields
   };
   ```

2. **Provider-specific endpoint validation**:
   ```rust
   // After provider is determined:
   match config.provider {
       Provider::OpenAI | Provider::Cohere => {
           if let Ok(endpoint) = env::var("EMBEDDING_API_ENDPOINT") {
               // Only use if domain matches provider
               if config.provider == Provider::OpenAI && endpoint.contains("openai.com") {
                   config.api_endpoint = Some(endpoint);
               } else if config.provider == Provider::Cohere && endpoint.contains("cohere") {
                   config.api_endpoint = Some(endpoint);
               }
               // Otherwise ignore - wrong provider's endpoint
           }
       }
       Provider::Ollama | Provider::Local => {
           // Accept any endpoint
           config.api_endpoint = env::var("EMBEDDING_API_ENDPOINT").ok();
       }
       Provider::Google => {
           // Google doesn't use EMBEDDING_API_ENDPOINT
           // Endpoint is constructed from region
       }
   }
   ```

3. **Domain validation rules**:
   - **OpenAI**: Must contain "openai.com"
   - **Cohere**: Must contain "cohere"
   - **Google**: Ignores `EMBEDDING_API_ENDPOINT` entirely
   - **Ollama/Local**: No validation (accepts any)

### Simplify `api_endpoint_url()` Method

Remove conditional logic that allows wrong-provider endpoints:
```rust
pub fn api_endpoint_url(&self) -> String {
    // Use validated api_endpoint from config, or provider default
    self.api_endpoint.clone().unwrap_or_else(|| {
        match self.provider {
            Provider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
            Provider::Cohere => "https://api.cohere.ai/v1/embed".to_string(),
            Provider::Ollama => "http://localhost:11434/api/embed".to_string(),
            Provider::Local => "http://localhost:8080/embed".to_string(),
            Provider::Google => {
                // Construct from region
                format!("https://{}-aiplatform.googleapis.com/v1", self.google_region)
            }
        }
    })
}
```

## Implementation Notes

### Recommended Approach
See `.agents/projects/PROVFIX_maproom-provider-fixes/planning/architecture.md` section "Solution Architecture" for detailed implementation (Option B).

Key principles:
- **Provider-first**: Always determine provider before loading endpoints
- **Domain validation**: Simple substring matching for cloud providers
- **Fail-safe**: Wrong-provider endpoints are silently ignored, falling back to defaults
- **Backward compatible**: Ollama and Local providers retain current flexibility

### Edge Cases to Handle
- Empty `EMBEDDING_API_ENDPOINT` variable → No-op, use provider default
- Invalid domain (e.g., `http://malicious.site` for OpenAI) → Ignore, use default
- Custom OpenAI-compatible endpoints (e.g., Azure) → Must contain "openai.com" in domain
- Google provider always constructs endpoint from region, ignoring environment variable

### Testing Strategy
Unit tests will be added in ticket PROVFIX-1002. This ticket focuses on implementation only.

Key test scenarios to support:
1. OpenAI provider with Ollama endpoint → uses OpenAI default
2. OpenAI provider with valid OpenAI endpoint → uses custom endpoint
3. Ollama provider with any endpoint → uses custom endpoint
4. Google provider with any endpoint → constructs from region (ignores env)

## Dependencies
- None (first ticket in PROVFIX project)
- This ticket must complete before PROVFIX-3001 (CLI cleanup)

## Risk Assessment
- **Risk**: Changes core configuration loading, could break existing setups
  - **Mitigation**: Unit tests verify all provider scenarios (PROVFIX-1002); CLI workaround provides rollback path if needed (PROVFIX-3001)

- **Risk**: Domain validation too strict, blocks legitimate custom endpoints
  - **Mitigation**: Keep validation simple (domain substring match); power users can still set custom endpoints with correct domain

- **Risk**: Breaking change for users with custom OpenAI-compatible endpoints (e.g., Azure OpenAI)
  - **Mitigation**: Document endpoint requirements; most Azure OpenAI endpoints include "openai" in domain; if needed, validation can be relaxed in future ticket

## Files/Packages Affected
- `/workspace/crates/maproom/src/embedding/config.rs` (primary changes)
  - Modify: `EmbeddingConfig::from_env()` method (lines 150-180 approx)
  - Simplify: `api_endpoint_url()` method (lines 186-200 approx)

## Planning References
- Analysis: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/analysis.md`
  - Section: "1. Critical: Rust Endpoint Resolution Bug"
- Architecture: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/architecture.md`
  - Section: "Solution Architecture" (Option B)
- Plan: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md`
  - Phase 1, Ticket 1
