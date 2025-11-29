# Architecture: Embedding Inheritance

## Solution Overview

Add a pre-generation step to the embedding pipeline that copies existing embeddings from `code_embeddings` to chunks before attempting API generation.

## Design Principles

1. **Minimal change**: Work within existing pipeline structure
2. **Fail-safe**: If lookup fails, fall back to generation
3. **Idempotent**: Safe to run multiple times
4. **Measurable**: Track copy hits vs generation

## Component Changes

### 1. Embedding Pipeline (`crates/maproom/src/embedding/pipeline.rs`)

Add new step before generation:

```rust
pub async fn run(&self, client: &Client) -> Result<PipelineStats> {
    // EXISTING: Find chunks with NULL embeddings
    let chunks = self.find_chunks_needing_embeddings(client).await?;

    // NEW: Copy embeddings from code_embeddings table
    let (copied, still_null) = self.copy_existing_embeddings(client, &chunks).await?;

    // EXISTING: Generate for remaining NULL chunks
    let generated = self.generate_embeddings(client, &still_null).await?;

    Ok(PipelineStats {
        total_chunks: chunks.len(),
        copied_from_cache: copied,
        generated_new: generated,
        ...
    })
}
```

### 2. New Function: `copy_existing_embeddings`

```sql
UPDATE maproom.chunks c
SET
    code_embedding = ce.code_embedding,
    text_embedding = ce.text_embedding,
    updated_at = NOW()
FROM maproom.code_embeddings ce
WHERE
    c.blob_sha = ce.blob_sha
    AND (c.code_embedding IS NULL OR c.text_embedding IS NULL)
RETURNING c.id;
```

Single UPDATE with JOIN - efficient, atomic, correct.

### 3. Stats Tracking

Extend `PipelineStats` struct:

```rust
pub struct PipelineStats {
    pub total_chunks: usize,
    pub copied_from_cache: usize,  // NEW
    pub generated_new: usize,
    pub skipped: usize,
    pub cost_saved_usd: f64,       // NEW: copied * cost_per_embedding
    ...
}
```

## Data Flow

```
Scan Phase:
  ├─ Insert chunks with blob_sha
  └─ Leave embeddings NULL

Embedding Phase:
  ├─ Find chunks with NULL embeddings
  ├─ Copy from code_embeddings (by blob_sha) ← NEW
  ├─ Generate for remaining NULLs
  └─ Update stats
```

## Performance Characteristics

**Before**:
- 42K chunks → 42K API calls → hours
- Cost: 42K × $0.00013 = $5.46

**After**:
- 42K chunks → SQL JOIN → ~1 second
- 100 new chunks → 100 API calls → ~30 seconds
- Cost: 100 × $0.00013 = $0.013

**Improvement**: 200-500× faster, 400× cheaper

## Backward Compatibility

- No schema changes
- No breaking API changes
- Existing embeddings unaffected
- Falls back to generation if copy fails

## Edge Cases

1. **Partial embeddings**: Chunk has `code_embedding` but not `text_embedding`
   - Solution: Update query checks both fields independently

2. **Multiple chunks same blob SHA**:
   - Solution: UPDATE works for all matching rows

3. **code_embeddings missing**:
   - Solution: No match in JOIN, falls through to generation

4. **Concurrent updates**:
   - Solution: UPDATE is atomic, no coordination needed

## Future Optimization

Not in scope but noted:
- Batch copy before scan (pre-warm cache)
- Copy during upsert (avoid NULL entirely)
- Materialized view for faster lookup

Keep it simple: post-upsert copy is sufficient.
