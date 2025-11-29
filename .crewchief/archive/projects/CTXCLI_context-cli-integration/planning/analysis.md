# Analysis: Context CLI Integration

## Problem Statement

The Rust context assembler (implemented in SQLIMPL-4001 through SQLIMPL-4004) is complete but **not exposed** to the MCP server. The TypeScript MCP context tool (`packages/maproom-mcp/src/tools/context.ts`) currently implements its own PostgreSQL-based context assembly, bypassing the Rust implementation entirely.

### Current State

1. **Rust Context Module** (Complete)
   - Location: `crates/maproom/src/context/`
   - Components: assembler, cache, graph, file_loader, token_counter, strategies, detectors
   - Uses SQLite via `SqliteStore`
   - Supports `ExpandOptions` (callers, callees, tests, docs, hooks, jsx_parents, etc.)
   - Has language-specific strategies (React, Python, Rust)
   - Includes LRU caching and parallel graph loading

2. **MCP Context Tool** (Needs Update)
   - Location: `packages/maproom-mcp/src/tools/context.ts`
   - Uses PostgreSQL (`pg` client) directly
   - Queries `maproom.chunks`, `maproom.files`, `maproom.worktrees`, `maproom.relationships`
   - Duplicates context assembly logic in TypeScript
   - Missing language-specific strategies

3. **JSON-RPC Daemon** (Needs Extension)
   - Location: `crates/maproom/src/daemon/mod.rs`
   - Currently supports: `ping`, `search`
   - Does NOT support: `context`
   - Used by MCP server for search operations

### Integration Gap

```
┌─────────────────────────────────────────────────────────────────┐
│                          MCP Server                              │
│  (packages/maproom-mcp)                                          │
├─────────────────────────────────────────────────────────────────┤
│  search tool ─────► daemon (JSON-RPC) ─────► Rust search        │
│  context tool ────► PostgreSQL (pg) ────► TypeScript assembly   │ ← Gap!
└─────────────────────────────────────────────────────────────────┘
```

The search tool already uses the Rust daemon. The context tool should follow the same pattern.

## Technical Analysis

### CLI Command Structure

Existing CLI commands in `main.rs`:
- `Db` - Database migrations and cleanup
- `Cache` - Cache management
- `Scan` - Repository indexing
- `Upsert` - File updates
- `Watch` - File watching
- `Search` - FTS search
- `VectorSearch` - Vector search
- `Status` - Index status
- `GenerateEmbeddings` - Embedding generation
- `Migrate` - Markdown migration
- `Serve` - JSON-RPC daemon

The `context` command should match this pattern.

### Daemon Integration

The daemon (`src/daemon/mod.rs`) handles JSON-RPC requests:
```rust
match request.method.as_str() {
    "ping" => ...,
    "search" => ...,
    // "context" => ... // Add this
}
```

Adding a `context` method requires:
1. Define `ContextParams` in `daemon/types.rs`
2. Add `context` case in `handle_request()`
3. Call `BasicContextAssembler::assemble()`
4. Serialize result to JSON

### ExpandOptions Mapping

MCP `expand` schema vs Rust `ExpandOptions`:
```
MCP                    Rust
---                    ----
callers: boolean       callers: bool
callees: boolean       callees: bool
tests: boolean         tests: bool
docs: boolean          docs: bool
config: boolean        config: bool
max_depth: number      max_depth: i32
                       routes: bool        (React-specific)
                       hooks: bool         (React-specific)
                       jsx_parents: bool   (React-specific)
                       jsx_children: bool  (React-specific)
```

The MCP schema should be extended to include React-specific options.

## Existing Code to Reuse

1. **`BasicContextAssembler`** - Main context assembly logic
2. **`ContextCache`** - LRU caching for assembled bundles
3. **`load_relationships_parallel()`** - Parallel graph loading
4. **`ContextBundle`, `ContextItem`, `ExpandOptions`** - Serializable types
5. **Language strategies** - React, Python, Rust specific logic

## Dependencies

### Rust Dependencies (Already Available)
- `tokio` - Async runtime
- `serde`, `serde_json` - Serialization
- `anyhow` - Error handling
- `tracing` - Logging
- `async-trait` - Async traits

### TypeScript Dependencies (Already Available)
- `@crewchief/daemon-client` - JSON-RPC client
- `zod` - Schema validation

## Constraints

1. **Backward Compatibility** - MCP tool interface must remain unchanged
2. **Performance** - Context assembly must be fast (target < 100ms)
3. **Error Handling** - Graceful degradation if chunk not found
4. **Token Budget** - Must respect `budget_tokens` parameter

## Risks

1. **Schema Mismatch** - Rust and TypeScript context schemas may diverge
2. **Daemon Startup** - Cold start latency for first request
3. **File Access** - Rust assembler reads files from worktree (may not exist)

## Recommendation

**Option 1: Add CLI Context Command + Daemon Method** (Recommended)

1. Add `context` CLI command for standalone use
2. Add `context` method to JSON-RPC daemon
3. Update MCP context tool to use daemon client

Benefits:
- Consistent architecture (all tools use daemon)
- Single source of truth for context assembly
- Leverages existing caching and strategies
- Testable via CLI

This approach aligns with the existing `search` integration pattern.
