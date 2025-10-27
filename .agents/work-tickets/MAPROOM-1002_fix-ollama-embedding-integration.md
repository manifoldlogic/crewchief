# Ticket: MAPROOM-1002: Fix Ollama Embedding Integration to Restore Vector/Semantic Search

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- embeddings-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Fix the Ollama embedding integration to restore vector and semantic search functionality. The current implementation uses OpenAI's API format, which is incompatible with Ollama's embedding endpoint, causing all embedding generation to fail in the containerized environment.

## Background
Vector and semantic search were working before containerization, but the `generate-embeddings` command now fails when using Ollama. The embedding generation pipeline connects to Ollama successfully but fails to parse responses because:

1. **Wrong API endpoint**: Code uses `/api/embeddings` but Ollama uses `/api/embed` (singular)
2. **Wrong request format**: Code sends `{"model": "...", "prompt": [...]}` but Ollama expects `{"model": "...", "input": "..."}`
3. **Wrong response parsing**: Code expects OpenAI format with `data[].embedding` and `usage.total_tokens`, but Ollama returns `{"model": "...", "embeddings": [[...]]}`

**Current Behavior:**
```bash
$ crewchief-maproom generate-embeddings --sample 1
# Connects to Ollama successfully
# Fails with: "Batch 1 failed: Failed to generate code embeddings"
# Result: Generated: 0, Cached: 0, Failed: 1
```

**Environment Variables (Verified Correct):**
- `EMBEDDING_PROVIDER=ollama`
- `EMBEDDING_MODEL=nomic-embed-text`
- `EMBEDDING_DIMENSION=768`
- `EMBEDDING_API_ENDPOINT=http://ollama:11434`

**Ollama Service Status:** ✅ Running and healthy with nomic-embed-text model loaded

**Impact:** HIGH severity - Vector/semantic search completely broken in containerized environment. Only FTS (full-text search) works; semantic/hybrid search unavailable.

## Acceptance Criteria
- [ ] `generate-embeddings --sample 5` successfully generates 5 embeddings via Ollama
- [ ] Database has chunks with non-NULL `code_embedding` and `text_embedding` columns
- [ ] Vector search works: `crewchief-maproom search --repo crewchief --query "authentication" --mode vector`
- [ ] Hybrid search works: `crewchief-maproom search --repo crewchief --query "authentication" --mode hybrid`
- [ ] All embedding-related tests pass
- [ ] Cost tracking works (tokens estimated from input text length for Ollama)
- [ ] Full embedding generation completes: `generate-embeddings --incremental`

## Technical Requirements
- Preserve OpenAI compatibility (don't break existing OpenAI integration)
- Add proper Ollama response struct that matches actual API format
- Handle batch requests correctly (Ollama can handle arrays of inputs)
- Implement token estimation for Ollama (since it doesn't return usage stats)
- Use correct Ollama endpoint: `POST /api/embed`
- Use correct request format: `{"model": "nomic-embed-text", "input": "text to embed"}`
- Parse correct response format: `{"model": "...", "embeddings": [[...]]}`

## Implementation Notes

### Root Cause Analysis
The code was written for OpenAI's embedding API and needs to be adapted for Ollama's different API contract while maintaining backward compatibility.

### Files to Modify

1. **`/workspace/crates/maproom/src/embedding/client.rs`** (Primary changes)
   - **Line 194-232**: Fix Ollama request format to use `input` field instead of `prompt`
   - **Line 221**: Change endpoint from `/api/embeddings` to `/api/embed`
   - **Line 244**: Add separate response parsing for Ollama format vs OpenAI format
   - Consider adding provider-specific request/response structs

2. **`/workspace/crates/maproom/src/embedding/config.rs`**
   - **Line 221**: Fix default endpoint from `/api/embeddings` to `/api/embed`
   - May need provider-aware endpoint configuration

### Implementation Approach
1. Create separate request/response structs for OpenAI vs Ollama
2. Add conditional logic based on `EMBEDDING_PROVIDER` environment variable
3. Implement token estimation for Ollama (approximate from input text length)
4. Ensure batch processing works for both providers
5. Add clear error messages distinguishing provider-specific failures

### Testing Steps
```bash
# 1. Generate embeddings for sample
docker-compose exec -T maproom-mcp /usr/local/bin/crewchief-maproom generate-embeddings --sample 5

# 2. Verify embeddings in database
psql -h localhost -p 5433 -U maproom -d maproom -c "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL;"

# 3. Test vector search
docker-compose exec -T maproom-mcp /usr/local/bin/crewchief-maproom search --repo crewchief --query "agent spawn" --mode vector --k 5

# 4. Test hybrid search
docker-compose exec -T maproom-mcp /usr/local/bin/crewchief-maproom search --repo crewchief --query "agent spawn" --mode hybrid --k 5
```

### Expected Results After Fix
- Sample embedding run should show: "Generated: 5, Cached: 0, Failed: 0"
- Database query should return count > 0 for embeddings
- Vector search should return relevant results with similarity scores
- Hybrid search should combine FTS + vector results effectively

## Dependencies
- Ollama service must be running and healthy (prerequisite)
- PostgreSQL database must be accessible (prerequisite)
- Docker Compose environment must be configured (prerequisite)

## Risk Assessment
- **Risk**: Breaking OpenAI integration while fixing Ollama
  - **Mitigation**: Implement provider-specific code paths; test both providers if possible; use feature flags or environment variables to maintain compatibility

- **Risk**: Incorrect token estimation for Ollama causing cost tracking issues
  - **Mitigation**: Document that Ollama token counts are estimates; use conservative estimation (e.g., 1 token per 4 characters); consider adding a flag to disable cost tracking for Ollama

- **Risk**: Batch size mismatch between providers
  - **Mitigation**: Research Ollama's batch limits; implement provider-specific batch size configuration; add error handling for batch size limits

## Files/Packages Affected
- `/workspace/crates/maproom/src/embedding/client.rs` - Primary changes to request/response handling
- `/workspace/crates/maproom/src/embedding/config.rs` - Endpoint configuration changes
- `/workspace/crates/maproom/src/embedding/mod.rs` - May need updates to error types or exports
- Test files in `/workspace/crates/maproom/tests/` - Verify embedding-related tests

## References
- Ollama API docs: https://github.com/ollama/ollama/blob/main/docs/api.md#generate-embeddings
- Ollama embeddings endpoint: `POST /api/embed`
- Request format: `{"model": "nomic-embed-text", "input": "text to embed"}`
- Response format: `{"model": "...", "embeddings": [[...]]}`
