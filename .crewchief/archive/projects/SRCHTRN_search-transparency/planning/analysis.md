# Analysis: Search Transparency

## Problem Definition

Maproom's semantic search returns generic "RPC_ERROR: Search failed" messages when errors occur, providing no debugging information or guidance to users. During a recent user session, 2 search failures occurred with only these generic messages - no indication of what went wrong, why it failed, or how to fix it.

**Specific Pain Points:**
1. **Opaque Errors**: Generic RPC_ERROR provides no context about failure cause
2. **No Query Feedback**: Users don't know how their query was interpreted
3. **No Actionable Guidance**: No suggestions for query refinement
4. **Lost Context**: Error messages don't explain what stage failed (parsing, embedding, database, fusion)

## Context

### Why This Work Is Needed

Search is the primary interaction point for maproom users. When searches fail without explanation, users experience:
- **Frustration**: No way to debug or understand what went wrong
- **Lost Productivity**: Time wasted retrying queries blindly
- **Reduced Trust**: Opaque failures reduce confidence in the tool
- **Support Burden**: Generic errors lead to support requests

This is the highest-priority project from the maproom improvements initiative because it directly addresses user experience at the core interaction point.

### User Experience Impact

**Current Experience:**
```
Search failed: RPC_ERROR
```

**Desired Experience:**
```
Search failed: Embedding provider unavailable (OpenAI API timeout)

Suggestions:
- Check your OPENAI_API_KEY environment variable
- Verify network connectivity
- Try using FTS mode while waiting for provider: --mode fts
```

## Existing Solutions

### Industry Patterns

**Elasticsearch:**
- Returns detailed error objects with `type`, `reason`, and `caused_by` fields
- Provides query explanation via `_explain` API
- Shows how scores are calculated

**TypeSense:**
- Returns structured error responses with error codes
- Provides query understanding via `debug: true` parameter
- Shows typo corrections and suggestions

**Algolia:**
- Returns `query_suggestions` with alternatives
- Provides `explain` parameter for score breakdown
- Shows filters applied and why results were excluded

### Codebase Patterns

**Current Error Handling (TypeScript):**
```typescript
// packages/maproom-mcp/src/tools/search.ts:275-287
if (error instanceof RpcError) {
  // Check for repository not found error
  if (error.message.includes('query returned an unexpected number of rows')) {
    throw new ValidationError(
      `Repository '${repo}' not found or no data indexed.`,
      'REPO_NOT_FOUND'
    );
  }

  throw new ProcessError(
    `Daemon RPC error: ${error.message}`,
    'RPC_ERROR'
  );
}
```

**Problem**: Only one specific error pattern is detected (repo not found). All other failures become generic RPC_ERROR.

**Current Rust Error Types:**
```rust
// crates/maproom/src/search/pipeline.rs:476-492
pub enum PipelineError {
    QueryProcessing(QueryProcessorError),
    SearchExecution(ExecutorError),
    Database(String),
    Assembly(String),
}
```

**Problem**: Error context is lost during serialization to JSON-RPC. Only the display message survives, not the structured error data.

## Current State

### Error Flow

1. **Rust Pipeline Error** → Structured PipelineError with context
2. **Daemon RPC Handler** → Converts to JSON-RPC error with generic message
3. **TypeScript Client** → Receives RpcError with message string only
4. **MCP Tool** → Wraps in ProcessError("RPC_ERROR")
5. **User** → Sees "Search failed: RPC_ERROR"

**Context Lost at Step 2**: Structured error information (error type, stage, context) is flattened to a string message during JSON-RPC serialization.

### Query Processing Visibility

Currently, query processing details exist in Rust but are not exposed:
- `ProcessedQuery` contains tokens, mode detection, expanded terms
- `SearchMetadata` tracks timing and result counts
- Score fusion weights and strategies are internal

**Gap**: This information exists but is not returned to the client.

## Research Findings

### Performance Constraints

**Target**: <10ms overhead per search (current p95: ~100ms)

**Breakdown Analysis:**
- Query processing: ~5ms
- Search execution: ~30-40ms (parallel)
- Fusion: ~2-5ms
- Assembly: ~5-10ms

**Implication**: Adding metadata to existing structures is essentially free (no additional I/O). The data is already in memory.

### Error Scenarios Observed

From codebase analysis and user sessions:

1. **Embedding Provider Failures**
   - OpenAI API timeout
   - Google credentials missing
   - Ollama service not running

2. **Database Errors**
   - Repository not indexed
   - Worktree not found
   - Corrupted SQLite database

3. **Query Processing Errors**
   - Empty query
   - Query too long (>1000 chars)
   - No meaningful content

4. **Search Execution Errors**
   - FTS query syntax errors
   - Vector search with missing embeddings
   - Database connection timeout

### Backward Compatibility Requirements

**TypeScript Types** (daemon-client):
- Must remain in sync with Rust
- Cannot remove existing fields
- Can add optional fields

**JSON-RPC Protocol**:
- Cannot change error code structure
- Can add data field with structured information
- Existing clients must continue to work

## Constraints

### Technical Constraints

1. **Performance Budget**: <10ms overhead per search
2. **No Schema Changes**: Cannot modify database tables
3. **Backward Compatibility**: Additive API changes only
4. **Type Sync**: TypeScript ↔ Rust type alignment required
5. **Client-Side Validation**: Zod schemas must catch errors before RPC

### Design Constraints

1. **MVP Focus**: Ship value, not ceremonies
2. **Pragmatic Error Messages**: Actionable over comprehensive
3. **No Over-Engineering**: Don't build generic error framework for 5 error types
4. **Progressive Enhancement**: Basic errors first, advanced feedback later

### Resource Constraints

1. **Phase 1 Timeline**: Foundation work runs parallel with SRCHFLTR
2. **No Breaking Changes**: Cannot disrupt existing users
3. **Single Source of Truth**: Rust defines error structure

## Success Criteria

### Quantitative Metrics

1. **90% reduction in generic RPC_ERROR messages**
   - Current: ~95% of errors are generic RPC_ERROR
   - Target: <10% generic errors (only truly unknown failures)

2. **Query understanding visible on every search**
   - 100% of successful searches return metadata
   - Include: tokens, mode, expanded terms, filters applied

3. **At least 1-2 actionable suggestions per failed query**
   - Specific suggestions based on error type when context available
   - Generic but actionable suggestions acceptable for limited-context errors
   - Example: FTS mode suggestion for embedding failures
   - Note: Suggestion quality varies by error context availability - MVP accepts pragmatic suggestions

4. **Performance maintained: p95 <100ms**
   - No regression from adding metadata
   - Measured via existing metrics system

### Qualitative Criteria

1. **Error messages are actionable**
   - User knows what went wrong
   - User knows how to fix it
   - Example: "Check OPENAI_API_KEY" vs "embedding failed"

2. **Query understanding is clear**
   - User sees how query was interpreted
   - User understands why results match
   - User knows what filters were applied

3. **No debugging friction**
   - Developers can diagnose issues from error message alone
   - No need to check logs or run debug mode
   - Support burden reduced

### Acceptance Tests

1. **Embedding provider offline**
   - Error message identifies provider (OpenAI/Google/Ollama)
   - Suggests checking credentials and network
   - Recommends FTS mode fallback

2. **Repository not found**
   - Error message names the repository
   - Suggests running `status` to see available repos
   - Suggests running `scan` to index new repo

3. **Empty query**
   - Error caught at client validation (Zod)
   - Clear message before RPC call
   - No network round-trip

4. **Successful search**
   - Metadata shows query understanding
   - Tokens, mode, expanded terms visible
   - Timing breakdown available

## Risks and Mitigations

### Risk 1: Performance Regression

**Mitigation**:
- Metadata uses existing in-memory data
- No additional database queries
- Measure before/after with existing metrics

### Risk 2: Type Sync Drift

**Mitigation**:
- Comprehensive type sync documentation
- Comments linking Rust ↔ TypeScript types
- Integration tests validating serialization

### Risk 3: Over-Engineering

**Mitigation**:
- MVP scope: 5-6 error types only
- No generic error framework
- Pragmatic string-based suggestions

### Risk 4: Breaking Existing Clients

**Mitigation**:
- Additive changes only (optional fields)
- Existing clients ignore new fields
- Version field in responses for future compatibility
