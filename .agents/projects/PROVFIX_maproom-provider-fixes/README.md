# PROVFIX: Maproom Provider Configuration Fixes

## Problem

During implementation of provider selection for Maproom MCP, critical bugs were discovered in the Rust endpoint resolution logic. The current implementation requires brittle workarounds in the CLI that mask underlying issues.

### Symptoms

- ❌ OpenAI embeddings fail with "Connection refused" to localhost:11434
- ❌ Database updates fail with "column updated_at does not exist"
- ⚠️ CLI contains workaround code duplicated in 3 places
- ⚠️ Environment variable precedence is unclear and buggy

## Solution

Fix the root cause in Rust code, remove CLI workarounds, and address database schema issues:

1. **Rust Endpoint Resolution**: Fix `EmbeddingConfig` to validate endpoints match provider
2. **Database Schema**: Add missing `updated_at` column to chunks table
3. **CLI Cleanup**: Remove workaround code that explicitly sets endpoints
4. **Docker Cleanup**: Remove default endpoint that causes cross-provider pollution

## Impact

**Before Fixes**:
```
[ERROR] Failed to generate code embeddings: Connection refused (localhost:11434)
Provider: openai, Generated: 0, Failed: 854
```

**After Fixes**:
```
📊 Embedding Generation Summary:
   Provider: openai (1536 dimensions)
   Generated: 854, Cached: 0, Failed: 0
   API calls: 18, Tokens: 95000, Cost: $0.0019
```

## Project Structure

### Planning Documents

- **[analysis.md](planning/analysis.md)**: Deep dive into bugs discovered, root causes, and industry context
- **[architecture.md](planning/architecture.md)**: Proposed solutions without workarounds, implementation details
- **[quality-strategy.md](planning/quality-strategy.md)**: Pragmatic testing approach focused on preventing regression
- **[security-review.md](planning/security-review.md)**: Security implications analysis (low risk, improves posture)
- **[plan.md](planning/plan.md)**: Phased implementation plan with effort estimates and dependencies

### Tickets

Tickets will be created in the `tickets/` directory during implementation.

## Key Insights

### Root Cause

The bug exists because:
1. `EMBEDDING_API_ENDPOINT` environment variable loaded unconditionally
2. Docker Compose sets default `http://ollama:11434`
3. OpenAI provider inherits Ollama endpoint from environment
4. No validation that endpoint matches provider

### Workaround Applied (To Be Removed)

CLI explicitly sets `EMBEDDING_API_ENDPOINT=https://api.openai.com/v1/embeddings` for OpenAI:
- Duplicated in 3 functions
- Violates separation of concerns
- Must be maintained in parallel with Rust
- Masks underlying bug

### Proper Fix

Rust code should:
- Validate endpoint domain matches provider
- Ignore wrong-provider endpoints in environment
- Use provider-specific defaults
- Accept explicit overrides only when valid

## Relevant Agents

**Primary**: General-purpose agent
- Handles Rust, JavaScript, SQL
- Can complete all phases

**Alternative Specialists**:
- Rust specialist (if Phase 1 complex)
- Database specialist (if migration issues)
- Testing specialist (for integration tests)

## Phases

1. **Rust Core Fixes** (2-3 hours) - Critical path
2. **Database Schema** (1 hour) - Independent
3. **CLI Cleanup** (30 min) - Depends on Phase 1
4. **Docker Cleanup** (15 min) - After Phase 3
5. **Integration Testing** (1 hour) - Verify all fixes
6. **Documentation** (30 min) - Update docs

**Total Effort**: 5-6 hours

## Success Criteria

- ✅ OpenAI embeddings work without CLI workaround
- ✅ Database updates persist without errors
- ✅ CLI code is clean and simple
- ✅ All providers work with clear precedence rules
- ✅ Unit tests prevent regression

## Technical Details

### Files to Change

**Rust**:
- `/workspace/crates/maproom/src/embedding/config.rs` (main fix)
- `/workspace/crates/maproom/migrations/00XX_add_updated_at_to_chunks.sql` (new)

**JavaScript**:
- `/workspace/packages/maproom-mcp/bin/cli.cjs` (remove workarounds)

**Docker**:
- `/workspace/packages/maproom-mcp/config/docker-compose.yml` (clean defaults)

**Documentation**:
- `/workspace/packages/maproom-mcp/README.md` (update)

### Test Coverage

**Unit Tests** (Rust):
- Endpoint resolution for each provider
- Cross-provider endpoint rejection (the bug)
- Custom endpoint overrides
- Default endpoint fallbacks

**Integration Tests**:
- OpenAI: Setup → Scan → Embeddings
- Ollama: Verify still works
- Database: Verify `updated_at` column
- Environment precedence scenarios

## Risk Assessment

**Technical Risk**: Low
- Changes localized to config loading
- Workaround provides rollback path
- Existing tests catch regressions

**Security Risk**: Low → Very Low
- Fixes improve security posture
- Prevents unintended endpoint usage
- Reduces configuration complexity

## Related Issues

This project addresses issues discovered during:
- Provider selection implementation
- OpenAI integration testing
- Docker Compose configuration review

## Dependencies

**Phase 1** (Rust fixes) must complete before Phase 3 (CLI cleanup).

**Phase 2** (database) is independent and can proceed in parallel.

## Next Steps

1. Review planning documents
2. Create implementation tickets
3. Assign to general-purpose agent
4. Execute phases sequentially
5. Verify with integration tests

## Questions?

See planning documents for detailed analysis and implementation guidance.
