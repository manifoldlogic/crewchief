# MPEMBED: Multi-Provider Embedding Support - Implementation Plan

## Overview

This plan breaks the project into 5 phases, each with clear deliverables and dependencies. Phases are designed for sequential execution with minimal rework.

**Timeline:** 2-3 weeks (11-16 days of engineering effort)
**Team size:** 2-3 agents working in parallel where possible
**Risk level:** Low (well-defined requirements, stable interfaces)

## Phase 0: Pre-Implementation Setup (Day 0, 0.5 days)

**Goal:** Establish baseline and prepare for migration

### Deliverables:
1. **Fixture database snapshot**
   - Backup production database (23,632 chunks, OpenAI embeddings)
   - Create test fixture with 100 chunks for fast iteration
   - Document current schema and indexes

2. **Performance baseline**
   - Measure search latency (current: ~50ms p95)
   - Measure index sizes (current: ~150MB for 1536-dim)
   - Document embedding generation throughput (current: OpenAI only)

3. **Dependency audit**
   - Run `cargo audit` to check for vulnerabilities
   - Update outdated dependencies (non-breaking)
   - Document required new dependencies:
     - `google-cloud-auth` (Google Vertex AI)
     - `async-trait` (provider trait)

### Agents:
- **database-engineer**: Create backups and fixtures
- **performance-engineer**: Establish baselines

### Success criteria:
- ✅ Production database backed up
- ✅ Test fixture loads in <5 seconds
- ✅ Baseline metrics documented
- ✅ No critical vulnerabilities in `cargo audit`

---

## Phase 1: Database Migration (Days 1-2, 1.5 days)

**Goal:** Add 768-dimensional columns without breaking existing functionality

### Tasks:

#### 1.1: Write Migration SQL
**Agent:** migration-safety-specialist

```sql
-- migration 0015_add_ollama_columns.sql
BEGIN;

-- Add 768-dimensional columns
ALTER TABLE maproom.chunks
  ADD COLUMN code_embedding_ollama vector(768),
  ADD COLUMN text_embedding_ollama vector(768);

-- Create IVFFlat indexes
CREATE INDEX CONCURRENTLY idx_chunks_code_vec_ollama
  ON maproom.chunks
  USING ivfflat (code_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);

CREATE INDEX CONCURRENTLY idx_chunks_text_vec_ollama
  ON maproom.chunks
  USING ivfflat (text_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);

COMMIT;
```

#### 1.2: Write Rollback SQL
**Agent:** migration-safety-specialist

```sql
-- migration 0015_add_ollama_columns_rollback.sql
BEGIN;

DROP INDEX CONCURRENTLY IF EXISTS idx_chunks_code_vec_ollama;
DROP INDEX CONCURRENTLY IF EXISTS idx_chunks_text_vec_ollama;

ALTER TABLE maproom.chunks
  DROP COLUMN IF EXISTS code_embedding_ollama,
  DROP COLUMN IF EXISTS text_embedding_ollama;

COMMIT;
```

#### 1.3: Verification Script
**Agent:** migration-safety-specialist

```bash
#!/bin/bash
# scripts/verify_migration_0015.sh

set -e

echo "Verifying migration 0015..."

# Check columns exist
psql $DATABASE_URL -c "SELECT code_embedding_ollama FROM maproom.chunks LIMIT 1" > /dev/null
echo "✓ code_embedding_ollama column exists"

psql $DATABASE_URL -c "SELECT text_embedding_ollama FROM maproom.chunks LIMIT 1" > /dev/null
echo "✓ text_embedding_ollama column exists"

# Check indexes exist
psql $DATABASE_URL -c "SELECT indexname FROM pg_indexes WHERE tablename='chunks' AND indexname='idx_chunks_code_vec_ollama'" | grep -q idx_chunks_code_vec_ollama
echo "✓ idx_chunks_code_vec_ollama index exists"

psql $DATABASE_URL -c "SELECT indexname FROM pg_indexes WHERE tablename='chunks' AND indexname='idx_chunks_text_vec_ollama'" | grep -q idx_chunks_text_vec_ollama
echo "✓ idx_chunks_text_vec_ollama index exists"

# Check existing embeddings preserved
OPENAI_COUNT=$(psql $DATABASE_URL -t -c "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL")
echo "✓ OpenAI embeddings preserved: $OPENAI_COUNT chunks"

# Check new columns are NULL
OLLAMA_COUNT=$(psql $DATABASE_URL -t -c "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding_ollama IS NOT NULL")
if [ "$OLLAMA_COUNT" -eq 0 ]; then
  echo "✓ Ollama columns are empty (as expected)"
else
  echo "⚠ Warning: Ollama columns have $OLLAMA_COUNT non-NULL values"
fi

echo "Migration verification complete!"
```

#### 1.4: Test on Fixture
**Agent:** database-engineer

- Run migration on 100-chunk fixture
- Verify 0 data loss (OpenAI embeddings intact)
- Measure index build time (~5 seconds for 100 chunks)

#### 1.5: Run on Production
**Agent:** migration-safety-specialist (with DBA supervision)

- Schedule maintenance window (if downtime needed)
- Run migration with verification
- Monitor index build (CREATE INDEX CONCURRENTLY allows reads)
- Run rollback if verification fails

### Deliverables:
- `migrations/0015_add_ollama_columns.sql`
- `migrations/0015_add_ollama_columns_rollback.sql`
- `scripts/verify_migration_0015.sh`
- Migration runbook (documentation)

### Dependencies:
- Phase 0 complete (backups exist)

### Success criteria:
- ✅ Migration runs successfully on production
- ✅ All existing OpenAI embeddings preserved (0 data loss)
- ✅ New columns exist and are NULL
- ✅ Indexes created successfully
- ✅ Search queries still work (use existing 1536-dim columns)
- ✅ Rollback tested on staging environment

---

## Phase 2: Provider Abstraction (Days 3-5, 3 days)

**Goal:** Refactor embedding service to support multiple providers via trait

### Tasks:

#### 2.1: Define EmbeddingProvider Trait
**Agent:** provider-abstraction-architect

- Design trait with methods: `embed()`, `embed_batch()`, `dimension()`, `provider_name()`
- Choose trait object strategy (`Box<dyn EmbeddingProvider>`)
- Ensure object safety (all methods use `&self`, return concrete types)
- Add metrics method (optional, for OpenAI cost tracking)

**Deliverable:** `crates/maproom/src/embedding/provider.rs`

#### 2.2: Implement OllamaProvider
**Agent:** rust-indexer-engineer

- HTTP client for Ollama API (`POST /api/embed`)
- Handle Ollama-specific request/response JSON schemas
- Implement concurrent batching (Ollama doesn't support native batching)
- Add retry logic for transient failures

**Deliverable:** `crates/maproom/src/embedding/ollama.rs`

#### 2.3: Refactor OpenAIClient to Implement Trait
**Agent:** embeddings-engineer

- Wrap existing `OpenAIClient` to implement `EmbeddingProvider` trait
- Preserve existing behavior (caching, retry logic, cost tracking)
- Ensure backward compatibility (existing code still works)

**Deliverable:** `crates/maproom/src/embedding/openai.rs` (modified)

#### 2.4: Implement Provider Factory
**Agent:** provider-abstraction-architect

- Auto-detect Ollama (try `http://localhost:11434/api/tags`)
- Fall back to `EMBEDDING_PROVIDER` env var
- Validate configuration (API keys, endpoints, models)
- Return `Box<dyn EmbeddingProvider>` with graceful errors

**Deliverable:** `crates/maproom/src/embedding/factory.rs`

#### 2.5: Refactor EmbeddingService
**Agent:** embeddings-engineer

- Replace `OpenAIClient` field with `Box<dyn EmbeddingProvider>`
- Update `from_env()` to use factory
- Add `dimension()` and `provider_name()` methods
- Ensure caching layer remains provider-agnostic

**Deliverable:** `crates/maproom/src/embedding/service.rs` (modified)

### Deliverables:
- Provider trait and implementations (Ollama, OpenAI)
- Provider factory with auto-detection
- Refactored EmbeddingService

### Dependencies:
- Phase 1 complete (database ready for both dimensions)

### Success criteria:
- ✅ Ollama provider generates 768-dim embeddings
- ✅ OpenAI provider generates 1536-dim embeddings (unchanged)
- ✅ Auto-detection prefers Ollama if available
- ✅ Switching providers via env var works
- ✅ Existing OpenAI-based workflows unaffected
- ✅ Unit tests pass for provider factory

---

## Phase 3: Google Vertex AI Integration (Days 6-8, 3 days)

**Goal:** Add Google Cloud Vertex AI as third provider option

### Tasks:

#### 3.1: Implement GoogleProvider
**Agent:** google-cloud-integration-engineer

- Authenticate with service account JSON key
- Call Vertex AI predict endpoint (REST API)
- Handle regional endpoints (us-central1, europe-west1, etc.)
- Support task types (RETRIEVAL_DOCUMENT, RETRIEVAL_QUERY)
- Implement native batching (Google supports multiple instances)

**Deliverable:** `crates/maproom/src/embedding/google.rs`

#### 3.2: Add Google to Factory
**Agent:** provider-abstraction-architect

- Add "google" case to factory match statement
- Validate required env vars (GOOGLE_PROJECT_ID, credentials path)
- Check service account file permissions (warn if >600)
- Handle auth errors gracefully

**Deliverable:** `crates/maproom/src/embedding/factory.rs` (modified)

#### 3.3: Integration Tests
**Agent:** google-cloud-integration-engineer

- Test with real GCP project (CI credentials)
- Verify 768-dim embeddings persist correctly
- Test regional endpoint switching
- Test IAM permission errors (should fail gracefully)

**Deliverable:** `crates/maproom/tests/google_provider_integration.rs`

#### 3.4: Documentation
**Agent:** google-cloud-integration-engineer

- Setup guide: Create service account, grant IAM roles, generate key
- Configuration examples for multiple regions
- Troubleshooting section (auth errors, quota limits)

**Deliverable:** `docs/providers/google-vertex-ai-setup.md`

### Deliverables:
- GoogleProvider implementation
- Integration tests with GCP
- Setup documentation

### Dependencies:
- Phase 2 complete (provider abstraction exists)
- GCP test project with Vertex AI enabled

### Success criteria:
- ✅ Google provider generates 768-dim embeddings
- ✅ Service account auth works in test environment
- ✅ Embeddings persist to `*_ollama` columns (column sharing)
- ✅ Integration tests pass with real GCP project
- ✅ Documentation enables non-GCP-expert to set up
- ✅ IAM follows least-privilege principle

---

## Phase 4: Database and Search Integration (Days 9-11, 3 days)

**Goal:** Update database operations and search queries for multi-dimension support

### Tasks:

#### 4.1: Column Selection Logic
**Agent:** database-engineer

```rust
pub fn select_columns_for_dimension(dimension: usize) -> Result<(&'static str, &'static str), DbError> {
    match dimension {
        768 => Ok(("code_embedding_ollama", "text_embedding_ollama")),
        1536 => Ok(("code_embedding", "text_embedding")),
        _ => Err(DbError::InvalidDimension(dimension)),
    }
}
```

**Deliverable:** `crates/maproom/src/db/columns.rs`

#### 4.2: Update Embedding Upsert
**Agent:** database-engineer

- Modify `upsert_embeddings()` to accept dimension parameter
- Select columns dynamically based on dimension
- Use parameterized queries (SQL injection safe)

**Deliverable:** `crates/maproom/src/db/chunks.rs` (modified)

#### 4.3: Update Search Queries
**Agent:** vector-database-engineer

- Hybrid search: Use COALESCE pattern for mixed embeddings
- Vector-only search: Select column based on query dimension
- FTS-only search: Unchanged (no vectors involved)

**Example:**
```sql
SELECT
  c.id,
  c.symbol_name,
  COALESCE(
    1 - (c.code_embedding_ollama <=> $1),
    1 - (c.code_embedding <=> $2)
  ) as similarity
FROM maproom.chunks c
WHERE c.code_embedding_ollama IS NOT NULL OR c.code_embedding IS NOT NULL
ORDER BY similarity DESC
LIMIT 10;
```

**Deliverable:** `crates/maproom/src/search/hybrid.rs` (modified)

#### 4.4: Integration with Embedding Pipeline
**Agent:** embeddings-engineer

- Pass provider dimension to `upsert_embeddings()`
- Update batch processing to handle dimension parameter
- Ensure incremental embedding generation uses correct columns

**Deliverable:** `crates/maproom/src/embedding/pipeline.rs` (modified)

#### 4.5: Test Mixed Embeddings
**Agent:** integration-tester

- Create fixture with 50 OpenAI chunks + 50 Ollama chunks
- Verify search returns results from both providers
- Test preference order (768-dim prioritized in COALESCE)

**Deliverable:** `crates/maproom/tests/mixed_embeddings_search.rs`

### Deliverables:
- Column selection logic
- Updated upsert and search queries
- Integration tests for mixed embeddings

### Dependencies:
- Phase 3 complete (all providers implemented)
- Phase 1 complete (database columns exist)

### Success criteria:
- ✅ Ollama embeddings insert into `*_ollama` columns
- ✅ Google embeddings insert into `*_ollama` columns
- ✅ OpenAI embeddings insert into original columns
- ✅ Search works with 768-dim queries
- ✅ Search works with 1536-dim queries
- ✅ Mixed embeddings search returns correct results
- ✅ COALESCE preference order works (768 > 1536)

---

## Phase 5: MCP Integration and Documentation (Days 12-14, 3 days)

**Goal:** Update MCP TypeScript wrapper and complete user-facing documentation

### Tasks:

#### 5.1: Provider Detection in MCP
**Agent:** mcp-tools-engineer

```typescript
async function detectProvider(): Promise<string | null> {
  // Check explicit config first
  const configProvider = process.env.EMBEDDING_PROVIDER;
  if (configProvider) {
    return configProvider.toLowerCase();
  }

  // Auto-detect Ollama
  try {
    const response = await fetch('http://localhost:11434/api/tags', {
      method: 'GET',
      signal: AbortSignal.timeout(2000),
    });
    if (response.ok) {
      return 'ollama';
    }
  } catch (error) {
    // Ollama not available
  }

  return null;
}
```

**Deliverable:** `packages/maproom-mcp/src/utils/provider-detection.ts`

#### 5.2: Update MCP Tools
**Agent:** mcp-tools-engineer

- Modify `scan.ts` to pass `--provider` flag
- Modify `upsert.ts` to pass `--provider` flag
- Update `search.ts` to handle both dimension types
- Add error handling for provider unavailability

**Deliverables:**
- `packages/maproom-mcp/src/tools/scan.ts` (modified)
- `packages/maproom-mcp/src/tools/upsert.ts` (modified)

#### 5.3: CLI Flag Additions
**Agent:** rust-indexer-engineer

```rust
#[derive(Args, Debug)]
pub struct Scan {
    // ... existing fields ...

    #[arg(long, help = "Embedding provider (ollama, google, openai)")]
    provider: Option<String>,
}
```

**Deliverable:** `crates/maproom/src/main.rs` (modified)

#### 5.4: Provider Comparison Documentation
**Agent:** mcp-tools-engineer

Create comparison table:

| Feature | Ollama | Google Vertex AI | OpenAI |
|---------|--------|------------------|--------|
| Cost | $0 | ~$0.10-0.20 per 100K chunks | ~$0.19 per 100K chunks |
| Speed | ~4.5 chunks/s | ~50-100 chunks/s | ~50-200 chunks/s |
| Privacy | 100% local | Cloud (Google) | Cloud (OpenAI) |
| Setup | Zero config | Service account | API key |
| Compliance | GDPR-friendly | BAA available | Not HIPAA |

**Deliverable:** `docs/providers/comparison.md`

#### 5.5: Setup Guides
**Agent:** mcp-tools-engineer

- **Ollama**: Installation, model download, verification
- **Google**: Service account creation, IAM roles, key setup
- **OpenAI**: API key acquisition, billing setup

**Deliverables:**
- `docs/providers/ollama-setup.md`
- `docs/providers/google-setup.md` (already done in Phase 3)
- `docs/providers/openai-setup.md`

#### 5.6: Migration Guide for Existing Users
**Agent:** mcp-tools-engineer

- Document how to preserve existing OpenAI embeddings
- Steps to switch from OpenAI to Ollama
- Steps to run both providers simultaneously

**Deliverable:** `docs/guides/provider-migration.md`

#### 5.7: README Updates
**Agent:** mcp-tools-engineer

- Update main README with provider options
- Add "Quick Start" section (Ollama zero-config)
- Add FAQ section (dimension questions, provider choice)

**Deliverable:** `README.md` (modified)

### Deliverables:
- MCP TypeScript wrapper updates
- CLI flag additions
- Complete documentation suite
- Updated README

### Dependencies:
- Phase 4 complete (database and search working)

### Success criteria:
- ✅ `npx -y @crewchief/maproom-mcp` works with Ollama (zero config)
- ✅ Setting `EMBEDDING_PROVIDER=google` switches to Google
- ✅ Setting `EMBEDDING_PROVIDER=openai` uses OpenAI
- ✅ Error messages are clear when provider unavailable
- ✅ Documentation enables users to set up any provider
- ✅ Migration guide tested by non-expert user

---

## Phase 6: Testing and Validation (Days 15-16, 2 days)

**Goal:** Comprehensive testing and performance validation

### Tasks:

#### 6.1: Contract Tests
**Agent:** contract-test-engineer

- Verify all providers satisfy `EmbeddingProvider` trait
- Test dimension consistency (embedding length == provider.dimension())
- Test batch ordering (outputs match inputs)

**Deliverable:** `crates/maproom/tests/provider_contract.rs`

#### 6.2: End-to-End Tests
**Agent:** integration-tester

- **Scenario 1:** Scan with Ollama → Search with 768-dim
- **Scenario 2:** Scan with Google → Search with 768-dim
- **Scenario 3:** Scan with OpenAI → Search with 1536-dim
- **Scenario 4:** Mixed embeddings (OpenAI + Ollama) → Search finds both

**Deliverable:** `crates/maproom/tests/e2e_multi_provider.rs`

#### 6.3: Performance Benchmarks
**Agent:** performance-engineer

- Search latency with COALESCE (compare to baseline)
- Embedding generation throughput (all providers)
- Index sizes (768-dim vs 1536-dim)
- Report any regressions

**Deliverable:** `benchmarks/multi_provider_performance.md`

#### 6.4: Security Audit
**Agent:** (Manual review by senior engineer)

- No API keys in logs or error messages
- Service account file permissions checked
- SQL injection prevention (all queries parameterized)
- TLS enforced for cloud providers

**Deliverable:** Security checklist sign-off

#### 6.5: Manual Testing
**Agent:** QA engineer or tech lead

- Complete manual testing checklist (from QUALITY_STRATEGY.md)
- Test on fresh installation (zero-config experience)
- Test provider switching (Ollama → Google → OpenAI)
- Verify backward compatibility (existing OpenAI users)

**Deliverable:** Manual testing report

### Deliverables:
- Contract tests, E2E tests, benchmarks
- Security audit sign-off
- Manual testing report

### Dependencies:
- Phase 5 complete (all features implemented)

### Success criteria:
- ✅ All automated tests pass
- ✅ Performance regressions <5% (acceptable)
- ✅ Security checklist complete (no critical issues)
- ✅ Manual testing report shows all scenarios working

---

## Post-Launch Activities (Ongoing)

### Monitoring
- Track embedding generation success/failure rates by provider
- Monitor search latency (p50, p95, p99)
- Watch for dimension mismatch errors (should be zero)

### User Feedback
- Collect feedback on provider setup difficulty
- Identify common configuration mistakes
- Document new troubleshooting scenarios

### Performance Tuning
- Adjust IVFFlat `lists` parameter based on corpus growth
- Optimize COALESCE queries if latency increases
- Consider materialized views for mixed embeddings

### Future Enhancements
- Add support for Cohere, Anthropic, HuggingFace providers
- Implement embedding quality evaluation (precision, recall)
- Add cost tracking dashboard for cloud providers
- Support custom embedding dimensions (e.g., OpenAI 256-3072 range)

---

## Risk Mitigation

### Risk 1: Migration Fails in Production
**Mitigation:**
- Test on staging environment first (25K chunk replica)
- Have rollback SQL ready and tested
- Schedule during low-traffic window
- Monitor database locks and query performance

### Risk 2: Google Auth Complexity Blocks Users
**Mitigation:**
- Provide step-by-step setup guide with screenshots
- Offer pre-configured example (dev project)
- Document common IAM permission errors
- Fallback: Ollama remains zero-config default

### Risk 3: Performance Regression from COALESCE
**Mitigation:**
- Benchmark before merge (Phase 6)
- Optimize query planner with EXPLAIN ANALYZE
- Consider separate search endpoints (768-dim, 1536-dim) if needed
- Rollback option: direct column access (breaking change)

### Risk 4: Provider API Changes Break Integration
**Mitigation:**
- Pin provider SDK versions (don't auto-update)
- Implement integration tests against real APIs
- Monitor provider changelogs and deprecation notices
- Have alert system for embedding failures

---

## Success Metrics

**Project succeeds if:**

**Functionality:**
- ✅ All three providers generate embeddings correctly
- ✅ Search works with all dimension types
- ✅ Mixed embeddings search returns relevant results
- ✅ Zero-config Ollama experience works out of the box

**Quality:**
- ✅ Zero data loss during migration (existing embeddings intact)
- ✅ No dimension mismatch errors post-launch
- ✅ Search latency regression <5%
- ✅ All security checklist items passed

**User Experience:**
- ✅ Documentation enables non-expert setup
- ✅ Error messages are clear and actionable
- ✅ Provider switching works first try for 95% of users

**Performance:**
- ✅ Ollama: 1,000 chunks in <5 minutes
- ✅ Google: 10,000 chunks in <2 minutes
- ✅ OpenAI: 10,000 chunks in <2 minutes
- ✅ Search: <100ms p95 latency for 25K chunks

---

## Timeline Summary

| Phase | Duration | Key Milestone |
|-------|----------|---------------|
| 0. Pre-Implementation | 0.5 days | Baselines established |
| 1. Database Migration | 1.5 days | 768-dim columns added |
| 2. Provider Abstraction | 3 days | Ollama + OpenAI providers |
| 3. Google Integration | 3 days | Google Vertex AI working |
| 4. Database Integration | 3 days | Search with mixed embeddings |
| 5. MCP & Docs | 3 days | Zero-config experience |
| 6. Testing & Validation | 2 days | All tests pass, benchmarks good |
| **Total** | **16 days** | **Ready for production** |

**Aggressive timeline:** 11 days (with specialized agents working in parallel)
**Conservative timeline:** 20 days (with iterations and rework)
**Recommended timeline:** 14-16 days (balanced, accounts for unknowns)
