# MPEMBED: Multi-Provider Embedding Support - Analysis

## Problem Space

### The Core Issue

Maproom's semantic search currently has a **dimension mismatch crisis** blocking all embedding generation:

- **Database schema**: Hardcoded `vector(1536)` columns (OpenAI text-embedding-3-small dimensions)
- **Current provider**: Ollama with nomic-embed-text model generates `vector(768)` embeddings
- **Result**: 23,632 indexed chunks, **0 embeddings generated** (all attempts silently fail with dimension mismatch)
- **Impact**: Semantic search completely non-functional; only full-text search (FTS) works

This isn't just a configuration issue—it's an **architectural constraint** that prevents users from choosing their embedding provider based on their priorities:

1. **Privacy-conscious users** want local embeddings (Ollama) but can't use them
2. **Cost-sensitive users** want free options (Ollama) but are forced toward paid APIs
3. **Enterprise users** on Google Cloud want native integration (Vertex AI) but have no path forward
4. **Performance-focused users** want fast cloud embeddings (OpenAI, Google) but sacrifice flexibility

### Why This Matters

Embedding provider choice is **not a luxury—it's a necessity** for different deployment contexts:

**Local/Offline Environments:**
- No internet access or air-gapped systems
- Privacy regulations (GDPR, HIPAA) prohibit sending code to external APIs
- Cost constraints (startups, open-source projects, personal use)
- **Current state**: Completely blocked from using semantic search

**Enterprise Cloud Environments:**
- Existing Google Cloud commitments with negotiated rates
- Compliance requirements for data residency (EU, Asia-Pacific regions)
- Need for SLA guarantees and enterprise support contracts
- **Current state**: Forced to use OpenAI or no semantic search

**Hybrid Requirements:**
- Development locally (Ollama), production on cloud (Google/OpenAI)
- Different teams with different budgets/policies
- Migration paths between providers as requirements change
- **Current state**: Lock-in to single provider, no migration path

### Current Industry Solutions

#### 1. **Pinecone** - Cloud-native vector database
**Approach**: Provider-agnostic storage, separate embedding generation
- Accepts any dimension vectors (configurable per index)
- Users generate embeddings client-side with their chosen provider
- Indexes optimized per dimension (768, 1536, etc.)
- **Trade-off**: Fully managed SaaS ($70+/month), no self-hosting

#### 2. **Weaviate** - Open-source vector database
**Approach**: Module-based provider plugins
- Built-in modules: OpenAI, Cohere, HuggingFace, Google, custom
- Each module creates separate vector properties with correct dimensions
- Search queries specify which module/property to use
- **Trade-off**: Heavy infrastructure (Go backend, gRPC), complex setup

#### 3. **Qdrant** - High-performance vector database
**Approach**: Flexible schema with multiple vector fields per record
- Same record can have multiple vectors (e.g., `openai_vec`, `cohere_vec`)
- Each vector field has independent dimension configuration
- Payload-based filtering to select active vector field
- **Trade-off**: Rust-based (good), but separate service deployment required

#### 4. **LlamaIndex/LangChain** - LLM orchestration frameworks
**Approach**: Abstraction layer over embedding providers
- Unified interface (`embed_documents()`, `embed_query()`) for all providers
- Provider-specific clients (OpenAI, HuggingFace, Ollama, etc.)
- Dimension handling delegated to storage layer
- **Trade-off**: Framework overhead, not a database solution

### What Makes Maproom Different

Maproom is **not a general-purpose vector database**—it's a **semantic code search engine** with specific constraints:

1. **Integrated system**: Parser → Indexer → Database → Search (single binary)
2. **Zero-config promise**: `npx -y @crewchief/maproom-mcp` should "just work"
3. **PostgreSQL foundation**: Leverages existing enterprise database infrastructure
4. **Code-aware indexing**: Tree-sitter parsing, symbol extraction, relationship graphs
5. **Hybrid search**: FTS + vector + graph signals combined for code search

This means we **cannot** adopt Pinecone/Weaviate's approach of "generate embeddings client-side"—we need **built-in embedding generation** that:
- Works out of the box (zero config)
- Supports multiple providers without breaking changes
- Handles dimension differences transparently
- Maintains backward compatibility with existing installations

### The Dimension Problem

**Why do different models have different dimensions?**

Embedding dimensions are **architecture decisions** made during model training:

- **768 dimensions**: Standard BERT-derived models (Google BERT, Sentence-BERT, nomic-embed-text)
  - Balances expressiveness vs computational cost
  - Faster inference, smaller storage (768 floats × 4 bytes = 3KB per embedding)
  - Sufficient for most semantic similarity tasks

- **1536 dimensions**: OpenAI's proprietary architecture (text-embedding-3-small, text-embedding-ada-002)
  - Higher dimensionality for nuanced semantic differences
  - Slower inference, larger storage (1536 floats × 4 bytes = 6KB per embedding)
  - Optimized for OpenAI's specific training objectives

**Can we convert between dimensions?** No, not without retraining:
- Dimensions are learned representations, not arbitrary choices
- Truncating 1536→768 loses critical information
- Padding 768→1536 with zeros doesn't add semantic content
- The only solution: **store both dimension types separately**

### Architectural Constraints

Maproom's current architecture has several hard constraints that shape our solution:

1. **PostgreSQL pgvector**:
   - Column dimension **must** be declared at schema creation: `vector(N)`
   - Inserting wrong dimension raises runtime error: "expected 1536 dimensions, not 768"
   - No dynamic dimension columns in PostgreSQL
   - Solution: Multiple columns with different dimensions

2. **IVFFlat indexes**:
   - Vector indexes are dimension-specific
   - Each vector column needs its own index
   - Index `lists` parameter tuned based on corpus size (~sqrt(N))
   - Solution: Multiple indexes, one per vector column

3. **Rust embedding service**:
   - Currently hardcoded to OpenAI client (`OpenAIClient` struct)
   - Config assumes OpenAI-specific fields (api_key, model, endpoint)
   - Ollama API is HTTP POST with different JSON schema
   - Google Vertex AI uses gRPC with service account auth
   - Solution: Provider abstraction with trait-based dispatch

4. **MCP TypeScript wrapper**:
   - Calls Rust binary with `--generate-embeddings` flag
   - No awareness of which provider or dimension
   - Needs to query database for available embeddings before search
   - Solution: Provider detection and column selection logic

5. **Search queries**:
   - Currently assumes single `code_embedding` column
   - Hybrid search combines FTS + vector similarity
   - Need to handle cases where only one dimension type has embeddings
   - Solution: COALESCE pattern to use available embeddings

### Existing Implementation State

**What works:**
- ✅ OpenAI embedding generation (when dimension matches)
- ✅ Ollama embedding generation (when dimension matches)
- ✅ Caching layer (SQLite-based, dimension-agnostic)
- ✅ Batch processing with concurrency control
- ✅ Cost tracking and metrics
- ✅ Retry logic and error handling
- ✅ Auto-embedding trigger after scan/upsert (LOCAL-5005)

**What's broken:**
- ❌ Database schema locked to 1536 dimensions
- ❌ No column routing based on provider/dimension
- ❌ Search queries assume single embedding column
- ❌ No provider abstraction (only OpenAI client exists, despite Ollama support in config)
- ❌ No Google Vertex AI integration

**What's misleading:**
- EmbeddingConfig has `provider` field ("openai" | "ollama") but it's only used for endpoint selection
- OpenAIClient handles both OpenAI and Ollama, but name suggests OpenAI-only
- Ollama "support" exists but is blocked by dimension mismatch at database layer

### Key Insights from Recent Work

From LOCAL-5005 (auto-embedding generation):
1. **Auto-embedding works**: The `--generate-embeddings` flag successfully triggers incremental embedding generation
2. **MCP integration works**: TypeScript wrapper correctly passes flag to Rust binary
3. **Timeout is appropriate**: 10-minute timeout allows for reasonable batch sizes (~2,700 chunks)
4. **Dimension mismatch is the blocker**: All embedding attempts fail silently at database insertion

From performance testing (23,632 chunks):
- Ollama (local): ~87.5 minutes, $0, 100% private
- Google Vertex AI (cloud): ~5-10 minutes, ~$0.10-0.20, cloud-processed
- OpenAI (cloud): ~5 minutes, ~$0.19, cloud-processed
- **Speed vs cost vs privacy triangle**: Users need to choose based on their priority

### User Personas and Requirements

**Persona 1: Privacy-First Developer (Sarah)**
- Works on proprietary codebases with strict NDAs
- Cannot send code to external APIs (legal restriction)
- Willing to wait 90 minutes for embeddings
- **Requirement**: Ollama (local) must work out of the box

**Persona 2: Enterprise Platform Team (DevOps team)**
- Manages Google Cloud infrastructure for 50+ developers
- Has GCP commitment with negotiated rates
- Needs SLA guarantees and enterprise support
- **Requirement**: Google Vertex AI integration, no OpenAI dependency

**Persona 3: Open Source Maintainer (Alex)**
- Indexes public repositories for documentation search
- Budget: $0 (hobby project)
- Can wait for embeddings, runs overnight
- **Requirement**: Free option (Ollama) with no API keys

**Persona 4: Startup Engineering Team**
- Fast iteration, values developer velocity
- Has budget for API costs ($50-200/month)
- Wants fastest possible embedding generation
- **Requirement**: OpenAI or Google for speed, easy setup

### Success Criteria

The multi-provider implementation succeeds if:

1. **Zero-config experience preserved**:
   - `npx -y @crewchief/maproom-mcp` works immediately
   - Ollama auto-detected on localhost:11434, falls back gracefully
   - No required environment variables for basic functionality

2. **Provider choice supported**:
   - Users can set `EMBEDDING_PROVIDER=ollama|google|openai`
   - Each provider has documented setup (API keys, auth, endpoints)
   - Switching providers doesn't require re-indexing files (only embeddings)

3. **Dimension handling transparent**:
   - 768-dim and 1536-dim embeddings coexist in database
   - Search queries automatically use available embeddings
   - No user-facing dimension configuration (inferred from provider)

4. **Backward compatibility maintained**:
   - Existing OpenAI embeddings (1536-dim) continue working
   - Adding Ollama support doesn't break existing installations
   - Migration is additive (add columns), not destructive

5. **Performance acceptable**:
   - Ollama: <2 minutes for 1,000 chunks (local CPU)
   - Google: <1 minute for 10,000 chunks (cloud GPUs)
   - OpenAI: <1 minute for 10,000 chunks (cloud GPUs)

### Research Findings

**Ollama nomic-embed-text:**
- Embedding dimension: 768 (fixed)
- Context length: 8,192 tokens (sufficient for code chunks)
- License: Apache 2.0 (fully open)
- Model size: 274MB (reasonable for local deployment)
- Inference speed: ~4.5 chunks/s on M-series Mac CPU

**Google Vertex AI text-embedding-gecko@003:**
- Embedding dimension: 768 (matches Ollama!)
- Task types: RETRIEVAL_DOCUMENT (for indexing), RETRIEVAL_QUERY (for search)
- Context length: 3,072 tokens
- Cost: $0.00001 per 1,000 characters (~$0.10-0.20 per 100K chunks)
- Authentication: Service account JSON key
- Regions: us-central1, europe-west1, asia-southeast1
- Inference speed: ~50-100 chunks/s (estimated)

**OpenAI text-embedding-3-small:**
- Embedding dimension: 1536 (default) or configurable (256-3072)
- Context length: 8,191 tokens
- Cost: $0.02 per 1M tokens (~$0.19 per 100K chunks)
- Authentication: API key (simple)
- Inference speed: ~50-200 chunks/s (batch API)

**Key finding**: Ollama and Google share 768 dimensions, enabling **column sharing** in database schema. This reduces storage overhead and simplifies migration.

### Risks and Mitigations

**Risk 1: Complex migration for existing users**
- Users with OpenAI embeddings need to preserve them
- Adding new columns shouldn't break existing queries
- **Mitigation**: Additive migration, COALESCE pattern in queries

**Risk 2: Performance degradation from COALESCE queries**
- Multiple column checks add query overhead
- Index selection might be suboptimal
- **Mitigation**: PostgreSQL query planner handles COALESCE efficiently, benchmark before/after

**Risk 3: Google Cloud authentication complexity**
- Service account JSON keys are enterprise-level setup
- Not "zero config" like Ollama
- **Mitigation**: Clear documentation, optional provider (default is Ollama)

**Risk 4: Embedding quality differences between providers**
- Different models → different semantic representations
- Search results may vary based on provider
- **Mitigation**: Document expected behavior, consider future eval harness

**Risk 5: Storage overhead (two embedding columns)**
- 768-dim + 1536-dim = 2× storage per chunk
- 23K chunks: ~70MB (768) + ~140MB (1536) = ~210MB total
- **Mitigation**: Users choose one provider, other column stays NULL

### Conclusion

This is a **tractable, well-scoped project** with clear boundaries:

- **Interface stability**: PostgreSQL schema, embedding provider APIs (OpenAI, Ollama, Google)
- **Context coherence**: Single architectural domain (embedding generation), ~12 core concepts
- **Testable completion**: Embeddings generate for all providers, search returns results, dimension handling works

The solution requires:
1. Database migration (add 768-dim columns)
2. Provider abstraction in Rust (trait-based dispatch)
3. Google Vertex AI client implementation
4. Search query updates (COALESCE pattern)
5. Configuration and documentation

Next: ARCHITECTURE.md will design the technical solution to these requirements.
