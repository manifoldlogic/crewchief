# Ticket: HYBRID_SEARCH-1003: Embedding Generation Pipeline

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- embeddings-engineer
- database-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement a batch embedding generation pipeline that generates embeddings for all existing code chunks in the database, with support for incremental updates, cost tracking, and monitoring. This pipeline will populate both `code_embedding` and `text_embedding` columns for semantic search capabilities.

## Background
As part of Phase 1 (Embedding Infrastructure) of the Hybrid Search project, we need to generate vector embeddings for all existing code chunks in the database. These embeddings will enable semantic similarity search to complement the existing full-text search. The pipeline must be efficient, cost-aware, and support incremental updates so that only new or changed chunks require re-embedding.

This is Task 3 of Phase 1, Week 1, building upon the Embedding Service (HYBRID_SEARCH-1001) and Database Schema updates (HYBRID_SEARCH-1002).

## Acceptance Criteria
- [ ] Successfully generate embeddings for all existing chunks in the database
- [ ] Embedding cache achieves >80% hit rate during batch processing
- [ ] Cost tracking implemented and reports total API costs for embedding generation
- [ ] Incremental updates working - only new/changed chunks generate new embeddings
- [ ] Pipeline tested with 1000 sample chunks before full rollout
- [ ] Batch processing reduces API calls through efficient grouping
- [ ] CLI command for generating embeddings is functional and well-documented
- [ ] Error handling and retry logic for API failures is implemented
- [ ] Progress reporting shows chunks processed, embeddings generated, and estimated completion time

## Technical Requirements
- Generate both `code_embedding` (1536-dim) and `text_embedding` (1536-dim) for each chunk
- Use OpenAI `text-embedding-3-small` model as configured in EmbeddingService
- Implement batch processing to reduce API calls (batch size configurable, default 100 chunks)
- Track API costs by counting tokens sent and calculating costs at current rates
- Support incremental mode: only generate embeddings for chunks where `code_embedding IS NULL` or `text_embedding IS NULL`
- Leverage LRU cache from EmbeddingService (10,000 entry cache with 1-hour TTL)
- Update database atomically using transactions for batch updates
- Provide CLI command: `crewchief-maproom generate-embeddings [--incremental] [--batch-size N] [--dry-run] [--sample N]`
- Log embedding generation statistics: chunks processed, API calls made, cache hit rate, total cost
- Handle rate limiting and implement exponential backoff retry logic

## Implementation Notes

### Architecture Reference
See `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`:
- Lines 83-117: Embedding Service implementation with caching
- Lines 268-272: Configuration for cache settings (cache_size: 10000, cache_ttl: 3600)

### Pipeline Flow
1. **Query Phase**: Fetch chunks that need embeddings (based on `--incremental` flag)
2. **Batch Phase**: Group chunks into batches (default 100) to minimize API overhead
3. **Generation Phase**: For each batch:
   - Generate `code_embedding` using chunk's code content
   - Generate `text_embedding` using chunk's documentation/comment content
   - Track cache hits/misses and API costs
   - Handle rate limits and retries
4. **Update Phase**: Write generated embeddings back to database in transaction
5. **Report Phase**: Display statistics on completion

### Cost Tracking
- Track total tokens sent to OpenAI API
- Calculate costs based on text-embedding-3-small pricing ($0.00002/1K tokens)
- Report running totals during execution
- Log to database for historical cost analysis

### Incremental Updates
- Default mode: only process chunks where embeddings are NULL
- Full mode (`--force`): regenerate all embeddings
- Change detection: track chunk content hashes to detect modifications

### Error Handling
- Retry failed API calls with exponential backoff (max 5 retries)
- Continue processing on individual chunk failures, log errors
- Checkpoint progress periodically to resume interrupted runs
- Validate embedding dimensions before database write (must be 1536)

## Dependencies
- **HYBRID_SEARCH-1001**: Embedding Service Setup
  - Required for EmbeddingService implementation with OpenAI client
  - Required for LRU cache functionality
  - Required for retry logic and error handling

- **HYBRID_SEARCH-1002**: Database Schema Migration
  - Required for `code_embedding` and `text_embedding` vector columns
  - Required for vector column indices if performance is critical
  - Required for schema to support 1536-dimensional vectors

## Risk Assessment
- **Risk**: OpenAI API rate limiting during large batch processing
  - **Mitigation**: Implement exponential backoff, respect rate limit headers, add `--batch-delay` parameter for controlled throttling

- **Risk**: High API costs if cache is ineffective or full regeneration is needed
  - **Mitigation**: Start with 1000 sample chunks test, monitor cost per chunk, implement cost ceiling parameter (`--max-cost`)

- **Risk**: Database transaction timeouts on large batch updates
  - **Mitigation**: Keep batch sizes reasonable (100 chunks), use smaller transactions, implement checkpoint/resume functionality

- **Risk**: Memory issues with large result sets
  - **Mitigation**: Stream chunks from database using cursor, process in batches, avoid loading entire dataset into memory

- **Risk**: Inconsistent embeddings if process is interrupted mid-batch
  - **Mitigation**: Use database transactions for atomic batch updates, implement resume capability based on chunk status tracking

## Files/Packages Affected
- `crates/maproom/src/embedding/pipeline.rs` - NEW: Core batch embedding generation pipeline
- `crates/maproom/src/embedding/cost_tracker.rs` - NEW: Cost monitoring and reporting
- `crates/maproom/src/cli/commands/generate_embeddings.rs` - NEW: CLI command implementation
- `crates/maproom/src/db/queries.rs` - MODIFY: Add queries for fetching chunks needing embeddings
- `crates/maproom/src/db/models.rs` - MODIFY: Add methods for batch embedding updates
- `crates/maproom/tests/integration/embedding_pipeline.rs` - NEW: Integration tests for pipeline
- `crates/maproom/README.md` - MODIFY: Document new CLI command
