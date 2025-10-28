# Multi-Provider Embedding Support

## Overview

Support three embedding providers with smart database column sharing based on vector dimensions.

## Supported Providers

### 1. Ollama (Default - Zero Configuration)
- **Model**: nomic-embed-text
- **Dimensions**: 768
- **Speed**: ~4.5 chunks/s (local CPU)
- **Cost**: $0 (completely free)
- **Privacy**: 100% local, no data leaves machine
- **Use Case**: Default for privacy-conscious users, offline work, zero cost

### 2. Google Vertex AI (Enterprise Option)
- **Model**: text-embedding-gecko@003
- **Dimensions**: 768 (same as Ollama!)
- **Speed**: ~50-100 chunks/s (estimated, cloud GPUs)
- **Cost**: Pay-per-use (Google Cloud billing)
- **Privacy**: Data sent to Google Cloud
- **Use Case**: Enterprise customers with Google Cloud, multilingual support, faster than Ollama
- **Special Features**:
  - Task-specific optimization (RETRIEVAL_QUERY vs RETRIEVAL_DOCUMENT)
  - Multilingual support (15+ languages)
  - Configurable output dimensions

### 3. OpenAI (Alternative Cloud Option)
- **Model**: text-embedding-3-small
- **Dimensions**: 1536
- **Speed**: ~50-200 chunks/s (cloud GPUs, batching)
- **Cost**: $0.02 per 1M tokens (~$0.19 for 23k chunks)
- **Privacy**: Data sent to OpenAI
- **Use Case**: Fastest option, widely adopted, established track record

## Database Schema Design

### Efficient Column Sharing

Since Ollama and Google both use 768 dimensions, they **share the same columns**:

```sql
-- 768-dimensional embeddings (Ollama + Google Vertex AI)
code_embedding_ollama vector(768)
text_embedding_ollama vector(768)
idx_chunks_code_vec_ollama ivfflat (code_embedding_ollama vector_cosine_ops)
idx_chunks_text_vec_ollama ivfflat (text_embedding_ollama vector_cosine_ops)

-- 1536-dimensional embeddings (OpenAI)
code_embedding vector(1536)
text_embedding vector(1536)
idx_chunks_code_vec ivfflat (code_embedding vector_cosine_ops)
idx_chunks_text_vec ivfflat (text_embedding vector_cosine_ops)
```

### Storage Implications

- **Two sets of columns** support three providers
- Users can switch between Ollama ↔ Google seamlessly (same dimensions)
- Users can have both 768-dim (Ollama/Google) AND 1536-dim (OpenAI) embeddings simultaneously
- Search queries automatically use available embeddings via `COALESCE()`

## Configuration

### Environment Variables

```bash
# Provider selection (default: ollama)
EMBEDDING_PROVIDER=ollama   # or: google, openai

# Ollama configuration
EMBEDDING_MODEL=nomic-embed-text
EMBEDDING_DIMENSION=768
EMBEDDING_API_ENDPOINT=http://ollama:11434/api/embed

# Google Vertex AI configuration
# EMBEDDING_PROVIDER=google
# GOOGLE_PROJECT_ID=your-project-id
# GOOGLE_LOCATION=us-central1
# EMBEDDING_MODEL=text-embedding-gecko@003
# EMBEDDING_DIMENSION=768
# EMBEDDING_TASK_TYPE=RETRIEVAL_DOCUMENT  # or RETRIEVAL_QUERY

# OpenAI configuration
# EMBEDDING_PROVIDER=openai
# OPENAI_API_KEY=your-api-key
# EMBEDDING_MODEL=text-embedding-3-small
# EMBEDDING_DIMENSION=1536
```

## Search Query Logic

Automatically use available embeddings with preference order:

```sql
SELECT * FROM maproom.chunks
WHERE COALESCE(
  code_embedding_ollama,  -- Prefer 768-dim (Ollama/Google)
  code_embedding           -- Fallback to 1536-dim (OpenAI)
) <=> $1 < 0.3
ORDER BY COALESCE(
  code_embedding_ollama,
  code_embedding
) <=> $1
LIMIT 10;
```

## Migration Strategy

### For New Installations
- Default to Ollama (zero configuration)
- Create both column sets during initial migration
- Auto-populate based on configured provider

### For Existing Installations
- Add new 768-dim columns via migration
- Existing OpenAI embeddings (1536-dim) remain in original columns
- Users can switch to Ollama/Google or keep both

### Migration SQL

```sql
-- Add 768-dimensional columns for Ollama/Google
ALTER TABLE maproom.chunks
  ADD COLUMN code_embedding_ollama vector(768),
  ADD COLUMN text_embedding_ollama vector(768);

-- Add indexes for new columns
CREATE INDEX idx_chunks_code_vec_ollama
  ON maproom.chunks
  USING ivfflat (code_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);

CREATE INDEX idx_chunks_text_vec_ollama
  ON maproom.chunks
  USING ivfflat (text_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);
```

## Performance Comparison

For 23,632 chunks (typical medium-sized codebase):

| Provider | Time | Cost | Privacy | Speed Factor |
|----------|------|------|---------|--------------|
| Ollama | ~90 min | $0 | Local | 1x (baseline) |
| Google Vertex AI | ~5-10 min | ~$0.10-0.20 | Cloud | 10-20x faster |
| OpenAI | ~5 min | ~$0.19 | Cloud | 15-40x faster |

## Implementation Priority

1. **Phase 1**: Add 768-dim columns to database schema
2. **Phase 2**: Implement Google Vertex AI provider in Rust embedding service
3. **Phase 3**: Update search queries to use COALESCE() pattern
4. **Phase 4**: Add provider detection and column selection logic
5. **Phase 5**: Documentation and configuration examples

## Benefits

- ✅ **Flexibility**: Users choose based on privacy, cost, and speed needs
- ✅ **No lock-in**: Can switch providers or use multiple simultaneously
- ✅ **Efficient storage**: Smart column sharing (768-dim shared by 2 providers)
- ✅ **Zero-config default**: Ollama works out of the box
- ✅ **Enterprise ready**: Google Vertex AI for Google Cloud customers
- ✅ **Migration friendly**: Existing OpenAI users keep their embeddings

## Related Tickets

- LOCAL-5005: Auto-generate embeddings during scan/upsert
- LOCAL-4010: Optimize embedding generation throughput
- Current blocker: Database dimension mismatch (1536 vs 768)
