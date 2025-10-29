# Ticket: MCP-002: Complete Google Vertex AI provider integration in config system

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Complete Google Vertex AI provider integration by updating all match statements in the config-based embedding system to handle the `Provider::Google` variant. The factory-based system already has complete Google support from MPEMBED tickets, but the legacy config system needs updates.

## Background
The maproom codebase has two parallel provider systems:

1. **Factory System** (New) - `src/embedding/factory.rs`
   - ✅ Complete Google Vertex AI support via MPEMBED-3001
   - Uses `create_provider_from_env()`
   - Works with environment variables

2. **Config System** (Legacy) - `src/embedding/config.rs`
   - ❌ Incomplete Google support
   - Uses `Provider` enum
   - Missing `Provider::Google` handling in match statements

**Current State:**
- `Provider::Google` variant added to enum (commit 330f638)
- Factory fully supports Google
- Config system has 4 match statements that need Google handling

**Error when using scan command:**
```
error[E0004]: non-exhaustive patterns: `Provider::Google` not covered
```

## Acceptance Criteria
- [x] All match statements handle `Provider::Google` variant
- [x] Google provider works via config system (not just factory)
- [x] Compilation succeeds with no enum exhaustiveness errors
- [x] Tests verify Google provider can be selected via config
- [x] Documentation updated to reflect dual provider system support

## Technical Requirements

### 1. Update `src/embedding/client.rs:195` - Request Building

**Location:** Line 195 in `embed()` method

**Current Code:**
```rust
let request = match self.config.provider {
    Provider::OpenAI => { /* ... */ },
    Provider::Cohere => { /* ... */ },
    Provider::Ollama => { /* ... */ },
    Provider::Local => { /* ... */ },
    // Provider::Google missing
};
```

**Required:**
```rust
let request = match self.config.provider {
    Provider::OpenAI => { /* existing code */ },
    Provider::Cohere => { /* existing code */ },
    Provider::Ollama => { /* existing code */ },
    Provider::Google => {
        // Google uses factory system, not this client
        // Return error directing user to use EMBEDDING_PROVIDER env var
        return Err(EmbeddingError::Config(ConfigError::InvalidValue {
            field: "provider".to_string(),
            reason: "Google provider requires using EMBEDDING_PROVIDER=google environment variable. \
                     The legacy OpenAIClient does not support Google Vertex AI. \
                     Use create_provider_from_env() for Google support.".to_string(),
        }));
    },
    Provider::Local => { /* existing code */ },
};
```

### 2. Update `src/embedding/client.rs:326` - Provider Name

**Location:** Line 326 in `provider_name()` method

**Current Code:**
```rust
let provider_name = match self.config.provider {
    Provider::OpenAI => "openai",
    Provider::Cohere => "cohere",
    Provider::Ollama => "ollama",
    Provider::Local => "local",
    // Provider::Google missing
};
```

**Required:**
```rust
let provider_name = match self.config.provider {
    Provider::OpenAI => "openai",
    Provider::Cohere => "cohere",
    Provider::Ollama => "ollama",
    Provider::Google => "google",
    Provider::Local => "local",
};
```

### 3. Update `src/embedding/config.rs:158` - API Key Loading

**Location:** Line 158 in `from_env()` method

**Current Code:**
```rust
config.api_key = match config.provider {
    Provider::OpenAI => env::var("OPENAI_API_KEY").ok(),
    Provider::Cohere => env::var("COHERE_API_KEY").ok(),
    Provider::Ollama => None,
    Provider::Local => None,
    // Provider::Google missing
};
```

**Required:**
```rust
config.api_key = match config.provider {
    Provider::OpenAI => env::var("OPENAI_API_KEY").ok(),
    Provider::Cohere => env::var("COHERE_API_KEY").ok(),
    Provider::Ollama => None,
    Provider::Google => None, // Google uses service account JSON, not API key
    Provider::Local => None,
};
```

### 4. Update `src/embedding/config.rs:251` - Endpoint URL

**Location:** Line 251 in `api_endpoint()` method

**Current Code:**
```rust
match self.provider {
    Provider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
    Provider::Cohere => "https://api.cohere.ai/v1/embed".to_string(),
    Provider::Ollama => "http://localhost:11434/api/embed".to_string(),
    Provider::Local => "http://localhost:8080/embeddings".to_string(),
    // Provider::Google missing
}
```

**Required:**
```rust
match self.provider {
    Provider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
    Provider::Cohere => "https://api.cohere.ai/v1/embed".to_string(),
    Provider::Ollama => "http://localhost:11434/api/embed".to_string(),
    Provider::Google => {
        // Google endpoint is region-specific and constructed by GoogleProvider
        // Default to us-central1 for compatibility
        let region = env::var("GOOGLE_REGION").unwrap_or_else(|_| "us-central1".to_string());
        let project = env::var("GOOGLE_PROJECT_ID").unwrap_or_else(|_| "unknown".to_string());
        format!("https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/textembedding-gecko@003:predict",
                region, project, region)
    },
    Provider::Local => "http://localhost:8080/embeddings".to_string(),
}
```

## Implementation Notes

### Architecture Decision

The config-based `OpenAIClient` is a legacy system designed for OpenAI-compatible APIs. Google Vertex AI uses a fundamentally different authentication model (service account JSON vs API keys), making it incompatible with the `OpenAIClient` architecture.

**Recommendation:**
- Add `Provider::Google` handling to prevent compilation errors
- Direct users to the factory system for Google support
- Consider deprecating the config-based system in favor of the factory

### Alternative Approach

Instead of making `OpenAIClient` support Google, modify the scan command to use the factory system:

**File:** `src/main.rs` or scan command handler

**Change:**
```rust
// OLD: Using config system
let config = EmbeddingConfig::from_env()?;
let client = OpenAIClient::new(config)?;

// NEW: Using factory system
let provider = create_provider_from_env().await?;
let cache = EmbeddingCache::new(cache_config)?;
let service = EmbeddingService::new(provider, Arc::new(cache));
```

This approach:
- ✅ Leverages complete Google implementation from MPEMBED
- ✅ Simpler than duplicating Google logic in `OpenAIClient`
- ✅ Unifies provider handling
- ❌ Requires refactoring scan command

## Testing Requirements

1. **Unit Tests** - Add tests in `src/embedding/config.rs`:
   ```rust
   #[test]
   fn test_provider_google_from_str() {
       let provider: Provider = "google".parse().unwrap();
       assert_eq!(provider, Provider::Google);
   }
   ```

2. **Integration Tests** - Verify Google can be selected:
   ```rust
   #[test]
   fn test_config_with_google_provider() {
       env::set_var("EMBEDDING_PROVIDER", "google");
       let config = EmbeddingConfig::from_env().unwrap();
       assert_eq!(config.provider, Provider::Google);
   }
   ```

3. **Compilation Test** - Ensure all match arms compile:
   ```bash
   cargo build --release --bin crewchief-maproom
   ```

## Dependencies
- None (standalone fix)

## Risk Assessment
- **Risk**: Users may try to use Google via `OpenAIClient` which won't work
  - **Mitigation**: Clear error message directing to factory system
- **Risk**: Dual provider systems cause confusion
  - **Mitigation**: Document recommended approach (use factory)

## Files/Packages Affected
- `crates/maproom/src/embedding/client.rs` - Add Google handling (2 locations)
- `crates/maproom/src/embedding/config.rs` - Add Google handling (2 locations)
- `crates/maproom/tests/config_test.rs` - Add Google provider tests (if exists)

## Related Issues
- MPEMBED-3001: Google provider implementation (completed via factory)
- MCP-001: Default DATABASE_URL for zero-config (completed)
- Commit 330f638: Added `Provider::Google` enum variant
