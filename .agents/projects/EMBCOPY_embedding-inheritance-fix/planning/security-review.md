# Security Review: Embedding Inheritance

## Scope

This change adds a SQL UPDATE with JOIN operation to copy embeddings. Security review focuses on SQL injection and data integrity.

## SQL Injection Analysis

### Query Pattern

```sql
UPDATE maproom.chunks c
SET code_embedding = ce.code_embedding, text_embedding = ce.text_embedding
FROM maproom.code_embeddings ce
WHERE c.blob_sha = ce.blob_sha
AND (c.code_embedding IS NULL OR c.text_embedding IS NULL)
```

**Assessment**: ✅ Safe
- No user input in query
- No dynamic SQL construction
- All identifiers are hardcoded
- Uses parameterized query pattern (sqlx)

## Data Integrity

### Embedding Correctness

**Concern**: Copying wrong embedding to wrong chunk

**Mitigation**:
- Join on blob_sha (content hash)
- Blob SHA is deterministic (Git algorithm)
- Same content → same blob SHA → correct embedding
- Already in production for cache check

**Risk**: Low - same logic as existing cache check

### Concurrent Updates

**Concern**: Race condition during copy

**Mitigation**:
- UPDATE is atomic
- No multi-statement transaction needed
- Idempotent operation (safe to retry)

**Risk**: None - database handles concurrency

## Access Control

**Change**: None - uses existing database connection
**Risk**: None - no new permissions needed

## Privacy/Compliance

**Data**: Embeddings are already stored, just being copied
**Risk**: None - no new PII, no new external access

## Denial of Service

**Concern**: Large UPDATE could lock table

**Mitigation**:
- Copy happens during embedding phase (already slow)
- WHERE clause limits to NULL chunks
- Indexes on blob_sha exist (BLOBSHA project)

**Risk**: Low - bounded by NULL chunk count

## Secrets Management

**Change**: None
**Risk**: None

## Audit Trail

**Logging**: Stats track copied count (observability)
**Metrics**: Cache metrics already exist
**Risk**: None

## Known Gaps

None - this is a simple optimization with no security surface.

## Conclusion

✅ **Approved for implementation**

No meaningful security risks. The change uses existing infrastructure (blob_sha index, code_embeddings table) and adds a safe, parameterized UPDATE query.
