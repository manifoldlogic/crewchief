# Architecture: embedding dimension 1024

## Overview

Add 1024-dimensional embedding support to enable mxbai-embed-large model, which handles all content types without tokenization crashes. This follows the existing pattern established by 768-dim support (Migration #7) and maintains backward compatibility with existing embeddings.

**Key Principle**: Minimal, focused change. Add 1024 to existing dimension infrastructure, remove sanitization workaround, update validation.

## Design Decisions

### Decision 1: Follow Existing Virtual Table Pattern

**Context**: The codebase already supports multiple dimensions via separate virtual tables (vec_code, vec_code_768).

**Decision**: Add `vec_code_1024` virtual table following the exact pattern of vec_code_768.

**Rationale**:
- **Consistency**: Uses proven approach from Migration #7
- **sqlite-vec requirement**: Virtual tables have fixed dimensions
- **Simplicity**: No schema changes to existing tables
- **Safety**: Isolated from existing embeddings (no risk of dimension mismatch)

### Decision 2: Make Ollama Dimension Configurable

**Context**: Current validation hardcodes dimension=768 for nomic-embed-text, blocking other Ollama models.

**Decision**: Remove model-specific dimension validation, allow any valid dimension for Ollama provider.

**Rationale**:
- **Flexibility**: Different Ollama models have different dimensions (nomic-embed-text=768, mxbai-embed-large=1024)
- **User control**: Let users choose model/dimension combination via environment variables
- **Fail-safe**: Generic dimension validation still catches invalid dimensions
- **Future-proof**: Easy to add more Ollama models without code changes

### Decision 3: Remove Sanitization Workaround Entirely

**Context**: Lines 344-386 in ollama.rs replace characters to avoid nomic-embed-text crashes.

**Decision**: Delete the sanitization code when model=mxbai-embed-large (keep for nomic-embed-text backward compat).

**Rationale**:
- **Root cause fix**: mxbai-embed-large doesn't have tokenization bugs
- **Quality**: No content mangling = better embeddings
- **Simplicity**: Less code to maintain
- **Backward compat**: Keep sanitization for nomic-embed-text users (they may not switch immediately)

**Implementation**: Check model name before sanitizing:
```rust
if self.model == "nomic-embed-text" {
    // Apply sanitization workaround
} else {
    // Use raw text (mxbai-embed-large and other models)
}
```

### Decision 4: Extend Dimension Constants Consistently

**Context**: Dimension mapping exists in embeddings.rs, vector.rs, and columns.rs.

**Decision**: Add 1024 to all three locations, keeping them synchronized.

**Rationale**:
- **Correctness**: All modules must agree on supported dimensions
- **Error messages**: Consistent reporting across codebase
- **Future maintenance**: Clear pattern for adding new dimensions

**Note**: columns.rs is for PostgreSQL (not currently used in SQLite deployments), but we update it for consistency.

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Virtual table | `vec_code_1024` | Follows existing pattern, sqlite-vec requirement |
| Migration | Version 10 | Next sequential number after Migration #9 (context_cache) |
| Table name mapping | Hardcoded match statement | Simple, explicit, easy to understand |
| Configuration | Environment variables | Existing pattern (MAPROOM_EMBEDDING_MODEL, MAPROOM_EMBEDDING_DIMENSION) |
| Validation | Dimension-based only | Remove model-specific checks, allow flexibility |

## Component Design

### Component 1: Database Migration

**File**: `/workspace/crates/maproom/src/db/sqlite/migrations.rs`

**Responsibility**: Add vec_code_1024 virtual table to schema.

**Interface**:
```rust
Migration {
    version: 10,
    name: "add_vec_code_1024",
    up: "CREATE VIRTUAL TABLE vec_code_1024 USING vec0(embedding float[1024]);",
    down: "DROP TABLE IF EXISTS vec_code_1024;",
}
```

**Behavior**:
- Runs automatically on database initialization (MigrationRunner::migrate)
- Idempotent (safe to run multiple times)
- Backward compatible (existing tables untouched)
- Non-blocking (no data transformation required)

### Component 2: Dimension Mapping (embeddings.rs)

**File**: `/workspace/crates/maproom/src/db/sqlite/embeddings.rs`

**Responsibility**: Map dimension to correct vec_code table for storage.

**Interface**:
```rust
fn get_vec_table_name(dimension: usize) -> Result<&'static str> {
    match dimension {
        768 => Ok("vec_code_768"),
        1024 => Ok("vec_code_1024"),  // NEW
        1536 => Ok("vec_code"),
        _ => bail!("Unsupported embedding dimension: {}. Supported dimensions: {:?}", dimension, SUPPORTED_DIMENSIONS)
    }
}

const SUPPORTED_DIMENSIONS: &[usize] = &[768, 1024, 1536];  // UPDATED
```

**Changes**:
- Add 1024 case to match statement
- Update SUPPORTED_DIMENSIONS constant
- All existing logic reuses this function (no other changes needed)

### Component 3: Dimension Mapping (vector.rs)

**File**: `/workspace/crates/maproom/src/db/sqlite/vector.rs`

**Responsibility**: Map dimension to correct vec_code table for search.

**Interface**: Identical to embeddings.rs (duplicate function, same changes).

**Note**: This duplication is suboptimal but out of scope to refactor. We maintain consistency by applying identical changes.

### Component 4: Column Selection (columns.rs)

**File**: `/workspace/crates/maproom/src/db/columns.rs`

**Responsibility**: Map dimension to PostgreSQL column names (not used in SQLite, but updated for consistency).

**Interface**:
```rust
pub fn select_columns_for_dimension(dimension: usize) -> anyhow::Result<ColumnSet> {
    match dimension {
        768 => Ok(ColumnSet::OLLAMA),
        1024 => Ok(ColumnSet::MXBAI),  // NEW (add constant)
        1536 => Ok(ColumnSet::OPENAI),
        _ => Err(anyhow::anyhow!("Unsupported embedding dimension: {}. Supported dimensions: 768 (Ollama/Google), 1024 (mxbai-embed-large), 1536 (OpenAI)", dimension))
    }
}
```

**Note**: This may not be used in SQLite deployments (code comments suggest PostgreSQL), but we update it to keep codebase consistent.

### Component 5: Ollama Provider Configuration

**File**: `/workspace/crates/maproom/src/embedding/ollama.rs`

**Responsibility**: Make dimension configurable, remove hardcoded 768.

**Change 1 - Dimension method**: Return dimension from config instead of hardcoded 768:
```rust
fn dimension(&self) -> usize {
    self.config.dimension  // Read from config
}
```

**Change 2 - Constructor**: Accept dimension parameter:
```rust
pub fn new(endpoint: String, model: String, dimension: usize) -> Result<Self, EmbeddingError> {
    // Store dimension in struct
}
```

**Change 3 - Conditional sanitization**:
```rust
async fn embed_batch_raw(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
    let processed_texts = if self.model == "nomic-embed-text" {
        // Apply sanitization workaround for nomic-embed-text
        texts.into_iter().map(|t| sanitize_for_nomic(&t)).collect()
    } else {
        // Use raw text for mxbai-embed-large and other models
        texts
    };
    // ... rest of embedding logic
}
```

### Component 6: Config Validation

**File**: `/workspace/crates/maproom/src/embedding/config.rs`

**Responsibility**: Remove model-specific dimension validation.

**Current Code** (lines 266-276):
```rust
if self.provider == Provider::Ollama && self.model == "nomic-embed-text" {
    if self.dimension != 768 {
        return Err(ConfigError::InvalidValue { ... });
    }
}
```

**Change**: Make validation recommend but not enforce:
```rust
// Remove the hard error, replace with warning log
if self.provider == Provider::Ollama && self.model == "nomic-embed-text" && self.dimension != 768 {
    tracing::warn!("nomic-embed-text typically uses 768 dimensions, got {}", self.dimension);
}
if self.provider == Provider::Ollama && self.model == "mxbai-embed-large" && self.dimension != 1024 {
    tracing::warn!("mxbai-embed-large typically uses 1024 dimensions, got {}", self.dimension);
}
```

**Rationale**: Trust user configuration, provide helpful warnings instead of hard errors.

## Data Flow

### Embedding Generation Flow

1. **User configures**: `MAPROOM_EMBEDDING_MODEL=mxbai-embed-large`, `MAPROOM_EMBEDDING_DIMENSION=1024`
2. **Config validation**: Passes (1024 now supported)
3. **OllamaProvider**: Reads dimension from config, sends to Ollama API
4. **Ollama API**: Returns 1024-dim vector
5. **Upsert**: Stores in `code_embeddings` table with `embedding_dim=1024`
6. **Sync**: Calls `get_vec_table_name(1024)` → `vec_code_1024`, inserts into virtual table

### Vector Search Flow

1. **User queries**: Daemon receives search request
2. **Generate query embedding**: OllamaProvider embeds query → 1024-dim vector
3. **Table selection**: `get_vec_table_name(1024)` → `vec_code_1024`
4. **Vector search**: sqlite-vec searches vec_code_1024 table
5. **Results**: Returns chunk IDs with distances
6. **Context assembly**: Loads chunks from database

## Integration Points

### Existing Systems

1. **Migration system**: Migration #10 runs during `db migrate` or daemon startup
2. **EmbeddingService**: Uses updated dimension validation
3. **Search**: Automatically uses correct vec_code table based on query embedding dimension
4. **Indexer**: No changes (dimension-agnostic, stores whatever dimension provider returns)

### Configuration

**Environment variables**:
```bash
MAPROOM_EMBEDDING_PROVIDER=ollama
MAPROOM_EMBEDDING_MODEL=mxbai-embed-large
MAPROOM_EMBEDDING_DIMENSION=1024
```

**Precedence**: Explicit dimension (env var) overrides model default.

### Backward Compatibility

**Existing embeddings preserved**:
- 768-dim embeddings remain in vec_code_768
- 1536-dim embeddings remain in vec_code
- No data migration or conversion required

**Mixed-dimension repositories**:
- If user switches models mid-repository, old embeddings won't match new dimension
- Solution: Re-run `generate-embeddings` command to regenerate all embeddings with new dimension
- Detection: Search will only return results from embeddings matching query dimension

## Performance Considerations

### Storage Growth

- **768-dim**: 768 × 4 bytes = 3,072 bytes per embedding
- **1024-dim**: 1024 × 4 bytes = 4,096 bytes per embedding (+33%)
- **1536-dim**: 1536 × 4 bytes = 6,144 bytes per embedding

**Impact**: Marginal. For 100K embeddings: 410MB (1024-dim) vs 307MB (768-dim) vs 614MB (1536-dim).

### Query Performance

- **sqlite-vec**: Performance scales with dimension (more dims = more compute)
- **1024 vs 768**: ~33% more distance calculations per comparison
- **Mitigation**: sqlite-vec is highly optimized, difference likely negligible for typical query volumes

**Benchmark recommendation**: Test search latency with 1024-dim on realistic corpus size.

### Embedding Generation

- **mxbai-embed-large**: ~670MB model, slower than nomic-embed-text (~274MB)
- **Throughput**: 6,780 tokens/sec (from EMBPERF project) vs ~8,000+ for nomic-embed-text
- **Impact**: Acceptable. Embedding is one-time cost per chunk, not on critical path for search.

## Maintainability

### Adding Future Dimensions

Clear pattern established:
1. Add migration for `vec_code_<N>` table
2. Update `SUPPORTED_DIMENSIONS` constant (3 locations)
3. Add case to `get_vec_table_name()` match statements (2 locations)
4. Update `select_columns_for_dimension()` if needed (columns.rs)
5. Document configuration in `docs/providers/`

**Estimate**: ~30 minutes per new dimension.

### Testing Strategy

**Unit tests** (per dimension):
- Storage: `test_<N>_dim_embedding_storage`
- Sync: `test_<N>_dim_vector_table_sync`
- Search: `test_vector_search_with_<N>_dim`

**Integration tests**:
- Mixed dimensions: Verify 768, 1024, 1536 coexist
- Migration: Verify Migration #10 is idempotent

### Documentation

**User-facing**:
- `/workspace/docs/providers/ollama-setup.md`: Add mxbai-embed-large configuration example
- Include storage/performance tradeoffs

**Developer-facing**:
- `/workspace/crates/maproom/CLAUDE.md`: Update "Supported Dimensions" section
- Document dimension addition pattern

## Migration Path

### For Existing Users

**Scenario 1: Currently using nomic-embed-text (768-dim)**
1. Pull mxbai-embed-large: `ollama pull mxbai-embed-large`
2. Update config: `MAPROOM_EMBEDDING_MODEL=mxbai-embed-large`, `MAPROOM_EMBEDDING_DIMENSION=1024`
3. Restart daemon (applies Migration #10 automatically)
4. Re-generate embeddings: `crewchief-maproom generate-embeddings --repo <repo>`

**Scenario 2: Currently using OpenAI (1536-dim)**
- No action required (1536-dim continues working)

**Scenario 3: Fresh install**
- Migration #10 runs automatically, all three dimensions available immediately

### Rollback Plan

If issues arise:
1. Switch back to previous model: `MAPROOM_EMBEDDING_MODEL=nomic-embed-text`, `MAPROOM_EMBEDDING_DIMENSION=768`
2. Old embeddings in vec_code_768 still work
3. Migration #10 rollback: `DROP TABLE IF EXISTS vec_code_1024` (manual SQL, not automated)
