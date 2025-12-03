# Project: Embedding Dimension 1024

**Slug:** DIM1024
**Status:** Planning
**Created:** 2025-12-03

## Summary

Add 1024-dimensional embedding support to enable mxbai-embed-large model, which handles all content types without tokenization crashes. This is a focused fix to address nomic-embed-text's GGML tokenization bugs that crash on certain characters (|, [], (), Unicode symbols).

**Key Benefits:**
- No content mangling (code stays intact for embedding)
- Better embedding quality (mxbai-embed-large outperforms nomic-embed-text)
- Clean fix vs workaround (removes 40+ lines of character sanitization code)
- Future-proof (easy to add more dimensions)

**Acceptable Tradeoffs:**
- Slightly larger model (670MB vs 274MB)
- ~30% more storage per embedding (1024 vs 768 floats)
- Need to re-embed existing content

## Problem Statement

The Ollama nomic-embed-text model (768-dim) crashes when processing certain characters due to GGML tokenization bugs. The current workaround replaces problematic characters before embedding, which mangles code content and reduces embedding quality.

mxbai-embed-large handles all content perfectly but produces 1024-dimensional embeddings, which the database currently doesn't support (only 768 and 1536).

## Proposed Solution

Follow the established pattern from Migration #7 (which added 768-dim support):

1. **Add Migration #10**: Create `vec_code_1024` virtual table
2. **Update dimension mappings**: Add 1024 → vec_code_1024 in embeddings.rs, vector.rs, columns.rs
3. **Make Ollama dimension configurable**: Remove hardcoded dimension=768, allow configuration
4. **Conditional sanitization**: Apply character replacement only for nomic-embed-text, not mxbai-embed-large
5. **Testing**: Unit, integration, and E2E tests for 1024-dim support
6. **Documentation**: Configuration examples for mxbai-embed-large

**Configuration:**
```bash
MAPROOM_EMBEDDING_MODEL=mxbai-embed-large
MAPROOM_EMBEDDING_DIMENSION=1024
```

## Relevant Agents

**Planning & Coordination:**
- project-planner (planning phase) - Complete
- ticket-creator (ticket generation) - Next step
- project-orchestrator (coordination)

**Implementation:**
- rust-developer (Rust code changes)
- unit-test-runner (test execution)
- documentation-writer (user/dev docs)

**Verification & Commit:**
- verify-ticket (acceptance criteria)
- commit-ticket (atomic commits)

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis, context, constraints, success criteria
- [architecture.md](planning/architecture.md) - Solution design, components, data flow, performance
- [plan.md](planning/plan.md) - 4-phase execution plan, risks, timeline (9-13 hours)
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach, critical paths, quality gates
- [security-review.md](planning/security-review.md) - Security assessment (no meaningful concerns)

## Project Phases

### Phase 1: Database Foundation (2-3 hours)
- Migration #10: Create vec_code_1024 table
- Update SUPPORTED_DIMENSIONS constants
- Add dimension mapping cases
- Unit tests for storage and sync

### Phase 2: Provider Configuration (3-4 hours)
- Make OllamaProvider dimension-aware
- Remove hardcoded dimension=768
- Update config validation
- Unit tests for configurability

### Phase 3: Sanitization Cleanup (1-2 hours)
- Conditional sanitization based on model
- Extract sanitization into helper function
- Preserve for nomic-embed-text, skip for mxbai-embed-large
- Integration tests with problematic characters

### Phase 4: Testing and Documentation (3-4 hours)
- E2E tests (embedding generation, search, mixed dimensions)
- Integration tests (migration idempotency)
- Update ollama-setup.md and CLAUDE.md
- Configuration examples

**Total Estimate:** 9-13 hours

## Key Design Decisions

1. **Follow existing pattern**: Use virtual table per dimension (vec_code_1024) like vec_code_768
2. **Make dimension configurable**: Remove model-specific validation, allow flexibility
3. **Keep backward compatibility**: Preserve sanitization for nomic-embed-text users
4. **Extend consistently**: Update all three dimension mapping locations (embeddings.rs, vector.rs, columns.rs)

## Success Criteria

**Functional:**
- [ ] 1024-dim embeddings can be stored and searched
- [ ] mxbai-embed-large configurable via environment variables
- [ ] No crashes on |, [], (), Unicode characters
- [ ] No content mangling (sanitization removed for mxbai)

**Quality:**
- [ ] All unit tests pass (including new 1024-dim tests)
- [ ] Mixed dimensions coexist (768, 1024, 1536)
- [ ] Migration #10 is idempotent
- [ ] Backward compatibility maintained

**Documentation:**
- [ ] Configuration examples for mxbai-embed-large
- [ ] Storage/performance tradeoffs documented

## Tickets

Ticket generation pending. Run `/workstream:project-tickets DIM1024` to create tickets.

**Expected tickets:**
- DIM1024-1001: Database Foundation (Phase 1)
- DIM1024-2001: Provider Configuration (Phase 2)
- DIM1024-2002: Sanitization Cleanup (Phase 3)
- DIM1024-3001: Testing and Documentation (Phase 4)

## Files to Modify

**Database:**
- `/workspace/crates/maproom/src/db/sqlite/migrations.rs` (add Migration #10)
- `/workspace/crates/maproom/src/db/sqlite/embeddings.rs` (dimension mapping)
- `/workspace/crates/maproom/src/db/sqlite/vector.rs` (dimension mapping)
- `/workspace/crates/maproom/src/db/columns.rs` (dimension mapping)

**Provider:**
- `/workspace/crates/maproom/src/embedding/ollama.rs` (configurable dimension, conditional sanitization)
- `/workspace/crates/maproom/src/embedding/config.rs` (validation update)

**Tests:**
- `/workspace/crates/maproom/tests/sqlite_integration.rs` (integration tests)

**Documentation:**
- `/workspace/docs/providers/ollama-setup.md` (user guide)
- `/workspace/crates/maproom/CLAUDE.md` (developer reference)

## Next Steps

1. **Review planning**: `/workstream:project-review DIM1024` (recommended)
2. **Generate tickets**: `/workstream:project-tickets DIM1024`
3. **Execute tickets**: `/workstream:project-work DIM1024` or `/workstream:ticket DIM1024-1001`

## Context

**Background**: The nomic-embed-text model has GGML tokenization bugs that cause crashes on markdown tables, checkboxes, Unicode arrows, and box-drawing characters. A workaround was implemented (lines 344-386 in ollama.rs) that replaces these characters before embedding, but this mangles the code content and reduces embedding quality.

**Solution**: Switch to mxbai-embed-large, which handles all content perfectly. This requires adding 1024-dimensional embedding support to the database and configuration system.

**Industry Context**: Multiple embedding dimensions are standard practice (OpenAI offers 256/512/1536/3072, Cohere offers 384/768/1024). This project follows the pattern established by Migration #7 (768-dim support).

**Related Projects:**
- EMBPERF (archived): Optimized Ollama parallel batch processing, confirmed mxbai-embed-large handles all content
- LOCAL (archived): Local deployment configuration, documented mxbai-embed-large as alternative model

## Resources

**Documentation:**
- `/workspace/docs/providers/ollama-setup.md` - Ollama configuration guide
- `/workspace/crates/maproom/CLAUDE.md` - Maproom developer guide

**Code References:**
- Migration #7 (add_vec_code_768) - Pattern to follow
- Migration #9 (add_context_cache) - Latest migration number
- OllamaProvider (ollama.rs) - Embedding provider implementation

**External:**
- [mxbai-embed-large on Ollama](https://ollama.com/library/mxbai-embed-large)
- [MTEB Leaderboard](https://huggingface.co/spaces/mteb/leaderboard) - Embedding model benchmarks
