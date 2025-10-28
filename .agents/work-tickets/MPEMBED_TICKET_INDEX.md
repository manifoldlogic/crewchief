# MPEMBED Ticket Index

**Project**: Multi-Provider Embedding Support
**Project Folder**: `/workspace/crewchief_context/maproom/MPEMBED-multi-provider-embeddings/`
**Plan**: `MPEMBED_PLAN.md`
**Total Tickets**: 32
**Estimated Duration**: 14-16 days

## Project Overview

This project adds support for multiple embedding providers (Ollama, Google Vertex AI) alongside the existing OpenAI provider. The core problem is a dimension mismatch: production database has 23,632 chunks but 0 embeddings because Ollama generates 768-dim vectors but the schema expects 1536-dim.

### Solution Architecture

- **Database**: Add 768-dim columns (`*_embedding_ollama`) alongside existing 1536-dim columns
- **Provider Abstraction**: Rust trait `EmbeddingProvider` for dynamic dispatch
- **Column Sharing**: Ollama (768-dim) and Google (768-dim) share columns
- **Search**: COALESCE pattern for mixed embedding queries
- **Zero-Config UX**: Auto-detect Ollama, explicit config for cloud providers

---

## Phase 0: Pre-Implementation Setup (0.5 days)

**Goal**: Establish baselines and prepare test fixtures

| Ticket | Title | Agent | Dependencies |
|--------|-------|-------|--------------|
| **MPEMBED-0001** | Create 100-chunk database test fixture | database-engineer | None |
| **MPEMBED-0002** | Establish performance baselines | performance-engineer | 0001 |
| **MPEMBED-0003** | Audit and update dependencies | rust-indexer-engineer | None |

**Deliverables**:
- 100-chunk test fixture (~5s load time)
- Performance baseline metrics (search latency, index sizes, throughput)
- Dependencies: `async-trait`, `google-cloud-auth` added

---

## Phase 1: Database Migration (1.5 days)

**Goal**: Add 768-dim columns without breaking existing functionality

| Ticket | Title | Agent | Dependencies |
|--------|-------|-------|--------------|
| **MPEMBED-1001** | Write idempotent SQL migration | migration-safety-specialist | 0001 |
| **MPEMBED-1002** | Write rollback SQL migration | migration-safety-specialist | 1001 |
| **MPEMBED-1003** | Create migration verification script | migration-safety-specialist | 1001 |
| **MPEMBED-1901** | Test migration on fixture | integration-tester | 0001, 1001, 1002, 1003 |

**Deliverables**:
- Migration SQL: `migrations/0015_add_ollama_columns.sql`
- Rollback SQL: `migrations/0015_add_ollama_columns_rollback.sql`
- Verification script: `scripts/verify_migration_0015.sh`
- Integration tests confirming zero data loss

**Critical Requirements**:
- Zero data loss (preserve all 23,632 OpenAI embeddings)
- Zero downtime (CREATE INDEX CONCURRENTLY)
- Idempotent (safe to run multiple times)
- Tested rollback procedures

---

## Phase 2: Provider Abstraction (3 days)

**Goal**: Refactor embedding service to support multiple providers via trait

| Ticket | Title | Agent | Dependencies |
|--------|-------|-------|--------------|
| **MPEMBED-2001** | Define EmbeddingProvider trait | provider-abstraction-architect | 0003 |
| **MPEMBED-2002** | Implement OllamaProvider | rust-indexer-engineer | 2001 |
| **MPEMBED-2003** | Refactor OpenAIClient to implement trait | embeddings-engineer | 2001 |
| **MPEMBED-2004** | Implement provider factory | provider-abstraction-architect | 2001, 2002, 2003 |
| **MPEMBED-2005** | Refactor EmbeddingService | embeddings-engineer | 2004 |
| **MPEMBED-2901** | Test provider abstraction | contract-test-engineer | 2001-2005 |

**Deliverables**:
- Provider trait: `crates/maproom/src/embedding/provider.rs`
- Ollama provider: `crates/maproom/src/embedding/ollama.rs`
- OpenAI trait implementation: `crates/maproom/src/embedding/openai.rs` (modified)
- Factory: `crates/maproom/src/embedding/factory.rs`
- Refactored service: `crates/maproom/src/embedding/service.rs`

**Key Design Decisions**:
- Object-safe trait with `&self` methods
- `async-trait` for async support
- Dynamic dispatch via `Box<dyn EmbeddingProvider>`
- Auto-detect Ollama (2s timeout), explicit config for cloud

---

## Phase 3: Google Vertex AI Integration (3 days)

**Goal**: Add Google Cloud Vertex AI as third provider option

| Ticket | Title | Agent | Dependencies |
|--------|-------|-------|--------------|
| **MPEMBED-3001** | Implement GoogleProvider | google-cloud-integration-engineer | 2001 |
| **MPEMBED-3002** | Add Google to factory | provider-abstraction-architect | 3001, 2004 |
| **MPEMBED-3003** | Google integration tests | google-cloud-integration-engineer | 3001 |
| **MPEMBED-3004** | Google setup documentation | google-cloud-integration-engineer | 3001 |

**Deliverables**:
- Google provider: `crates/maproom/src/embedding/google.rs`
- Factory updated: `crates/maproom/src/embedding/factory.rs`
- Integration tests: `crates/maproom/tests/google_provider_integration.rs`
- Setup guide: `docs/providers/google-vertex-ai-setup.md`

**Security Requirements**:
- Service account authentication (JSON key file)
- Least-privilege IAM roles (`roles/aiplatform.user`)
- No credential exposure in logs or errors

---

## Phase 4: Database and Search Integration (3 days)

**Goal**: Update database operations and search queries for multi-dimension support

| Ticket | Title | Agent | Dependencies |
|--------|-------|-------|--------------|
| **MPEMBED-4001** | Column selection logic | database-engineer | 1001 |
| **MPEMBED-4002** | Update embedding upsert | database-engineer | 4001 |
| **MPEMBED-4003** | Update search queries | vector-database-engineer | 4001 |
| **MPEMBED-4004** | Integrate embedding pipeline | embeddings-engineer | 4002, 2005 |
| **MPEMBED-4901** | Test mixed embeddings | integration-tester | 4003 |

**Deliverables**:
- Column selection: `crates/maproom/src/db/columns.rs`
- Updated upsert: `crates/maproom/src/db/chunks.rs`
- COALESCE search: `crates/maproom/src/search/hybrid.rs`
- Pipeline integration: `crates/maproom/src/embedding/pipeline.rs`
- Mixed embedding tests: `crates/maproom/tests/mixed_embeddings_search.rs`

**Key Patterns**:
```rust
// Column selection
match dimension {
    768 => ("code_embedding_ollama", "text_embedding_ollama"),
    1536 => ("code_embedding", "text_embedding"),
    _ => Err(DbError::InvalidDimension(dimension)),
}
```

```sql
-- COALESCE search pattern (prefer 768-dim over 1536-dim)
SELECT id, symbol_name,
  COALESCE(
    1 - (code_embedding_ollama <=> $1),
    1 - (code_embedding <=> $2)
  ) as similarity
FROM maproom.chunks
WHERE code_embedding_ollama IS NOT NULL OR code_embedding IS NOT NULL
ORDER BY similarity DESC LIMIT 10;
```

---

## Phase 5: MCP Integration and Documentation (3 days)

**Goal**: Update MCP TypeScript wrapper and complete user-facing documentation

| Ticket | Title | Agent | Dependencies |
|--------|-------|-------|--------------|
| **MPEMBED-5001** | Provider detection in MCP | mcp-tools-engineer | None |
| **MPEMBED-5002** | Update MCP tools | mcp-tools-engineer | 5001 |
| **MPEMBED-5003** | CLI provider flag | rust-indexer-engineer | 2004 |
| **MPEMBED-5004** | Provider comparison docs | mcp-tools-engineer | 3004 |
| **MPEMBED-5005** | Setup guides | mcp-tools-engineer | 5004 |
| **MPEMBED-5006** | Migration guide | mcp-tools-engineer | 5005 |
| **MPEMBED-5007** | README updates | mcp-tools-engineer | 5006 |

**Deliverables**:
- Provider detection: `packages/maproom-mcp/src/utils/provider-detection.ts`
- MCP tools updated: `packages/maproom-mcp/src/tools/scan.ts`, `upsert.ts`
- CLI flags: `crates/maproom/src/main.rs`
- Documentation:
  - `docs/providers/comparison.md`
  - `docs/providers/ollama-setup.md`
  - `docs/providers/openai-setup.md`
  - `docs/guides/provider-migration.md`
  - `README.md` (updated)

**Provider Comparison**:

| Feature | Ollama | Google Vertex AI | OpenAI |
|---------|--------|------------------|--------|
| Cost | $0 | ~$0.10-0.20 per 100K chunks | ~$0.19 per 100K chunks |
| Speed | ~4.5 chunks/s | ~50-100 chunks/s | ~50-200 chunks/s |
| Privacy | 100% local | Cloud (Google) | Cloud (OpenAI) |
| Setup | Zero config | Service account | API key |
| Compliance | GDPR-friendly | BAA available | Not HIPAA |

---

## Phase 6: Testing and Validation (2 days)

**Goal**: Comprehensive testing and performance validation

| Ticket | Title | Agent | Dependencies |
|--------|-------|-------|--------------|
| **MPEMBED-6001** | Contract tests | contract-test-engineer | 2001, 2002, 2003, 3001 |
| **MPEMBED-6002** | E2E tests | integration-tester | 4004, 5003 |
| **MPEMBED-6003** | Performance benchmarks | performance-engineer | 0002, 6002 |
| **MPEMBED-6901** | Manual testing | verify-ticket | All phases complete |

**Deliverables**:
- Contract tests: `crates/maproom/tests/provider_contract.rs`
- E2E tests: `crates/maproom/tests/e2e_multi_provider.rs`
- Benchmarks: `benchmarks/multi_provider_performance.md`
- Manual testing checklist: `tests/manual/mpembed_checklist.md`

**Test Scenarios**:
1. Scan with Ollama → Search with 768-dim
2. Scan with Google → Search with 768-dim
3. Scan with OpenAI → Search with 1536-dim
4. Mixed embeddings (OpenAI + Ollama) → Search finds both

**Performance Targets**:
- Search latency regression: <5% (from baseline)
- Ollama throughput: 1,000 chunks in <5 minutes
- Google throughput: 10,000 chunks in <2 minutes
- OpenAI throughput: 10,000 chunks in <2 minutes

---

## Success Metrics

**Project succeeds if**:

### Functionality
- ✅ All three providers generate embeddings correctly
- ✅ Search works with all dimension types (768, 1536)
- ✅ Mixed embeddings search returns relevant results
- ✅ Zero-config Ollama experience works out of the box

### Quality
- ✅ Zero data loss during migration (existing embeddings intact)
- ✅ No dimension mismatch errors post-launch
- ✅ Search latency regression <5%
- ✅ All security checklist items passed

### User Experience
- ✅ Documentation enables non-expert setup
- ✅ Error messages are clear and actionable
- ✅ Provider switching works first try for 95% of users

### Performance
- ✅ Ollama: 1,000 chunks in <5 minutes
- ✅ Google: 10,000 chunks in <2 minutes
- ✅ OpenAI: 10,000 chunks in <2 minutes
- ✅ Search: <100ms p95 latency for 25K chunks

---

## Execution Recommendations

### Sequential Phases
Execute phases 0-6 in order. Each phase depends on the previous.

### Parallel Work Within Phases
Within a phase, tickets can be parallelized when dependencies allow:
- **Phase 2**: 2002 (Ollama) and 2003 (OpenAI) can run in parallel after 2001 completes
- **Phase 3**: 3003 (tests) and 3004 (docs) can run in parallel with 3001
- **Phase 5**: Documentation tickets (5004-5007) can run in parallel

### Critical Path
The critical path determines minimum project duration:
```
0001 → 1001 → 1002 → 1003 → 1901 →
2001 → 2002 → 2004 → 2005 →
3001 → 3002 →
4001 → 4002 → 4004 →
5003 →
6002 → 6003 → 6901
```

**Minimum duration**: 14 days (with perfect parallelization)
**Realistic duration**: 16 days (accounting for rework and unknowns)

### Agent Allocation

**Primary agents needed**:
- migration-safety-specialist (Phase 1)
- provider-abstraction-architect (Phases 2, 3)
- google-cloud-integration-engineer (Phase 3)
- database-engineer (Phases 0, 1, 4)
- vector-database-engineer (Phase 4)
- embeddings-engineer (Phases 2, 4)
- rust-indexer-engineer (Phases 0, 2, 5)
- mcp-tools-engineer (Phase 5)
- contract-test-engineer (Phases 2, 6)
- integration-tester (Phases 1, 4, 6)
- performance-engineer (Phases 0, 6)

---

## Risk Management

### High-Priority Risks

**1. Migration fails in production**
- **Mitigation**: Test on staging (25K chunk replica), rollback SQL tested, low-traffic deployment window

**2. COALESCE performance regression**
- **Mitigation**: Benchmark in Phase 6, optimize query planner with EXPLAIN ANALYZE, rollback option

**3. Google Auth complexity blocks users**
- **Mitigation**: Step-by-step guide with screenshots, Ollama remains zero-config default

**4. Provider API changes break integration**
- **Mitigation**: Pin SDK versions, integration tests against real APIs, monitor changelogs

### Medium-Priority Risks

**5. Dimension mismatch errors post-launch**
- **Mitigation**: Column selection logic validated, dimension checks in pipeline, extensive testing

**6. Ollama concurrent requests overwhelm system**
- **Mitigation**: Semaphore limits concurrency to 10, timeout set to 30s

---

## References

### Project Documents
- **Project Folder**: `/workspace/crewchief_context/maproom/MPEMBED-multi-provider-embeddings/`
- **Analysis**: `MPEMBED_ANALYSIS.md`
- **Architecture**: `MPEMBED_ARCHITECTURE.md`
- **Plan**: `MPEMBED_PLAN.md`
- **Quality Strategy**: `MPEMBED_QUALITY_STRATEGY.md`
- **Security Review**: `MPEMBED_SECURITY_REVIEW.md`
- **Agent Suggestions**: `MPEMBED_AGENT_SUGGESTIONS.md`

### Related Documentation
- **Original Spec**: `/workspace/crewchief_context/maproom/multi-provider-embedding-support.md`
- **Maproom Docs**: `/workspace/docs/maproom/`
- **Database Schema**: `/workspace/crates/maproom/migrations/`

---

## Notes

### Test Ticket Numbering
Test tickets use the 900s range within each phase:
- **1901**: Phase 1 test (migration on fixture)
- **2901**: Phase 2 test (provider abstraction)
- **4901**: Phase 4 test (mixed embeddings)
- **6901**: Phase 6 manual test (production readiness)

### Ticket Workflow
Each ticket follows the standard workflow:
1. **Implementation**: Primary agent marks "Task completed"
2. **Testing**: test-runner executes relevant tests
3. **Verification**: verify-ticket agent checks acceptance criteria
4. **Commit**: commit-ticket agent creates commit with Conventional Commit message

### Context Preservation
All tickets reference the project plan and supporting documents, ensuring agents have complete context for implementation decisions.

---

**Last Updated**: 2025-10-28
**Status**: All 32 tickets created, ready for execution
**Next Step**: Begin Phase 0 with MPEMBED-0001
