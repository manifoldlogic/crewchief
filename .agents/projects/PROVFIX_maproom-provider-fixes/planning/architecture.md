# Architecture: Clean Provider Configuration

## Design Principles

1. **Provider Logic in Rust**: All provider-specific configuration belongs in Rust codebase
2. **CLI is Orchestration**: CLI passes generic environment, Rust resolves specifics
3. **Explicit Precedence**: Clear hierarchy for configuration resolution
4. **Fail Fast**: Validate configuration early with clear error messages
5. **No Duplication**: Single source of truth for provider endpoints

## Solution Architecture

### 1. Fix Rust Endpoint Resolution

#### Current (Buggy) Implementation

```rust
// config.rs:167
config.api_endpoint = env::var("EMBEDDING_API_ENDPOINT").ok();

// config.rs:248-256
pub fn api_endpoint_url(&self) -> String {
    if let Some(endpoint) = &self.api_endpoint {
        endpoint.clone()  // BUG: Always uses env var if set
    } else {
        match self.provider {
            Provider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
            // ...
        }
    }
}
```

#### Proposed (Fixed) Implementation

**Option A: Provider-Specific Override Semantics**

```rust
pub fn api_endpoint_url(&self) -> String {
    // Only use EMBEDDING_API_ENDPOINT for Ollama (local override)
    // Cloud providers should explicitly set if needed, not inherit
    match self.provider {
        Provider::OpenAI => {
            self.api_endpoint.clone()
                .unwrap_or_else(|| "https://api.openai.com/v1/embeddings".to_string())
        }
        Provider::Google => {
            // Google uses region-specific endpoint, not EMBEDDING_API_ENDPOINT
            let region = env::var("GOOGLE_VERTEX_REGION")
                .unwrap_or_else(|_| "us-west1".to_string());
            format!("https://{}-aiplatform.googleapis.com/v1", region)
        }
        Provider::Cohere => {
            self.api_endpoint.clone()
                .unwrap_or_else(|| "https://api.cohere.ai/v1/embed".to_string())
        }
        Provider::Ollama => {
            // Ollama allows local override via EMBEDDING_API_ENDPOINT
            self.api_endpoint.clone()
                .unwrap_or_else(|| "http://localhost:11434/api/embed".to_string())
        }
        Provider::Local => {
            self.api_endpoint.clone()
                .ok_or_else(|| EmbeddingError::Config(ConfigError::MissingConfig(
                    "Local provider requires EMBEDDING_API_ENDPOINT".to_string()
                )))
                .expect("Local provider must have endpoint")
        }
    }
}
```

**Option B: Clear Environment Variable Semantics (RECOMMENDED)**

```rust
// config.rs - Add new method
impl EmbeddingConfig {
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let mut config = Self::default();

        // Load provider first
        if let Ok(provider) = env::var("EMBEDDING_PROVIDER") {
            config.provider = provider.parse()?;
        }

        // Load model and dimension
        if let Ok(model) = env::var("EMBEDDING_MODEL") {
            config.model = model;
        }
        if let Ok(dim) = env::var("EMBEDDING_DIMENSION") {
            config.dimension = dim.parse().map_err(|_| ConfigError::InvalidValue {
                field: "EMBEDDING_DIMENSION".to_string(),
                reason: "Must be a positive integer".to_string(),
            })?;
        }

        // Load API endpoint ONLY for providers that support custom endpoints
        // This is the key fix: make endpoint override provider-aware
        match config.provider {
            Provider::Ollama | Provider::Local => {
                // These providers need/allow custom endpoints
                config.api_endpoint = env::var("EMBEDDING_API_ENDPOINT").ok();
            }
            Provider::OpenAI | Provider::Cohere => {
                // Cloud providers: only use endpoint if explicitly different
                // Prevents Docker Compose defaults from leaking
                if let Ok(endpoint) = env::var("EMBEDDING_API_ENDPOINT") {
                    // Only use if it's actually for this provider
                    if config.provider == Provider::OpenAI
                        && endpoint.contains("openai.com") {
                        config.api_endpoint = Some(endpoint);
                    } else if config.provider == Provider::Cohere
                        && endpoint.contains("cohere") {
                        config.api_endpoint = Some(endpoint);
                    }
                    // Otherwise ignore - wrong provider's endpoint
                }
            }
            Provider::Google => {
                // Google doesn't use EMBEDDING_API_ENDPOINT at all
                // Uses region-based construction
            }
        }

        // Rest of config loading...
        Ok(config)
    }

    pub fn api_endpoint_url(&self) -> String {
        // Now api_endpoint is already provider-validated
        if let Some(endpoint) = &self.api_endpoint {
            return endpoint.clone();
        }

        // Provider-specific defaults
        match self.provider {
            Provider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
            Provider::Cohere => "https://api.cohere.ai/v1/embed".to_string(),
            Provider::Ollama => "http://localhost:11434/api/embed".to_string(),
            Provider::Google => {
                let region = env::var("GOOGLE_VERTEX_REGION")
                    .unwrap_or_else(|_| "us-west1".to_string());
                format!("https://{}-aiplatform.googleapis.com/v1", region)
            }
            Provider::Local => {
                // Local must have endpoint set
                panic!("Local provider requires EMBEDDING_API_ENDPOINT")
            }
        }
    }
}
```

**Recommendation**: Option B is cleaner because it validates endpoint during config loading rather than at use time.

### 2. Remove CLI Workarounds

#### Current (Workaround) Code

```javascript
// bin/cli.cjs - THREE places with this workaround
if (provider === 'openai') {
  providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
  providerEnv.EMBEDDING_DIMENSION = '1536';
  providerEnv.EMBEDDING_API_ENDPOINT = 'https://api.openai.com/v1/embeddings';  // REMOVE
}
```

#### Proposed (Clean) Code

```javascript
// bin/cli.cjs - After Rust fix
if (provider === 'openai') {
  providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
  providerEnv.EMBEDDING_DIMENSION = '1536';
  // NO endpoint override needed - Rust handles it
}
```

The CLI should ONLY set:
- `EMBEDDING_PROVIDER` (required)
- `EMBEDDING_MODEL` (provider-specific default)
- `EMBEDDING_DIMENSION` (provider-specific default)
- Provider-specific API keys (OPENAI_API_KEY, etc.)

### 3. Fix Database Schema

#### Issue

Missing `updated_at` column in `maproom.chunks` table.

#### Solution

Add migration to add the column:

```sql
-- migration: 00XX_add_updated_at_to_chunks.sql
ALTER TABLE maproom.chunks
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();

-- Add trigger to auto-update
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_chunks_updated_at
    BEFORE UPDATE ON maproom.chunks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

#### Migration Path

1. Check if migration already exists in `/workspace/crates/maproom/migrations/`
2. If not, create new migration file with next number
3. Update schema initialization to run all migrations
4. Test with fresh database and existing database

### 4. Clean Environment Variable Contract

#### Clear Precedence Rules

```
For Cloud Providers (OpenAI, Cohere, Google):
1. Provider default endpoint (hardcoded in Rust)
2. EMBEDDING_API_ENDPOINT (only if matches provider domain)

For Local Providers (Ollama, Local):
1. EMBEDDING_API_ENDPOINT (if set)
2. Provider default (Ollama only)
3. Error if not set (Local provider)
```

#### Docker Compose Changes

Update `docker-compose.yml` to not set default `EMBEDDING_API_ENDPOINT`:

```yaml
# OLD (causes bug)
environment:
  EMBEDDING_API_ENDPOINT: ${EMBEDDING_API_ENDPOINT:-http://ollama:11434}

# NEW (only set when needed)
environment:
  EMBEDDING_API_ENDPOINT: ${EMBEDDING_API_ENDPOINT:-}  # Empty default
```

Or better, remove entirely and let Rust defaults handle it.

## Migration Strategy

### Phase 1: Fix Rust Core (Critical Path)

1. Fix `EmbeddingConfig::from_env()` with provider-aware endpoint loading
2. Update `api_endpoint_url()` to use validated endpoint
3. Add unit tests for each provider's endpoint resolution
4. Add integration test with mixed environment variables

### Phase 2: Database Schema

1. Create migration for `updated_at` column
2. Test migration on fresh database
3. Test migration on existing database with data
4. Verify embedding updates succeed

### Phase 3: Remove CLI Workarounds

1. Remove explicit endpoint setting from `runScan()`
2. Remove explicit endpoint setting from `runSetup()`
3. Remove explicit endpoint setting from `upsertFiles()`
4. Remove now-unnecessary deletion logic
5. Simplify environment building code

### Phase 4: Clean Up & Test

1. Update Docker Compose to not set default endpoint
2. Test full flow: setup → scan → embeddings with OpenAI
3. Test full flow with Google Vertex AI
4. Test full flow with Ollama
5. Test error cases (missing API keys, wrong endpoints)

## Success Criteria

✅ OpenAI embeddings work without CLI workaround
✅ Google Vertex AI embeddings work (untested previously)
✅ Ollama still works with default and custom endpoints
✅ Database updates succeed with `updated_at` column
✅ No duplicate endpoint logic between CLI and Rust
✅ Clear error messages for misconfiguration
✅ All integration tests pass

## Non-Goals

- Changing provider architecture (keep current design)
- Adding new providers (focus on fixing existing)
- Optimizing performance (already working)
- Major refactoring beyond the bug fixes

## Risks

**Low Risk**:
- Changes are localized to config loading
- Existing tests will catch regressions
- Workaround provides rollback path

**Medium Risk**:
- Database migration on existing production data
- Mitigation: Test migration path thoroughly

**Avoided Risk**:
- Temptation to over-engineer provider abstraction
- Keep fixes minimal and focused
