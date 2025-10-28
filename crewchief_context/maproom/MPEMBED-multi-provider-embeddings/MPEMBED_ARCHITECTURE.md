# MPEMBED: Multi-Provider Embedding Support - Architecture

## Design Philosophy

**Guiding principles:**
1. **Zero-config default**: Ollama works out of the box, no setup required
2. **Explicit provider choice**: Users opt-in to cloud providers via config
3. **Dimension transparency**: Users never think about vector dimensions
4. **Additive, not destructive**: Preserve existing OpenAI embeddings, add capabilities
5. **MVP pragmatism**: Support 3 providers well, not 10 providers poorly

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    MCP TypeScript Layer                      │
│  - Provider detection (Ollama available? Config set?)       │
│  - Pass --provider flag to Rust binary                      │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                   Rust Embedding Service                     │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │          EmbeddingProvider Trait (NEW)             │    │
│  │  - async fn embed(&self, text: &str) -> Vector    │    │
│  │  - async fn embed_batch(&self, texts) -> Vectors  │    │
│  │  - fn dimension(&self) -> usize                    │    │
│  │  - fn provider_name(&self) -> &str                 │    │
│  └────────────────────────────────────────────────────┘    │
│                          │                                   │
│          ┌───────────────┼───────────────┐                  │
│          ▼               ▼               ▼                  │
│    ┌─────────┐   ┌─────────────┐   ┌──────────┐           │
│    │ Ollama  │   │   Google    │   │  OpenAI  │           │
│    │Provider │   │ VertexAI    │   │ Provider │           │
│    │ (768)   │   │ Provider    │   │  (1536)  │           │
│    └─────────┘   │   (768)     │   └──────────┘           │
│                   └─────────────┘                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│               PostgreSQL Database (pgvector)                 │
│                                                              │
│  maproom.chunks table:                                       │
│    code_embedding_ollama  vector(768)  -- Ollama + Google   │
│    text_embedding_ollama  vector(768)  -- Ollama + Google   │
│    code_embedding         vector(1536) -- OpenAI            │
│    text_embedding         vector(1536) -- OpenAI            │
│                                                              │
│  Search query pattern:                                       │
│    COALESCE(code_embedding_ollama, code_embedding) <=> $1   │
└─────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Database Schema Changes

**Migration 0015: Add 768-dimensional columns**

```sql
-- Add Ollama/Google columns (768 dimensions)
ALTER TABLE maproom.chunks
  ADD COLUMN code_embedding_ollama vector(768),
  ADD COLUMN text_embedding_ollama vector(768);

-- Create IVFFlat indexes for 768-dim vectors
-- lists = 200 is optimal for ~25K chunks (sqrt(N) heuristic)
CREATE INDEX idx_chunks_code_vec_ollama
  ON maproom.chunks
  USING ivfflat (code_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);

CREATE INDEX idx_chunks_text_vec_ollama
  ON maproom.chunks
  USING ivfflat (text_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);

-- Note: Existing 1536-dim columns remain unchanged
-- code_embedding vector(1536)
-- text_embedding vector(1536)
-- idx_chunks_code_vec (ivfflat, 1536-dim)
-- idx_chunks_text_vec (ivfflat, 1536-dim)
```

**Column selection logic:**
- **Ollama provider**: Insert into `*_ollama` columns (768)
- **Google provider**: Insert into `*_ollama` columns (768)
- **OpenAI provider**: Insert into original columns (1536)

**Storage efficiency:**
- Most users will use ONE provider → only one column set populated
- Advanced users can use BOTH (e.g., Ollama dev, OpenAI prod) → both populated
- NULL columns have minimal storage overhead (~1 byte per NULL in PostgreSQL)

### 2. Rust Provider Abstraction

**New trait: `EmbeddingProvider`**

```rust
// crates/maproom/src/embedding/provider.rs (NEW FILE)

use async_trait::async_trait;
use crate::embedding::cache::Vector;
use crate::embedding::error::EmbeddingError;

/// Abstract embedding provider interface.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text.
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError>;

    /// Generate embeddings for a batch of texts.
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError>;

    /// Get the embedding dimension for this provider.
    fn dimension(&self) -> usize;

    /// Get the provider name ("ollama", "google", "openai").
    fn provider_name(&self) -> &'static str;

    /// Get provider-specific metrics (optional).
    fn metrics(&self) -> Option<ProviderMetrics> {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProviderMetrics {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub failed_requests: u64,
    pub estimated_cost_usd: f64,
}
```

**Implementation: OllamaProvider**

```rust
// crates/maproom/src/embedding/ollama.rs (NEW FILE)

pub struct OllamaProvider {
    client: reqwest::Client,
    endpoint: String,
    model: String,
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        let response = self.client
            .post(&self.endpoint)
            .json(&OllamaRequest {
                model: self.model.clone(),
                input: text,
            })
            .send()
            .await?;

        let body: OllamaResponse = response.json().await?;
        Ok(body.embeddings[0].clone())
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        // Ollama doesn't support native batching
        // Sequential requests with tokio::spawn for concurrency
        let mut tasks = Vec::new();
        for text in texts {
            let provider = self.clone();
            tasks.push(tokio::spawn(async move {
                provider.embed(text).await
            }));
        }

        let results = futures::future::join_all(tasks).await;
        results.into_iter()
            .map(|r| r.map_err(|e| EmbeddingError::Other(e.to_string()))?)
            .collect()
    }

    fn dimension(&self) -> usize {
        768 // nomic-embed-text fixed dimension
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }
}
```

**Implementation: GoogleProvider**

```rust
// crates/maproom/src/embedding/google.rs (NEW FILE)

use google_cloud_auth::Credentials;
use google_cloud_googleapis::cloud::aiplatform::v1::PredictRequest;

pub struct GoogleProvider {
    credentials: Credentials,
    project_id: String,
    location: String,
    model: String,
    task_type: String, // "RETRIEVAL_DOCUMENT" or "RETRIEVAL_QUERY"
}

#[async_trait]
impl EmbeddingProvider for GoogleProvider {
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        let endpoint = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predict",
            self.location, self.project_id, self.location, self.model
        );

        let token = self.credentials.access_token().await?;

        let response = reqwest::Client::new()
            .post(&endpoint)
            .bearer_auth(token)
            .json(&GoogleRequest {
                instances: vec![GoogleInstance {
                    content: text,
                    task_type: self.task_type.clone(),
                }],
            })
            .send()
            .await?;

        let body: GoogleResponse = response.json().await?;
        Ok(body.predictions[0].embeddings.values.clone())
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        // Google Vertex AI supports batching via multiple instances
        let endpoint = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predict",
            self.location, self.project_id, self.location, self.model
        );

        let token = self.credentials.access_token().await?;

        let instances: Vec<GoogleInstance> = texts.into_iter()
            .map(|text| GoogleInstance {
                content: text,
                task_type: self.task_type.clone(),
            })
            .collect();

        let response = reqwest::Client::new()
            .post(&endpoint)
            .bearer_auth(token)
            .json(&GoogleRequest { instances })
            .send()
            .await?;

        let body: GoogleResponse = response.json().await?;
        Ok(body.predictions.into_iter()
            .map(|p| p.embeddings.values)
            .collect())
    }

    fn dimension(&self) -> usize {
        768 // text-embedding-gecko@003 fixed dimension
    }

    fn provider_name(&self) -> &'static str {
        "google"
    }
}
```

**Implementation: OpenAIProvider**

```rust
// Refactor existing OpenAIClient to implement EmbeddingProvider trait
// crates/maproom/src/embedding/openai.rs (MODIFIED)

#[async_trait]
impl EmbeddingProvider for OpenAIClient {
    async fn embed(&self, text: String) -> Result<Vector, EmbeddingError> {
        self.embed_text(text).await
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
        self.embed_batch(texts).await
    }

    fn dimension(&self) -> usize {
        1536 // text-embedding-3-small fixed dimension
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn metrics(&self) -> Option<ProviderMetrics> {
        Some(ProviderMetrics {
            total_requests: self.metrics.total_requests(),
            total_tokens: self.metrics.total_tokens(),
            failed_requests: self.metrics.failed_requests(),
            estimated_cost_usd: self.metrics.estimated_cost_usd(),
        })
    }
}
```

### 3. Provider Factory

**Provider construction from environment:**

```rust
// crates/maproom/src/embedding/factory.rs (NEW FILE)

pub fn create_provider_from_env() -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
    let provider = env::var("EMBEDDING_PROVIDER")
        .unwrap_or_else(|_| "ollama".to_string())
        .to_lowercase();

    match provider.as_str() {
        "ollama" => {
            let endpoint = env::var("EMBEDDING_API_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:11434/api/embed".to_string());
            let model = env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "nomic-embed-text".to_string());

            Ok(Box::new(OllamaProvider::new(endpoint, model)?))
        }
        "google" => {
            let project_id = env::var("GOOGLE_PROJECT_ID")
                .map_err(|_| EmbeddingError::Configuration("GOOGLE_PROJECT_ID required".into()))?;
            let location = env::var("GOOGLE_LOCATION")
                .unwrap_or_else(|_| "us-central1".to_string());
            let model = env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-gecko@003".to_string());
            let task_type = env::var("EMBEDDING_TASK_TYPE")
                .unwrap_or_else(|_| "RETRIEVAL_DOCUMENT".to_string());

            Ok(Box::new(GoogleProvider::new(project_id, location, model, task_type).await?))
        }
        "openai" => {
            let config = EmbeddingConfig::from_env()?;
            let client = OpenAIClient::new(config)?;
            Ok(Box::new(client))
        }
        _ => Err(EmbeddingError::Configuration(
            format!("Unknown provider: {}", provider)
        ))
    }
}
```

### 4. EmbeddingService Refactor

**Update EmbeddingService to use provider abstraction:**

```rust
// crates/maproom/src/embedding/service.rs (MODIFIED)

pub struct EmbeddingService {
    provider: Box<dyn EmbeddingProvider>,
    cache: Arc<EmbeddingCache>,
}

impl EmbeddingService {
    pub fn new(provider: Box<dyn EmbeddingProvider>, cache: Arc<EmbeddingCache>) -> Self {
        Self { provider, cache }
    }

    pub fn from_env() -> Result<Self, EmbeddingError> {
        let provider = create_provider_from_env()?;
        let cache_config = CacheConfig::from_env()?;
        let cache = EmbeddingCache::new(cache_config)?;
        Ok(Self::new(provider, Arc::new(cache)))
    }

    pub fn dimension(&self) -> usize {
        self.provider.dimension()
    }

    pub fn provider_name(&self) -> &str {
        self.provider.provider_name()
    }

    // embed_text, embed_batch methods now delegate to self.provider
}
```

### 5. Database Insertion Logic

**Column selection based on dimension:**

```rust
// crates/maproom/src/db/chunks.rs (MODIFIED)

pub async fn upsert_embeddings(
    pool: &PgPool,
    chunk_id: i64,
    code_embedding: Option<Vector>,
    text_embedding: Option<Vector>,
    dimension: usize,
) -> Result<(), DbError> {
    let (code_col, text_col) = match dimension {
        768 => ("code_embedding_ollama", "text_embedding_ollama"),
        1536 => ("code_embedding", "text_embedding"),
        _ => return Err(DbError::InvalidDimension(dimension)),
    };

    let query = format!(
        "UPDATE maproom.chunks SET {} = $1, {} = $2 WHERE id = $3",
        code_col, text_col
    );

    sqlx::query(&query)
        .bind(code_embedding)
        .bind(text_embedding)
        .bind(chunk_id)
        .execute(pool)
        .await?;

    Ok(())
}
```

### 6. Search Query Updates

**Hybrid search with COALESCE pattern:**

```rust
// crates/maproom/src/search/hybrid.rs (MODIFIED)

pub async fn hybrid_search(
    pool: &PgPool,
    query: &str,
    vector: Vector,
    dimension: usize, // Pass from provider
    limit: usize,
) -> Result<Vec<SearchResult>, SearchError> {
    let (code_col, text_col) = match dimension {
        768 => ("code_embedding_ollama", "text_embedding_ollama"),
        1536 => ("code_embedding", "text_embedding"),
        _ => return Err(SearchError::InvalidDimension(dimension)),
    };

    // Alternatively, use COALESCE to search both columns:
    // COALESCE(code_embedding_ollama, code_embedding) <=> $1

    let query_sql = format!(
        r#"
        WITH fts_results AS (
            SELECT id, ts_rank(ts_doc, plainto_tsquery($1)) as fts_score
            FROM maproom.chunks
            WHERE ts_doc @@ plainto_tsquery($1)
        ),
        vector_results AS (
            SELECT id, 1 - ({} <=> $2) as vec_score
            FROM maproom.chunks
            WHERE {} IS NOT NULL
            ORDER BY {} <=> $2
            LIMIT $3
        )
        SELECT
            c.id,
            c.symbol_name,
            c.preview,
            COALESCE(fts.fts_score, 0) * 0.3 + COALESCE(vec.vec_score, 0) * 0.7 as hybrid_score
        FROM maproom.chunks c
        LEFT JOIN fts_results fts ON c.id = fts.id
        LEFT JOIN vector_results vec ON c.id = vec.id
        WHERE fts.id IS NOT NULL OR vec.id IS NOT NULL
        ORDER BY hybrid_score DESC
        LIMIT $3
        "#,
        code_col, code_col, code_col
    );

    sqlx::query_as(&query_sql)
        .bind(query)
        .bind(vector)
        .bind(limit as i32)
        .fetch_all(pool)
        .await
}
```

**Fallback pattern for mixed embeddings:**

```sql
-- If user has BOTH 768-dim and 1536-dim embeddings, prefer 768-dim
-- This prioritizes faster/cheaper provider (Ollama/Google) over OpenAI
SELECT
  id,
  symbol_name,
  COALESCE(
    1 - (code_embedding_ollama <=> $1),  -- Try 768-dim first
    1 - (code_embedding <=> $2)           -- Fallback to 1536-dim
  ) as similarity
FROM maproom.chunks
WHERE code_embedding_ollama IS NOT NULL OR code_embedding IS NOT NULL
ORDER BY similarity DESC
LIMIT 10;
```

### 7. MCP TypeScript Integration

**Provider detection and flag passing:**

```typescript
// packages/maproom-mcp/src/tools/scan.ts (MODIFIED)

export async function scanTool(params: ScanParams): Promise<ScanResult> {
  // Detect provider (prefer Ollama if available, fall back to config)
  const provider = await detectProvider();

  const args = [
    'scan',
    '--path', params.path || process.cwd(),
    '--repo', params.repo,
    '--worktree', params.worktree,
    '--commit', params.commit || 'HEAD',
    '--generate-embeddings=true',
  ];

  if (provider) {
    args.push('--provider', provider);
  }

  // Add batch size if configured
  const batchSize = process.env.EMBEDDING_BATCH_SIZE;
  if (batchSize) {
    args.push('--embedding-batch-size', batchSize);
  }

  const result = await spawnMaproomBinary(args, {
    timeout: 600000, // 10 minutes
  });

  return parseStdout(result.stdout);
}

async function detectProvider(): Promise<string | null> {
  // Check explicit config first
  const configProvider = process.env.EMBEDDING_PROVIDER;
  if (configProvider) {
    return configProvider.toLowerCase();
  }

  // Auto-detect Ollama on localhost
  try {
    const response = await fetch('http://localhost:11434/api/tags', {
      method: 'GET',
      signal: AbortSignal.timeout(2000), // 2-second timeout
    });
    if (response.ok) {
      return 'ollama';
    }
  } catch (error) {
    // Ollama not available, will use configured provider or error
  }

  return null;
}
```

### 8. Configuration Schema

**Environment variables:**

```bash
# Provider selection (default: ollama, auto-detected)
EMBEDDING_PROVIDER=ollama  # or: google, openai

# === Ollama Configuration ===
EMBEDDING_MODEL=nomic-embed-text
EMBEDDING_API_ENDPOINT=http://localhost:11434/api/embed

# === Google Vertex AI Configuration ===
# EMBEDDING_PROVIDER=google
# GOOGLE_PROJECT_ID=my-project-123
# GOOGLE_LOCATION=us-central1
# GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
# EMBEDDING_MODEL=text-embedding-gecko@003
# EMBEDDING_TASK_TYPE=RETRIEVAL_DOCUMENT  # or RETRIEVAL_QUERY for search

# === OpenAI Configuration ===
# EMBEDDING_PROVIDER=openai
# OPENAI_API_KEY=sk-...
# EMBEDDING_MODEL=text-embedding-3-small

# === Shared Configuration ===
EMBEDDING_BATCH_SIZE=50
EMBEDDING_MAX_CONCURRENCY=4
```

## Migration Strategy

### Phase 1: Database Schema (Non-Breaking)

1. Run migration 0015 to add 768-dim columns
2. Existing 1536-dim columns remain unchanged
3. Existing OpenAI embeddings continue working
4. No downtime required

### Phase 2: Rust Provider Abstraction

1. Add `EmbeddingProvider` trait
2. Implement OllamaProvider, GoogleProvider
3. Refactor OpenAIClient to implement trait
4. Update EmbeddingService to use trait
5. Add provider factory
6. **Backward compatible**: Default to OpenAI if no provider specified

### Phase 3: Column Routing Logic

1. Update `upsert_embeddings` to select columns by dimension
2. Update search queries to use COALESCE pattern
3. Test with mixed embeddings (768 + 1536 simultaneously)

### Phase 4: MCP Integration

1. Add provider detection in TypeScript wrapper
2. Pass `--provider` flag to Rust binary
3. Update README with configuration examples

### Phase 5: Documentation

1. Provider comparison table (speed, cost, privacy)
2. Setup guides for each provider (API keys, auth, endpoints)
3. Migration guide for existing OpenAI users
4. Troubleshooting section

## Trade-offs and Decisions

### Decision 1: Trait-based dispatch vs enum

**Chosen: Trait-based dispatch (`Box<dyn EmbeddingProvider>`)**

Pros:
- Extensible: Add new providers without touching core service
- Clean separation: Each provider is self-contained module
- Testable: Easy to mock providers for testing

Cons:
- Dynamic dispatch overhead (negligible for I/O-bound embedding calls)
- Slightly more complex than enum match

**Rejected: Enum with match statements**

Would require modifying service.rs every time a provider is added.

### Decision 2: Column sharing vs separate columns

**Chosen: Column sharing (Ollama + Google → same 768-dim columns)**

Pros:
- Reduces storage overhead (2 column sets instead of 3)
- Enables seamless switching between Ollama ↔ Google
- Simplified query logic (fewer COALESCE branches)

Cons:
- Cannot have both Ollama AND Google embeddings simultaneously
- If providers diverge on dimension, would need new columns

**Why it's acceptable**: Users won't need both Ollama and Google simultaneously—they serve different use cases (local vs cloud).

### Decision 3: Auto-detect Ollama vs explicit config

**Chosen: Auto-detect Ollama, fall back to explicit config**

Pros:
- Best zero-config experience (Ollama "just works" if installed)
- Users can override with `EMBEDDING_PROVIDER` if needed

Cons:
- Adds network call to localhost:11434 at startup
- Might surprise users if Ollama is running but they want OpenAI

**Mitigation**: 2-second timeout, clear logs indicating provider selection.

### Decision 4: COALESCE preference order

**Chosen: Prefer 768-dim over 1536-dim**

```sql
COALESCE(code_embedding_ollama, code_embedding)
```

Pros:
- Prioritizes local/cheaper providers (Ollama/Google)
- Encourages migration away from OpenAI if both exist

Cons:
- If OpenAI embeddings are "better quality", users might not get them

**Why it's acceptable**: Users with strong OpenAI preference can avoid generating 768-dim embeddings.

## Performance Considerations

**Storage overhead:**
- 768-dim: 3KB per chunk (768 floats × 4 bytes)
- 1536-dim: 6KB per chunk
- 25K chunks: ~75MB (768) or ~150MB (1536)
- With both: ~225MB (acceptable on modern systems)

**Index build time:**
- IVFFlat index with lists=200: ~30-60 seconds for 25K chunks
- One-time cost at migration

**Query performance:**
- COALESCE adds minimal overhead (<5% query time)
- PostgreSQL query planner optimizes NULL checks efficiently
- IVFFlat approximate nearest neighbor: O(log N) lookups

**Embedding generation:**
- Ollama: ~4.5 chunks/s = 1,000 chunks in ~3-4 minutes
- Google: ~50-100 chunks/s = 25,000 chunks in ~5-10 minutes
- OpenAI: ~50-200 chunks/s = 25,000 chunks in ~3-5 minutes

## Testing Strategy

See QUALITY_STRATEGY.md for comprehensive testing approach. Key architecturally-relevant tests:

1. **Provider interface tests**: Each provider correctly implements trait
2. **Dimension routing tests**: Embeddings go to correct columns
3. **Search fallback tests**: COALESCE pattern works with mixed embeddings
4. **Migration tests**: Existing OpenAI embeddings survive migration
5. **Performance benchmarks**: Query latency with COALESCE vs direct column

## Next Steps

See PLAN.md for phased implementation roadmap.
