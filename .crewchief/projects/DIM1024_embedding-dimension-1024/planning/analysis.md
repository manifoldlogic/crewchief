# Analysis: embedding dimension 1024

## Problem Definition

The Ollama nomic-embed-text model (768-dim) crashes when processing certain characters (|, [], (), Unicode symbols) due to GGML tokenization bugs that cause attention layer failures. These crashes prevent reliable code embedding, especially for markdown documentation and structured code with these common characters.

**Specific Issue**: The current workaround (lines 344-386 in `ollama.rs`) replaces problematic characters before embedding, which:
- Mangles code content (changes semantic meaning)
- Reduces embedding quality (corrupted input → degraded output)
- Is brittle (requires maintaining character replacement lists)
- Doesn't address root cause (tokenizer bugs in nomic-embed-text)

**Better Solution**: Switch to mxbai-embed-large, which handles all content perfectly but produces 1024-dimensional embeddings instead of the currently supported 768 and 1536 dimensions.

## Context

### Current Embedding Support

The codebase currently supports two embedding dimensions:
- **768-dim**: Ollama nomic-embed-text → `vec_code_768` table
- **1536-dim**: OpenAI text-embedding-3-small → `vec_code` table

Dimension mapping is enforced in multiple locations:
- `/workspace/crates/maproom/src/db/sqlite/embeddings.rs:32` - `SUPPORTED_DIMENSIONS: &[usize] = &[768, 1536]`
- `/workspace/crates/maproom/src/db/sqlite/vector.rs:6` - Same constant
- `/workspace/crates/maproom/src/db/columns.rs:91-98` - `select_columns_for_dimension()` function
- `/workspace/crates/maproom/src/embedding/config.rs:266-276` - Validation for nomic-embed-text requires 768

### Why This Matters

The mxbai-embed-large model:
- **Handles all characters**: No tokenization crashes on |, [], (), Unicode
- **Better quality**: Outperforms nomic-embed-text on MTEB benchmarks
- **Clean solution**: Removes 40+ lines of character sanitization workaround
- **Acceptable tradeoffs**: Slightly larger model (670MB vs 274MB), ~30% more storage per embedding

## Existing Solutions

### Industry Patterns

Multiple embedding dimension support is standard practice:
- **OpenAI**: Offers 256, 512, 1536, 3072 dimensions via `dimensions` parameter
- **Cohere**: Offers 384, 768, 1024 dimensions via `embedding_types` parameter
- **Google Vertex**: Fixed dimensions per model (768 for gecko, 768 for gecko-multilingual)

### Codebase Patterns

The codebase already has the infrastructure for multi-dimensional support:
1. **Virtual table per dimension**: `vec_code` (1536), `vec_code_768` (768)
2. **Dimension tracking**: `code_embeddings.embedding_dim` column
3. **Dynamic table selection**: `get_vec_table_name()` in embeddings.rs and vector.rs
4. **Migration system**: Versioned, idempotent migrations in `migrations.rs`

Adding 1024-dim follows the exact pattern used for 768-dim in Migration #7.

## Current State

### Dimension Mapping

The mapping is consistent across three modules:

**embeddings.rs** (storage):
```rust
fn get_vec_table_name(dimension: usize) -> Result<&'static str> {
    match dimension {
        768 => Ok("vec_code_768"),
        1536 => Ok("vec_code"),
        _ => bail!("Unsupported embedding dimension: {}. Supported dimensions: {:?}", dimension, SUPPORTED_DIMENSIONS)
    }
}
```

**vector.rs** (search):
```rust
// Identical function, duplicate implementation
fn get_vec_table_name(dimension: usize) -> Result<&'static str> {
    match dimension {
        768 => Ok("vec_code_768"),
        1536 => Ok("vec_code"),
        _ => bail!("Unsupported embedding dimension: {}. Supported dimensions: {:?}", dimension, SUPPORTED_DIMENSIONS)
    }
}
```

**columns.rs** (PostgreSQL column selection - NOT used by SQLite):
```rust
pub fn select_columns_for_dimension(dimension: usize) -> anyhow::Result<ColumnSet> {
    match dimension {
        768 => Ok(ColumnSet::OLLAMA),
        1536 => Ok(ColumnSet::OPENAI),
        _ => Err(anyhow::anyhow!("Unsupported embedding dimension: {}. Supported dimensions: 768 (Ollama/Google), 1536 (OpenAI)", dimension))
    }
}
```

**Note**: `columns.rs` is for PostgreSQL (code comments reference `*_embedding_ollama` columns which don't exist in SQLite schema). SQLite uses the virtual table approach instead.

### Ollama Configuration Validation

`config.rs` (line 266-276) enforces dimension requirements:
```rust
if self.provider == Provider::Ollama && self.model == "nomic-embed-text" {
    if self.dimension != 768 {
        return Err(ConfigError::InvalidValue {
            field: "dimension".to_string(),
            reason: format!(
                "Ollama provider with nomic-embed-text requires dimension=768, got {}",
                self.dimension
            ),
        });
    }
}
```

This validation is **overly strict** - it only allows 768 for nomic-embed-text, but doesn't account for other Ollama models like mxbai-embed-large (1024-dim).

### Sanitization Workaround

Lines 344-386 in `ollama.rs` implement character replacement:
```rust
let sanitized_texts: Vec<String> = texts
    .into_iter()
    .map(|t| {
        let sanitized = t
            .replace('|', " ") // Markdown table pipes
            .replace('[', "(") // Brackets (checkboxes, links)
            .replace(']', ")")
            .replace('→', "->") // Unicode arrows
            .replace('←', "<-")
            .replace('↔', "<->")
            // Box-drawing characters (directory trees)
            .replace('├', "+")
            .replace('└', "+")
            .replace('│', " ")
            .replace('─', "-")
            // ... 15 more replacements
```

This workaround is **removed entirely** when switching to mxbai-embed-large.

## Research Findings

### Testing Evidence

From archived project EMBPERF (Ollama parallel optimization):
- mxbai-embed-large handles all content types without crashes
- Throughput: 6,780 tokens/sec with batch size 64 (slightly slower than nomic-embed-text but acceptable)
- Quality: Superior on MTEB benchmarks (see docs/providers/ollama-setup.md)

### Migration Pattern

Migration #7 added vec_code_768 for 768-dim:
```sql
CREATE VIRTUAL TABLE vec_code_768 USING vec0(
    embedding float[768]
);
```

Migration #10 (new) will add vec_code_1024 for 1024-dim following the same pattern.

### Storage Impact

- **Per embedding**: 1024 floats × 4 bytes = 4,096 bytes (vs 3,072 for 768-dim)
- **Increase**: 33% more storage than 768-dim, 27% less than 1536-dim
- **Acceptable**: Storage is cheap, quality and reliability matter more

## Constraints

### Technical Constraints

1. **sqlite-vec limitation**: Fixed dimension per virtual table (cannot mix dimensions)
2. **One dimension per model**: Each embedding model produces a fixed dimension
3. **Backward compatibility**: Must not break existing 768 and 1536-dim embeddings
4. **Migration safety**: One-time migration, must be idempotent and backward compatible

### Business Constraints

1. **MVP timeline**: This is a focused fix, not a feature expansion
2. **Testing scope**: Must verify all three dimensions work (768, 1024, 1536)
3. **Documentation**: Must document configuration for mxbai-embed-large

### Resource Constraints

1. **Model size**: mxbai-embed-large is 670MB (vs 274MB for nomic-embed-text)
2. **VRAM**: ~1.5GB required (acceptable for dev machines, may be tight for low-memory environments)
3. **Re-embedding**: Users must re-generate embeddings after switching models

## Success Criteria

### Functional Success

1. **1024-dim support**: Database accepts and searches 1024-dimensional embeddings
2. **mxbai-embed-large works**: Model configurable via environment variables
3. **No crashes**: All content types embed successfully (|, [], (), Unicode, etc.)
4. **Character preservation**: No content mangling (sanitization code removed)

### Quality Success

1. **All tests pass**: Unit tests for 1024-dim added and passing
2. **Mixed dimensions**: Existing 768 and 1536-dim embeddings still work
3. **Search quality**: Vector search returns relevant results for 1024-dim embeddings
4. **Migration safety**: Migration #10 is idempotent and backward compatible

### Operational Success

1. **Configuration clear**: Documentation explains how to use mxbai-embed-large
2. **Error messages helpful**: Unsupported dimensions report all supported values
3. **Graceful fallback**: If mxbai-embed-large unavailable, clear error message

### Non-Goals (Future Work)

1. **Not configuring dimension dynamically**: Dimension still tied to model choice
2. **Not removing 768/1536 support**: All three dimensions coexist
3. **Not automatic migration**: Users manually switch models and re-embed
4. **Not optimizing storage**: No compression or quantization (out of scope)
