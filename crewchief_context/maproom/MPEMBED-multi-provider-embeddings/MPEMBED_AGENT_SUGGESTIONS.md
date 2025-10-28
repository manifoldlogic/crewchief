# MPEMBED: Agent Suggestions

## Overview

This document identifies specialized agents that would help complete the multi-provider embedding support project. Most work can be accomplished with existing agents, but a few specialized capabilities would significantly improve efficiency.

## Existing Agents (Sufficient for Most Work)

The following existing agents are well-suited for this project:

- **database-engineer**: Database migration (add 768-dim columns), query optimization (COALESCE pattern)
- **rust-indexer-engineer**: Rust embedding service refactor, provider abstraction implementation
- **embeddings-engineer**: Embedding pipeline modifications, provider integration
- **integration-tester**: E2E tests for scan → embed → search workflows
- **contract-test-engineer**: Provider interface contract tests
- **performance-engineer**: Benchmark COALESCE queries, measure embedding throughput
- **vector-database-engineer**: IVFFlat index tuning, dimension handling optimization
- **mcp-tools-engineer**: TypeScript wrapper updates, provider detection logic

## New Agent Suggestions

### 1. google-cloud-integration-engineer

**Why needed:**
- Google Vertex AI integration requires **specialized GCP knowledge** (IAM, service accounts, gRPC)
- Different from generic API integration (OAuth2 flows, regional endpoints, IAM policies)
- Expertise in `google-cloud-auth` crate and Cloud Client Libraries

**Responsibilities:**
- Implement `GoogleProvider` struct with Vertex AI predict endpoint calls
- Handle service account authentication (JSON key file, Workload Identity)
- Configure task types (RETRIEVAL_DOCUMENT vs RETRIEVAL_QUERY)
- Implement regional endpoint routing (us-central1, europe-west1, asia-southeast1)
- Write GCP-specific integration tests (require service account credentials)
- Document IAM permissions (least-privilege roles)

**Skills:**
- Google Cloud Platform (Vertex AI, IAM, service accounts)
- gRPC and protobuf (Vertex AI uses gRPC internally)
- Rust async/await patterns for HTTP clients
- OAuth2 and JWT token handling

**Deliverables:**
- `crates/maproom/src/embedding/google.rs` implementation
- Integration tests for Google provider
- Documentation: "Google Vertex AI Setup Guide"

**Rationale:**
While generic Rust engineers can implement HTTP clients, GCP's authentication model (service accounts, workload identity, IAM roles) and regional architecture require domain expertise. Without this agent, expect 2-3x longer implementation time and higher likelihood of security misconfigurations.

---

### 2. provider-abstraction-architect

**Why needed:**
- Trait-based provider abstraction is **critical architectural decision** that affects extensibility
- Requires balancing flexibility (easy to add providers) vs simplicity (not over-engineered)
- Rust trait design for async I/O is nuanced (async-trait crate, object safety, dynamic dispatch)

**Responsibilities:**
- Design `EmbeddingProvider` trait with optimal method signatures
- Decide on trait objects (`Box<dyn EmbeddingProvider>`) vs generics vs enum dispatch
- Handle provider-specific configuration (OpenAI API key vs Google service account)
- Design provider factory with graceful fallbacks (Ollama auto-detect → config → error)
- Ensure trait is testable (easy to mock providers for tests)
- Document extension points for future providers

**Skills:**
- Rust trait design and object-oriented patterns
- Async Rust (tokio, async-trait, futures)
- API design and extensibility
- Performance trade-offs (dynamic dispatch vs monomorphization)

**Deliverables:**
- `crates/maproom/src/embedding/provider.rs` trait definition
- `crates/maproom/src/embedding/factory.rs` provider construction logic
- Design document: "Provider Abstraction Design Rationale"
- Example: "Adding a New Provider (HuggingFace)" guide

**Rationale:**
Poor abstraction design leads to rigid systems that resist future changes. This agent ensures the trait is **just right**: not too abstract (boilerplate hell), not too concrete (can't add Cohere/Anthropic later). The existing `embeddings-engineer` agent focuses on embedding pipelines, not Rust API design.

---

### 3. migration-safety-specialist

**Why needed:**
- Database migration must **preserve existing OpenAI embeddings** (24K+ chunks)
- Rollback strategy required if migration fails mid-way
- Schema changes in production require careful planning (downtime, locking, verification)

**Responsibilities:**
- Write idempotent migration SQL (safe to run multiple times)
- Add rollback migration (drop new columns if needed)
- Create pre-migration backup script
- Design verification queries (check column counts, embedding preservation)
- Test migration on production-size fixtures (25K+ chunks)
- Document migration runbook (steps, rollback procedure, verification)

**Skills:**
- PostgreSQL schema migrations and pgvector
- Production database operations (zero-downtime deployments, locking strategies)
- Disaster recovery planning
- SQL performance (index creation time, ALTER TABLE impacts)

**Deliverables:**
- `crates/maproom/migrations/0015_add_ollama_columns.sql` (migration)
- `crates/maproom/migrations/0015_add_ollama_columns_rollback.sql` (rollback)
- `scripts/verify_migration.sh` (post-migration checks)
- Runbook: "Production Migration Guide"

**Rationale:**
The existing `database-engineer` agent can write migrations, but **production migration safety** requires specialized knowledge: transaction boundaries, lock timeouts, index creation strategies, rollback procedures. This agent ensures migrations are production-ready, not just "works on my machine."

---

## Why Not More Agents?

**Considered but rejected:**

**ollama-integration-specialist**: Unnecessary
- Ollama API is simple HTTP POST (already implemented in codebase)
- No special expertise needed beyond HTTP client usage
- Existing `rust-indexer-engineer` can handle Ollama refactor

**openai-refactor-specialist**: Unnecessary
- OpenAI client already exists and works
- Refactor is straightforward (wrap in trait implementation)
- Existing `embeddings-engineer` can handle this

**coalesce-query-optimizer**: Unnecessary
- COALESCE pattern is well-documented PostgreSQL feature
- Existing `database-engineer` + `vector-database-engineer` have sufficient expertise
- Performance testing covered by `performance-engineer`

**documentation-writer**: Unnecessary
- Documentation is integrated into each agent's deliverables
- Technical writers add overhead for small project (overkill for MVP)
- Code comments + README + ADRs sufficient

## Agent Assignment Strategy

**Phase-based assignment:**

**Phase 1 (Database Migration):**
- Primary: `migration-safety-specialist` (migration SQL, rollback, verification)
- Support: `database-engineer` (index tuning, pgvector optimization)

**Phase 2 (Provider Abstraction):**
- Primary: `provider-abstraction-architect` (trait design, factory)
- Support: `rust-indexer-engineer` (refactor existing OpenAI client)

**Phase 3 (Google Integration):**
- Primary: `google-cloud-integration-engineer` (Vertex AI implementation)
- Support: `embeddings-engineer` (integrate with embedding pipeline)

**Phase 4 (Testing):**
- Primary: `integration-tester` (E2E workflows)
- Support: `contract-test-engineer` (provider contract tests)
- Support: `performance-engineer` (benchmarks)

**Phase 5 (MCP Integration):**
- Primary: `mcp-tools-engineer` (TypeScript wrapper, provider detection)
- Support: `rust-indexer-engineer` (CLI flag additions)

## Measuring Agent Effectiveness

**Success criteria for new agents:**

**google-cloud-integration-engineer:**
- ✅ Google provider works first try on test GCP project
- ✅ IAM permissions follow least-privilege principle
- ✅ Documentation enables non-GCP-expert to set up
- ✅ No security vulnerabilities in service account handling

**provider-abstraction-architect:**
- ✅ Adding a fourth provider (e.g., Cohere) requires <100 lines of code
- ✅ Trait is object-safe and easy to mock in tests
- ✅ No performance regressions vs direct OpenAI client
- ✅ Other developers understand trait design from documentation

**migration-safety-specialist:**
- ✅ Migration runs on 25K chunk fixture in <2 minutes
- ✅ Zero data loss (all existing embeddings preserved)
- ✅ Rollback script successfully undoes migration
- ✅ Production runbook enables DBA to execute safely

## Budget Allocation

**Estimated effort by agent:**

| Agent | Effort | Justification |
|-------|--------|---------------|
| google-cloud-integration-engineer | 3-4 days | GCP auth complexity, regional endpoints, testing |
| provider-abstraction-architect | 2-3 days | Trait design, refactor existing code, extensibility |
| migration-safety-specialist | 1-2 days | Migration SQL, rollback, verification scripts, runbook |
| **Total (new agents)** | **6-9 days** | |
| Existing agents | 5-7 days | Database queries, MCP integration, testing |
| **Project total** | **11-16 days** | 2-3 week timeline |

**Cost-benefit:**
- Without specialized agents: 20-25 days (generic agents fumbling through GCP, trait design, production migrations)
- With specialized agents: 11-16 days (focused expertise, fewer iterations)
- **Time saved: 40-45%** (9-14 days faster)

## Hiring Criteria

If creating these agents:

**google-cloud-integration-engineer:**
- Must have: Production GCP experience, Vertex AI usage, IAM expertise
- Nice to have: Rust experience (can learn), multi-region deployments

**provider-abstraction-architect:**
- Must have: Rust trait design, async programming, API design patterns
- Nice to have: Embeddings domain knowledge (OpenAI/Ollama APIs)

**migration-safety-specialist:**
- Must have: PostgreSQL production operations, pgvector experience, zero-downtime migrations
- Nice to have: Disaster recovery experience, large-scale databases (>1M rows)

## Conclusion

This project can succeed with existing agents alone, but three specialized agents would significantly improve:
1. **Quality**: GCP security best practices, trait extensibility, migration safety
2. **Speed**: Domain expertise reduces trial-and-error iterations
3. **Maintainability**: Well-designed abstractions ease future provider additions

**Recommendation**: Create all three agents if timeline is critical (<2 weeks). For relaxed timeline (3-4 weeks), existing agents sufficient with more iterations.
