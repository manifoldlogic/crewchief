# MPEMBED: Multi-Provider Embedding Support

**Project Status:** Design Complete, Ready for Implementation
**Timeline:** 2-3 weeks (14-16 days)
**Priority:** High (blocks semantic search functionality)

## Problem

Maproom's semantic code search has a **dimension mismatch crisis**: the database is hardcoded to 1536-dimensional vectors (OpenAI), but the current default provider (Ollama) generates 768-dimensional vectors. This results in **zero embeddings generated** across 23,632 indexed chunks, rendering semantic search completely non-functional.

Beyond fixing this immediate blocker, users need **provider choice** based on their priorities:
- **Privacy:** Local embeddings (Ollama) vs cloud (OpenAI, Google)
- **Cost:** Free (Ollama) vs paid ($0.10-0.20 per 100K chunks)
- **Speed:** 4.5 chunks/s (Ollama) vs 50-200 chunks/s (cloud)

## Solution

Implement multi-provider embedding support with three providers sharing two column sets:

| Provider | Dimensions | Database Columns | Speed | Cost |
|----------|------------|------------------|-------|------|
| **Ollama** (default) | 768 | `*_embedding_ollama` | ~4.5 chunks/s | $0 |
| **Google Vertex AI** | 768 | `*_embedding_ollama` (shared) | ~50-100 chunks/s | ~$0.10-0.20 |
| **OpenAI** | 1536 | `*_embedding` (existing) | ~50-200 chunks/s | ~$0.19 |

**Key insight:** Ollama and Google share dimensions, enabling **column sharing** and seamless provider switching without re-embedding.

**User experience:**
- **Zero-config default:** `npx -y @crewchief/maproom-mcp` works immediately with Ollama
- **Explicit opt-in:** Set `EMBEDDING_PROVIDER=google` or `=openai` for cloud providers
- **Backward compatible:** Existing OpenAI embeddings (1536-dim) preserved during migration

## Architecture Overview

```
┌───────────────────────────────────────────────────────────┐
│              EmbeddingProvider Trait (Rust)               │
│  - async fn embed(&self, text: &str) -> Vector           │
│  - fn dimension(&self) -> usize                           │
├─────────────┬─────────────────┬─────────────────────────┤
│ Ollama      │ Google Vertex   │ OpenAI                  │
│ Provider    │ Provider        │ Provider                │
│ (768-dim)   │ (768-dim)       │ (1536-dim)              │
└─────────────┴─────────────────┴─────────────────────────┘
                       ▼
┌───────────────────────────────────────────────────────────┐
│            PostgreSQL Database (pgvector)                 │
│  code_embedding_ollama vector(768)  ← Ollama + Google    │
│  text_embedding_ollama vector(768)  ← Ollama + Google    │
│  code_embedding        vector(1536) ← OpenAI             │
│  text_embedding        vector(1536) ← OpenAI             │
└───────────────────────────────────────────────────────────┘
                       ▼
┌───────────────────────────────────────────────────────────┐
│               Search Queries (COALESCE)                   │
│  COALESCE(code_embedding_ollama, code_embedding) <=> $1  │
│  → Automatically uses available embeddings               │
└───────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Database Migration (1.5 days)
- Add `*_embedding_ollama vector(768)` columns
- Create IVFFlat indexes for 768-dim vectors
- Preserve existing OpenAI embeddings (zero data loss)

**Agent:** migration-safety-specialist, database-engineer

### Phase 2: Provider Abstraction (3 days)
- Define `EmbeddingProvider` trait in Rust
- Implement OllamaProvider (768-dim)
- Refactor OpenAIClient to implement trait (1536-dim)
- Create provider factory with auto-detection

**Agents:** provider-abstraction-architect, rust-indexer-engineer, embeddings-engineer

### Phase 3: Google Vertex AI Integration (3 days)
- Implement GoogleProvider with GCP authentication
- Support regional endpoints and task types
- Integration tests with real GCP project
- Documentation: IAM setup, service accounts

**Agent:** google-cloud-integration-engineer

### Phase 4: Database Integration (3 days)
- Column selection logic (dimension → column name)
- Update `upsert_embeddings()` for dimension parameter
- Search queries with COALESCE pattern
- Test mixed embeddings (768 + 1536)

**Agents:** database-engineer, vector-database-engineer, embeddings-engineer

### Phase 5: MCP Integration & Docs (3 days)
- MCP TypeScript wrapper: provider detection
- CLI flags: `--provider` option
- Documentation: provider comparison, setup guides, migration guide
- README updates

**Agents:** mcp-tools-engineer, rust-indexer-engineer

### Phase 6: Testing & Validation (2 days)
- Contract tests (provider interface)
- End-to-end tests (scan → embed → search)
- Performance benchmarks (search latency, throughput)
- Security audit (API key handling, SQL injection)

**Agents:** contract-test-engineer, integration-tester, performance-engineer

**Total timeline:** 14-16 days (2-3 weeks)

## Relevant Agents

### Existing Agents
- **database-engineer**: Database migrations, query optimization
- **rust-indexer-engineer**: Rust embedding service refactor
- **embeddings-engineer**: Embedding pipeline modifications
- **vector-database-engineer**: pgvector optimization, IVFFlat tuning
- **mcp-tools-engineer**: TypeScript MCP wrapper updates
- **integration-tester**: End-to-end workflow tests
- **contract-test-engineer**: Provider interface contract tests
- **performance-engineer**: Search latency benchmarks

### New Agents (Recommended)
- **google-cloud-integration-engineer**: GCP Vertex AI implementation (3-4 days)
- **provider-abstraction-architect**: Rust trait design, extensibility (2-3 days)
- **migration-safety-specialist**: Production-safe database migrations (1-2 days)

**Time savings with specialized agents:** 40-45% faster (9-14 days saved)

## Success Criteria

**Functionality:**
- ✅ All three providers generate embeddings correctly
- ✅ Search works with 768-dim and 1536-dim queries
- ✅ Mixed embeddings (OpenAI + Ollama) search returns relevant results
- ✅ Zero-config Ollama experience works out of the box

**Quality:**
- ✅ Zero data loss during migration (existing embeddings preserved)
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

## Project Documents

- **[MPEMBED_ANALYSIS.md](./MPEMBED_ANALYSIS.md)**: Deep dive into problem space, industry solutions, user personas, and research findings
- **[MPEMBED_ARCHITECTURE.md](./MPEMBED_ARCHITECTURE.md)**: Technical design, component diagrams, database schema, provider abstraction, trade-offs
- **[MPEMBED_QUALITY_STRATEGY.md](./MPEMBED_QUALITY_STRATEGY.md)**: Testing philosophy, risk-based prioritization, test levels, contract tests
- **[MPEMBED_SECURITY_REVIEW.md](./MPEMBED_SECURITY_REVIEW.md)**: Threat model, API key management, data privacy, input validation, security gaps
- **[MPEMBED_AGENT_SUGGESTIONS.md](./MPEMBED_AGENT_SUGGESTIONS.md)**: Specialized agents needed, skills required, budget allocation
- **[MPEMBED_PLAN.md](./MPEMBED_PLAN.md)**: 6-phase implementation plan with tasks, deliverables, dependencies, success criteria

## Key Technical Decisions

### 1. Column Sharing Strategy
**Decision:** Ollama + Google share `*_embedding_ollama` columns (both 768-dim)

**Rationale:**
- Reduces storage overhead (2 column sets instead of 3)
- Enables seamless switching between Ollama ↔ Google
- Most users will use ONE provider (not multiple simultaneously)

**Trade-off:** Cannot have both Ollama AND Google embeddings simultaneously (acceptable, they serve different use cases)

### 2. Provider Abstraction Pattern
**Decision:** Trait-based dispatch (`Box<dyn EmbeddingProvider>`)

**Rationale:**
- Extensible: Add new providers without touching core service
- Clean separation: Each provider is self-contained module
- Testable: Easy to mock providers for tests

**Trade-off:** Dynamic dispatch overhead (negligible for I/O-bound embedding calls)

### 3. Auto-Detection vs Explicit Config
**Decision:** Auto-detect Ollama, fall back to explicit `EMBEDDING_PROVIDER` env var

**Rationale:**
- Best zero-config experience (Ollama "just works" if installed)
- Users can override if Ollama running but they want cloud provider

**Trade-off:** Adds network call to localhost:11434 at startup (2-second timeout, acceptable)

### 4. COALESCE Preference Order
**Decision:** Prefer 768-dim over 1536-dim in search queries

```sql
COALESCE(code_embedding_ollama, code_embedding) <=> $1
```

**Rationale:**
- Prioritizes local/cheaper providers (Ollama/Google) over OpenAI
- Encourages migration away from OpenAI if both exist

**Trade-off:** Users who prefer OpenAI quality should avoid generating 768-dim embeddings

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Migration fails in production | High | Test on staging (25K chunks), rollback SQL ready |
| Google auth complexity blocks users | Medium | Step-by-step guide, Ollama remains default |
| Performance regression from COALESCE | Medium | Benchmark before merge, optimize query planner |
| Provider API changes break integration | Low | Pin SDK versions, monitor changelogs |

## Security Considerations

**Critical safeguards:**
- API keys never logged or exposed in error messages
- Service account files checked for permissions (warn if >600)
- All database queries use parameterized inputs (SQL injection prevention)
- HTTPS enforced for cloud providers (OpenAI, Google)
- Model names validated against allowlist/regex (command injection prevention)

**Known limitations (documented for enterprise):**
- No automatic key rotation (manual process)
- No rate limiting (rely on provider limits)
- No audit logging (standard app logs only)
- No encryption at rest (database-level access controls)
- Environment variables for secrets (not secret managers)

**Compliance notes:**
- **GDPR:** Ollama (local) or Google Vertex AI (EU regions) for EU citizen code
- **HIPAA:** Ollama only, or Google Cloud with BAA (OpenAI not compliant)
- **SOC 2:** Document limitations, point to enterprise solutions (audit logging, key rotation)

## Getting Started

**After implementation, users will:**

1. **Install with zero config (Ollama):**
   ```bash
   npx -y @crewchief/maproom-mcp
   # Automatically uses Ollama if available on localhost:11434
   ```

2. **Switch to Google Vertex AI:**
   ```bash
   export EMBEDDING_PROVIDER=google
   export GOOGLE_PROJECT_ID=my-project-123
   export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
   npx -y @crewchief/maproom-mcp
   ```

3. **Use OpenAI:**
   ```bash
   export EMBEDDING_PROVIDER=openai
   export OPENAI_API_KEY=sk-...
   npx -y @crewchief/maproom-mcp
   ```

**Migration for existing OpenAI users:**
- Existing 1536-dim embeddings preserved automatically
- No re-indexing required
- Can switch to Ollama/Google incrementally (new files use new provider)

## Related Context

- **Original context doc:** [crewchief_context/maproom/multi-provider-embedding-support.md](../multi-provider-embedding-support.md)
- **Existing work:** LOCAL-5005 (auto-embedding generation, completed)
- **Blocker:** Dimension mismatch (768 vs 1536) preventing all embedding generation
- **Current state:** 23,632 chunks indexed, 0 embeddings generated

## Questions or Feedback?

This is a design document. Actual implementation will follow the phased plan in MPEMBED_PLAN.md.

**For implementation questions:**
- Database concerns → database-engineer, migration-safety-specialist
- Rust provider design → provider-abstraction-architect, rust-indexer-engineer
- Google Cloud setup → google-cloud-integration-engineer
- Testing strategy → integration-tester, contract-test-engineer
