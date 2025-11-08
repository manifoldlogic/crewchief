# Security Review: Content-Addressed Chunk Storage

## Threat Model

### Assets

1. **Code embeddings** - Expensive to generate ($0.00002 each), intellectual property
2. **Source code chunks** - Customer source code, potentially sensitive
3. **Database integrity** - Critical for search functionality
4. **Embedding API keys** - OpenAI credentials ($$ value)

### Attackers

**In scope**:
- Malicious code submissions (attempting hash collisions)
- SQL injection via chunk content
- Denial of service via migration

**Out of scope** (infrastructure-level):
- Database server compromise
- Network eavesdropping
- Physical access to servers

## Security Analysis by Component

### 1. Blob SHA Computation

#### Threat: Hash Collision Attack

**Scenario**: Attacker crafts two different code chunks with same SHA-256 hash

**Likelihood**: Cryptographically infeasible
- SHA-256 collision probability: ~2^-256
- Would require more compute than exists on Earth
- No practical SHA-256 collisions known (only theoretical SHA-1)

**Impact**: High (if possible)
- Would cause wrong embeddings to be retrieved
- Could corrupt search results

**Mitigation**: ✅ Built-in to SHA-256
- Use cryptographically secure hash (we do)
- No additional action needed

**Risk Level**: 🟢 **ACCEPTED** (theoretical only)

#### Threat: Hash Injection

**Scenario**: Attacker provides malicious blob_sha value directly

**Likelihood**: Low (if using parameterized queries)

**Impact**: Medium
- Could reference non-existent embedding
- Could cause foreign key violations

**Mitigation**:
```rust
// ✅ GOOD: Compute blob SHA from content, never trust user input
let blob_sha = compute_blob_sha(&chunk.content);

// ❌ BAD: Accept blob SHA from user
// let blob_sha = chunk.blob_sha; // NEVER DO THIS
```

**Enforcement**:
```rust
// Make blob_sha private in struct
pub struct ParsedChunk {
    pub content: String,
    // No public blob_sha field - always computed
}

impl ParsedChunk {
    pub fn blob_sha(&self) -> String {
        compute_blob_sha(&self.content)
    }
}
```

**Risk Level**: 🟢 **MITIGATED** (by design)

### 2. SQL Injection

#### Threat: Content-Based SQL Injection

**Scenario**: Malicious code chunk contains SQL commands

```typescript
// Malicious content
const content = "'; DROP TABLE code_embeddings; --";
```

**Likelihood**: Low (with parameterized queries)

**Impact**: Critical
- Could delete embeddings
- Could corrupt database

**Mitigation**: ✅ Use parameterized queries everywhere

```rust
// ✅ GOOD: Parameterized query
sqlx::query!(
    "INSERT INTO chunks (content, blob_sha) VALUES ($1, $2)",
    chunk.content,
    blob_sha
)
.execute(pool)
.await?;

// ❌ BAD: String concatenation
// let query = format!("INSERT INTO chunks (content) VALUES ('{}')", chunk.content);
```

**Validation**:
- Run SQL injection test suite
- Use `sqlx`'s compile-time query validation
- Never use raw SQL strings with user content

**Risk Level**: 🟢 **MITIGATED** (by tooling)

#### Threat: Migration SQL Injection

**Scenario**: Malicious content in database before migration

**Likelihood**: Low (same as above)

**Impact**: Critical

**Mitigation**: Migration uses PostgreSQL function (parameterized)

```sql
-- ✅ SAFE: PostgreSQL function uses parameters internally
UPDATE chunks SET blob_sha = compute_git_blob_sha(content);

-- Function implementation uses digest() which is safe
CREATE OR REPLACE FUNCTION compute_git_blob_sha(content TEXT)
RETURNS TEXT AS $$
  SELECT encode(digest('blob ' || length(content) || E'\0' || content, 'sha256'), 'hex');
$$ LANGUAGE SQL IMMUTABLE;
```

**Risk Level**: 🟢 **MITIGATED**

### 3. Denial of Service

#### Threat: Migration Locks Database

**Scenario**: Long-running migration blocks all queries

**Likelihood**: Medium (large tables take time)

**Impact**: High
- Service unavailable during migration
- Could timeout and leave partial state

**Mitigation**:

**Strategy 1**: Use `CONCURRENTLY` for index creation
```sql
-- ✅ Doesn't block reads/writes
CREATE INDEX CONCURRENTLY idx_chunks_blob_sha ON chunks(blob_sha);

-- ❌ Locks table
-- CREATE INDEX idx_chunks_blob_sha ON chunks(blob_sha);
```

**Strategy 2**: Batch updates with explicit commits
```sql
DO $$
DECLARE
  batch_size INT := 1000;
BEGIN
  LOOP
    UPDATE chunks
    SET blob_sha = compute_git_blob_sha(content)
    WHERE chunk_id IN (
      SELECT chunk_id FROM chunks
      WHERE blob_sha IS NULL
      LIMIT batch_size
    );

    EXIT WHEN NOT FOUND;
    COMMIT; -- Release locks between batches
  END LOOP;
END $$;
```

**Strategy 3**: Run migration during maintenance window
- Schedule downtime
- Notify users
- Have rollback plan ready

**Risk Level**: 🟡 **MITIGATED** (with careful execution)

#### Threat: Embedding API Rate Limiting

**Scenario**: Regenerating all embeddings hits OpenAI rate limit

**Likelihood**: Low (for this project)
- We're NOT regenerating embeddings
- Only computing blob SHA (local operation)

**Impact**: None (not applicable)

**Risk Level**: 🟢 **NOT APPLICABLE** (deduplication prevents this)

#### Threat: Infinite Loop in Batch Migration

**Scenario**: Migration batching logic never terminates

**Likelihood**: Low

**Impact**: Medium (hangs migration)

**Mitigation**:
```sql
-- Add safety counter
DO $$
DECLARE
  batch_size INT := 1000;
  max_iterations INT := 10000; -- Safety limit
  iterations INT := 0;
BEGIN
  LOOP
    iterations := iterations + 1;

    IF iterations > max_iterations THEN
      RAISE EXCEPTION 'Migration exceeded max iterations';
    END IF;

    -- ... migration logic ...

    EXIT WHEN NOT FOUND;
  END LOOP;
END $$;
```

**Risk Level**: 🟢 **MITIGATED**

### 4. Data Integrity

#### Threat: Embedding Loss During Migration

**Scenario**: Migration fails midway, some embeddings lost

**Likelihood**: Medium (migrations can fail)

**Impact**: Critical
- Lost embeddings = expensive to regenerate
- Search quality degraded

**Mitigation**:

**Strategy 1**: Transaction-based migration
```sql
BEGIN;
  -- Create table
  CREATE TABLE code_embeddings (...);

  -- Migrate data
  INSERT INTO code_embeddings (...) SELECT ...;

  -- Validate
  -- If anything fails, entire transaction rolls back
COMMIT;
```

**Strategy 2**: Pre-migration validation
```sql
-- Count before
SELECT COUNT(*) FROM chunks WHERE embedding IS NOT NULL;
-- Should equal count after migration

-- Count after
SELECT COUNT(*) FROM code_embeddings;
```

**Strategy 3**: Backup before migration
```bash
pg_dump maproom > backup_before_blobsha.sql
```

**Risk Level**: 🟡 **MITIGATED** (with validation)

#### Threat: Foreign Key Violation

**Scenario**: Chunk references non-existent blob_sha

**Likelihood**: Low (if migration correct)

**Impact**: Medium (broken search results)

**Mitigation**:
```sql
-- Add foreign key constraint
ALTER TABLE chunks
ADD CONSTRAINT fk_chunks_embedding
FOREIGN KEY (blob_sha) REFERENCES code_embeddings(blob_sha);

-- This will FAIL if any orphaned blob_sha exists
-- Which is GOOD - catches integrity issues before they cause problems
```

**Risk Level**: 🟢 **MITIGATED** (by constraint)

### 5. API Key Exposure

#### Threat: Embedding API Key in Logs

**Scenario**: OpenAI API key logged accidentally

**Likelihood**: Low (if following best practices)

**Impact**: Critical
- Unauthorized API usage
- Cost exposure

**Mitigation**:

```rust
// ✅ GOOD: Don't log API key
debug!("Generating embedding for chunk {}", chunk_id);

// ❌ BAD: API key in logs
// debug!("API call: {}", request_with_key);
```

**Environment variable handling**:
```rust
let api_key = env::var("OPENAI_API_KEY")
    .expect("OPENAI_API_KEY must be set");

// Never print api_key
// Never include in error messages
```

**Risk Level**: 🟢 **MITIGATED** (by best practices)

#### Threat: API Key in Database

**Scenario**: API key accidentally stored in code_embeddings

**Likelihood**: Very Low

**Impact**: Critical

**Mitigation**: Schema doesn't have API key field (impossible)

**Risk Level**: 🟢 **NOT APPLICABLE**

## Security Checklist

### Design Phase
- [x] Use cryptographically secure hash (SHA-256)
- [x] Parameterized queries for all SQL
- [x] Foreign key constraints for referential integrity
- [x] Transaction-based migrations
- [x] Batch processing with commit intervals

### Implementation Phase
- [ ] Code review: No raw SQL with user content
- [ ] Code review: No blob_sha from user input
- [ ] Code review: No API keys in logs
- [ ] Validate all SQL uses sqlx macros or parameterized queries
- [ ] Test SQL injection attempts (should fail safely)

### Testing Phase
- [ ] Test migration rollback
- [ ] Test foreign key constraint violations
- [ ] Test with malicious content (SQL injection attempts)
- [ ] Verify backup/restore procedure

### Deployment Phase
- [ ] Backup database before migration
- [ ] Run migration in transaction
- [ ] Validate row counts before/after
- [ ] Monitor for errors during migration
- [ ] Have rollback script ready

## Compliance Considerations

### GDPR (if applicable)

**Right to erasure**: Can delete customer chunks and embeddings

```sql
-- Delete user's code chunks
DELETE FROM chunks WHERE file_path LIKE '/repos/:user_id/%';

-- Cascade delete will remove embeddings if no other chunks reference them
-- (handled by foreign key ON DELETE CASCADE if configured)
```

**Data minimization**: Only store necessary data
- ✅ We don't store PII
- ✅ Code content is necessary for embeddings
- ✅ blob_sha is necessary for deduplication

### SOC 2 (if applicable)

**Access controls**:
- Database credentials via environment variables (not hardcoded)
- Least privilege: Application user can't DROP tables

```sql
-- Create limited application user
CREATE USER maproom_app WITH PASSWORD 'secure-password';
GRANT SELECT, INSERT, UPDATE, DELETE ON chunks, code_embeddings TO maproom_app;
-- Don't grant DROP, TRUNCATE, ALTER
```

## Known Limitations

### Accepted Risks

1. **SHA-256 collision** - Theoretical only, accepting risk
2. **Migration downtime** - Mitigated with batching, some risk remains
3. **Content visibility** - Code chunks stored in plaintext (by design)

### Not Implemented (Deferred)

**Encryption at rest**: Not in MVP
- Rationale: Database-level encryption is infrastructure concern
- Future: Can enable PostgreSQL TDE if needed

**Audit logging**: Not in MVP
- Rationale: Focus on core functionality
- Future: Can add audit log for embedding generation

**Rate limiting**: Not in MVP
- Rationale: Single-tenant system, controlled usage
- Future: Add if multi-tenant

## Security Review Sign-Off

**Reviewer**: AI Agent (to be reviewed by human)
**Date**: 2025-11-08
**Status**: ✅ **APPROVED FOR MVP**

**Summary**:
- No critical unmitigated risks
- Standard SQL injection protections in place
- Migration strategy includes safety measures
- Backup and rollback procedures defined

**Recommendation**: Proceed with implementation

**Post-deployment monitoring**:
- Monitor query error rates (should be zero)
- Monitor migration duration (should complete in expected time)
- Verify deduplication working (embedding count < chunk count)
