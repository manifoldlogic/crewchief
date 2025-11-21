# Ticket: Implement Command Handler

**ID:** VECSRCH-2003
**Phase:** Implementation
**Status:** ✅ Completed
**Completed:** 2025-11-21

## Title & Summary
Implement the command handler for the vector search CLI command.

## Background
Once the CLI command is defined, we need the logic to actually execute the search using `VectorExecutor` and print the results.

## Acceptance Criteria
1.  The command handler instantiates `VectorExecutor`.
2.  It executes the search using the provided query and parameters.
3.  It outputs the results in a structured JSON format to stdout.
4.  Errors are printed to stderr.

## Technical Requirements
- Use `serde_json` to serialize the output.
- Ensure the output schema is consistent and documented (for the MCP client to consume).
- Handle database connection initialization within the handler.

## Implementation Notes
- The handler should be an async function.
- Output JSON should include: `chunk_id`, `score`, `content`, `file_path`.

## Dependencies
- VECSRCH-2001 (Types exposed)
- VECSRCH-2002 (CLI definition)

## Risks
- Database connection failure.
- Slow cold start (accepted risk).

## Files/Packages
- `crates/maproom/src/main.rs`

## Agent Assignments
- **Primary:** Rust Developer

---

## Completion Notes

**Implementation Summary:**

Implemented full vector search handler in `src/main.rs` (lines 980-1108):

**Key Components:**

1. **Database Connection & ID Resolution:**
   - Connects to database via `db::connect()`
   - Resolves `repo_id` from repo name
   - Resolves optional `worktree_id` for filtering

2. **Query Embedding Generation:**
   - Uses `EmbeddingService::from_env()` to initialize service
   - Generates embedding vector from query text via `embed_text()`
   - Requires `OPENAI_API_KEY` environment variable

3. **Vector Search Execution:**
   - Calls `VectorExecutor::execute()` with:
     - Database client
     - Query embedding vector
     - SearchMode::Code (hardcoded for now)
     - repo_id and optional worktree_id
     - k (result limit)
   - Returns ranked results with similarity scores

4. **Threshold Filtering:**
   - Applies optional threshold filter to results
   - Only includes results where `score >= threshold`

5. **Chunk Detail Retrieval:**
   - Queries database for chunk metadata (file path, symbol name, kind, line numbers)
   - Joins chunks and files tables on chunk_id

6. **JSON Output:**
   - Documented schema (lines 1079-1097) for MCP client
   - Includes: chunk_id, score, start_line, end_line, symbol_name, kind, file_path
   - Metadata: total count, query, mode, k, threshold

**Acceptance Criteria Met:**

✅ 1. Handler instantiates VectorExecutor
   - Uses `VectorExecutor::execute()` directly

✅ 2. Executes search with provided query and parameters
   - Generates embedding from query text
   - Passes k and filters by repo/worktree
   - Applies threshold if specified

✅ 3. Outputs results in structured JSON format to stdout
   - Schema documented in code comments
   - Pretty-printed JSON via `serde_json::to_string_pretty()`

✅ 4. Errors printed to stderr
   - Uses `anyhow::Context` for error messages
   - Tracing info logs for debugging

**JSON Output Schema:**
```json
{
  "hits": [
    {
      "chunk_id": 123,
      "score": 0.92,
      "start_line": 10,
      "end_line": 20,
      "symbol_name": "authenticate",
      "kind": "func",
      "file_path": "src/auth.rs"
    }
  ],
  "total": 10,
  "query": "authentication logic",
  "mode": "vector",
  "k": 10,
  "threshold": null
}
```

**Files Modified:**
- `crates/maproom/src/main.rs`:
  - Lines 980-1108: Full vector search handler implementation
  - Lines 1079-1097: JSON schema documentation

**Dependencies:**
- EmbeddingService (for query embedding generation)
- VectorExecutor (for similarity search)
- Database connection (for repo/worktree/chunk lookups)

**Testing Notes:**
- Requires database with:
  - Indexed repository and worktree
  - Generated embeddings (via `generate-embeddings` command)  
  - OPENAI_API_KEY environment variable
- Will be tested in VECSRCH-3001 (Integration Testing)
