# DIM1024 Ticket Index

Project: **embedding dimension 1024**
Slug: **DIM1024**
Total Tickets: **4**

## Overview

This project adds 1024-dimensional embedding support to enable the mxbai-embed-large model, which handles all content types without tokenization crashes. Implementation follows the established pattern from 768-dim support (Migration #7) and maintains backward compatibility with existing embeddings.

## Ticket Summary

### Phase 1: Database Foundation (1 ticket)
- **DIM1024-1001**: Database Foundation for 1024-Dimensional Embeddings
  - Status: Not started
  - Agent: rust-developer
  - Scope: 2-3 hours
  - Description: Add Migration #10 for vec_code_1024 virtual table and update dimension constants

### Phase 2: Provider Configuration (2 tickets)
- **DIM1024-2001**: Make Ollama Provider Dimension Configurable
  - Status: Not started
  - Agent: rust-developer
  - Scope: 3-4 hours
  - Dependencies: DIM1024-1001
  - Description: Remove hardcoded dimension=768, make dimension configurable via constructor

- **DIM1024-2002**: Conditional Sanitization for Model Compatibility
  - Status: Not started
  - Agent: rust-developer
  - Scope: 1-2 hours
  - Dependencies: DIM1024-2001
  - Description: Apply character sanitization only for nomic-embed-text, preserve raw text for mxbai-embed-large

### Phase 3: Testing and Documentation (1 ticket)
- **DIM1024-3001**: Comprehensive Testing and Documentation
  - Status: Not started
  - Agents: rust-developer, documentation-writer
  - Scope: 3-4 hours
  - Dependencies: DIM1024-1001, DIM1024-2001, DIM1024-2002
  - Description: E2E tests, migration idempotency tests, user documentation updates

## Execution Order

The tickets must be completed in this order due to dependencies:

1. **DIM1024-1001** (Database Foundation)
   - No dependencies, foundation for all other work

2. **DIM1024-2001** (Provider Configuration)
   - Depends on: DIM1024-1001
   - Reason: Cannot configure 1024-dim until database supports it

3. **DIM1024-2002** (Sanitization Cleanup)
   - Depends on: DIM1024-2001
   - Reason: Requires dimension-aware provider infrastructure

4. **DIM1024-3001** (Testing and Documentation)
   - Depends on: DIM1024-1001, DIM1024-2001, DIM1024-2002
   - Reason: Tests the integrated system

## Project Goals

- Enable mxbai-embed-large model support (1024 dimensions)
- Maintain backward compatibility with existing 768-dim and 1536-dim embeddings
- Remove character sanitization workaround for mxbai-embed-large
- Provide clear documentation for users switching models

## Success Criteria

- [ ] Migration #10 creates vec_code_1024 table
- [ ] OllamaProvider accepts configurable dimension
- [ ] mxbai-embed-large generates 1024-dim embeddings
- [ ] Vector search works with 1024-dim queries
- [ ] Mixed dimensions (768, 1024, 1536) coexist
- [ ] Documentation updated with configuration examples
- [ ] All tests pass (unit + integration + E2E)

## Key Files Modified

- `/workspace/crates/maproom/src/db/sqlite/migrations.rs`
- `/workspace/crates/maproom/src/db/sqlite/embeddings.rs`
- `/workspace/crates/maproom/src/db/sqlite/vector.rs`
- `/workspace/crates/maproom/src/db/columns.rs`
- `/workspace/crates/maproom/src/embedding/ollama.rs`
- `/workspace/crates/maproom/src/embedding/config.rs`
- `/workspace/crates/maproom/tests/sqlite_integration.rs`
- `/workspace/docs/providers/ollama-setup.md`
- `/workspace/crates/maproom/CLAUDE.md`

## Timeline Estimate

- **Phase 1**: 2-3 hours (Database Foundation)
- **Phase 2**: 4-6 hours (Provider Configuration + Sanitization)
- **Phase 3**: 3-4 hours (Testing and Documentation)
- **Total**: 9-13 hours

## References

- **Plan**: `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/plan.md`
- **Architecture**: `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/architecture.md`
- **Quality Strategy**: `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/quality-strategy.md`
