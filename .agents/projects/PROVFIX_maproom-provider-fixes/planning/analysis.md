# Analysis: Maproom Provider Configuration Bugs

## Problem Statement

During implementation of the provider selection feature for Maproom MCP, several critical bugs and architectural issues were discovered that prevent clean multi-provider support. The current implementation requires brittle workarounds in the CLI layer that mask underlying bugs in the Rust codebase.

## Issues Discovered

### 1. Critical: Rust Endpoint Resolution Bug

**Location**: `/workspace/crates/maproom/src/embedding/config.rs:248-256`

**Issue**: The `api_endpoint_url()` method incorrectly prioritizes the `EMBEDDING_API_ENDPOINT` environment variable over provider-specific defaults.

```rust
pub fn api_endpoint_url(&self) -> String {
    if let Some(endpoint) = &self.api_endpoint {
        endpoint.clone()  // BUG: Uses EMBEDDING_API_ENDPOINT for ALL providers
    } else {
        match self.provider {
            Provider::OpenAI => "https://api.openai.com/v1/embeddings".to_string(),
            Provider::Cohere => "https://api.cohere.ai/v1/embed".to_string(),
            Provider::Ollama => "http://localhost:11434/api/embed".to_string(),
            // ...
        }
    }
}
```

**Root Cause**:
- Line 167 loads `EMBEDDING_API_ENDPOINT` from environment unconditionally
- Docker Compose defaults set `EMBEDDING_API_ENDPOINT=http://ollama:11434`
- When OpenAI provider is configured, it inherits the Ollama endpoint from the environment

**Impact**:
- OpenAI provider attempts to connect to `http://localhost:11434/api/embed` instead of OpenAI API
- All API calls fail with "Connection refused" errors
- User experience is completely broken for cloud providers

**Evidence**:
```
[ERROR] Failed to generate code embeddings: Network(reqwest::Error {
  kind: Request,
  url: "http://localhost:11434/api/embed",
  source: ConnectError("Connection refused")
})
```

Despite environment showing:
```
EMBEDDING_PROVIDER: openai
EMBEDDING_MODEL: text-embedding-3-small
EMBEDDING_DIMENSION: 1536
EMBEDDING_API_ENDPOINT: (not set)  // Correctly unset by CLI
```

### 2. Workaround Applied (Must Be Removed)

**Location**: `/workspace/packages/maproom-mcp/bin/cli.cjs:1495-1507`

**Current Workaround**:
```javascript
if (provider === 'openai') {
  providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
  providerEnv.EMBEDDING_DIMENSION = '1536';
  // WORKAROUND: Explicitly set OpenAI endpoint due to Rust bug
  providerEnv.EMBEDDING_API_ENDPOINT = 'https://api.openai.com/v1/embeddings';
}
```

**Why This Is Brittle**:
- Duplicates endpoint logic that should live in Rust
- Requires CLI to know provider-specific URLs
- Violates separation of concerns
- Must be maintained in parallel with Rust code
- Same workaround duplicated in 3 places (runScan, runSetup, upsertFiles)

### 3. Database Schema Issue

**Location**: Embedding update queries

**Issue**: Missing `updated_at` column in `maproom.chunks` table

**Evidence**:
```
ERROR: column "updated_at" of relation "chunks" does not exist
Failed to update embeddings for chunk 451
```

**Impact**:
- Embeddings generate successfully via API
- Database update fails, preventing embeddings from persisting
- Silent data loss - chunks appear indexed but lack embeddings
- 854/854 chunks failed to update in test run

**Root Cause**: Migration mismatch between schema definition and query expectations

### 4. Incomplete Environment Variable Handling

**Location**: `/workspace/packages/maproom-mcp/bin/cli.cjs:1528-1531`

**Original Attempt** (removed in workaround):
```javascript
// For cloud providers, ensure EMBEDDING_API_ENDPOINT is not set
if (provider !== 'ollama' && env.EMBEDDING_API_ENDPOINT) {
  delete env.EMBEDDING_API_ENDPOINT;
}
```

**Issue**: This deletion was fighting with the workaround, showing architectural confusion about where endpoint logic belongs.

## Current State

### What Works
- ✅ Setup command with provider selection
- ✅ Provider validation (API key checks)
- ✅ OpenAI embeddings generate via API (with workaround)
- ✅ Scan and watch commands invoke correctly

### What's Broken
- ❌ Rust endpoint resolution logic
- ❌ Database schema for embedding updates
- ❌ Clean separation between CLI and provider logic
- ❌ Google Vertex AI provider (untested, likely same bug)

### Technical Debt Created
- 3 duplicate workaround code blocks in CLI
- Environment variable handling confusion
- Missing integration tests for multi-provider scenarios

## Industry Context

### How Other Tools Handle This

**LangChain Python**:
```python
embeddings = OpenAIEmbeddings()  # Endpoint is internal
# OR
embeddings = OpenAIEmbeddings(openai_api_base="https://custom")  # Explicit override
```

**Semantic Kernel (C#)**:
```csharp
var embeddings = new OpenAITextEmbeddingService(
    modelId: "text-embedding-3-small",
    endpoint: new Uri("https://api.openai.com")  // Explicit, typed
);
```

**Best Practices**:
1. Provider-specific endpoints are hardcoded defaults in provider implementation
2. Environment variable overrides are opt-in, not default behavior
3. Clear precedence: explicit parameter > env var > provider default
4. Configuration validates early, fails fast

## Root Cause Analysis

The bug exists because:

1. **Mixed Responsibilities**: `EmbeddingConfig` tries to be both:
   - Generic configuration container
   - Provider-specific configuration logic

2. **Environment Loading Too Early**: Line 167 loads `EMBEDDING_API_ENDPOINT` before provider is fully determined

3. **No Provider-Specific Validation**: Config doesn't validate endpoint matches provider

4. **Docker Compose Leaks Defaults**: Container defaults pollute environment namespace

## Lessons Learned

1. **Test with actual cloud providers early** - Ollama testing masked the endpoint bug
2. **Environment variable precedence must be explicit** - Current design is ambiguous
3. **Workarounds indicate architectural issues** - The CLI workaround was a signal
4. **Integration tests needed** - Unit tests didn't catch cross-layer bugs
