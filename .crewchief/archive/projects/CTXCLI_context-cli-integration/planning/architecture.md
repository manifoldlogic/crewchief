# Architecture: Context CLI Integration

## Overview

This document describes the technical architecture for exposing the Rust context assembler via CLI and JSON-RPC daemon, enabling the MCP server to use the unified SQLite-based context assembly.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          MCP Server                                      │
│  (packages/maproom-mcp)                                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐     │
│  │  search tool    │    │  context tool   │    │  other tools    │     │
│  └────────┬────────┘    └────────┬────────┘    └─────────────────┘     │
│           │                      │                                       │
│           └──────────┬───────────┘                                       │
│                      │                                                   │
│                      ▼                                                   │
│            ┌─────────────────────┐                                       │
│            │   daemon-client     │ (JSON-RPC over stdio)                 │
│            │   @crewchief/       │                                       │
│            │   daemon-client     │                                       │
│            └─────────┬───────────┘                                       │
│                      │                                                   │
└──────────────────────┼───────────────────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                    Rust Daemon (crewchief-maproom serve)                  │
│  (crates/maproom/src/daemon/)                                            │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐     │
│  │   "ping"        │    │   "search"      │    │   "context" NEW │     │
│  └─────────────────┘    └────────┬────────┘    └────────┬────────┘     │
│                                  │                      │               │
│                                  ▼                      ▼               │
│                        ┌─────────────────┐    ┌─────────────────┐      │
│                        │  search module  │    │ context module  │      │
│                        │  (fts/vector/   │    │ (assembler/     │      │
│                        │   hybrid)       │    │  strategies)    │      │
│                        └────────┬────────┘    └────────┬────────┘      │
│                                 │                      │                │
│                                 └──────────┬───────────┘                │
│                                            │                            │
│                                            ▼                            │
│                                  ┌─────────────────┐                    │
│                                  │   SqliteStore   │                    │
│                                  └────────┬────────┘                    │
│                                           │                             │
└───────────────────────────────────────────┼─────────────────────────────┘
                                            │
                                            ▼
                                  ┌─────────────────┐
                                  │  SQLite + FTS5  │
                                  │  + sqlite-vec   │
                                  └─────────────────┘
```

## Component Details

### 1. CLI Context Command

**Location:** `crates/maproom/src/main.rs`

```rust
/// Retrieve context bundle for a chunk
Context {
    /// Chunk ID to retrieve context for
    #[arg(long)]
    chunk_id: i64,

    /// Maximum tokens for the bundle (default: 6000)
    #[arg(long, default_value_t = 6000)]
    budget: usize,

    /// Include caller functions
    #[arg(long)]
    callers: bool,

    /// Include callee functions
    #[arg(long)]
    callees: bool,

    /// Include test files
    #[arg(long)]
    tests: bool,

    /// Include documentation
    #[arg(long)]
    docs: bool,

    /// Include configuration files
    #[arg(long)]
    config: bool,

    /// Maximum traversal depth (default: 2)
    #[arg(long, default_value_t = 2)]
    max_depth: i32,

    /// Output as JSON instead of human-readable
    #[arg(long)]
    json: bool,
}
```

### 2. Daemon Context Method

**Location:** `crates/maproom/src/daemon/types.rs`

```rust
#[derive(Debug, Deserialize)]
pub struct ContextParams {
    pub chunk_id: String,  // String for JSON compatibility
    #[serde(default = "default_budget")]
    pub budget_tokens: usize,
    #[serde(default)]
    pub expand: ExpandConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct ExpandConfig {
    #[serde(default)]
    pub callers: bool,
    #[serde(default)]
    pub callees: bool,
    #[serde(default)]
    pub tests: bool,
    #[serde(default)]
    pub docs: bool,
    #[serde(default)]
    pub config: bool,
    #[serde(default = "default_max_depth")]
    pub max_depth: i32,
    // React-specific
    #[serde(default)]
    pub routes: bool,
    #[serde(default)]
    pub hooks: bool,
    #[serde(default)]
    pub jsx_parents: bool,
    #[serde(default)]
    pub jsx_children: bool,
}

fn default_budget() -> usize { 6000 }
fn default_max_depth() -> i32 { 2 }
```

**Location:** `crates/maproom/src/daemon/mod.rs`

#### DaemonState with Context Support

The `BasicContextAssembler` must be part of `DaemonState` to enable caching across requests:

```rust
// Updated DaemonState with context assembler
pub struct DaemonState {
    pub store: Arc<SqliteStore>,
    pub embedding_service: EmbeddingService,
    pub context_assembler: BasicContextAssembler,  // NEW: enables caching
}

impl DaemonState {
    pub fn new(store: Arc<SqliteStore>, embedding_service: EmbeddingService) -> Self {
        Self {
            store: store.clone(),
            embedding_service,
            context_assembler: BasicContextAssembler::new(
                store,
                CacheConfig::default(),  // LRU cache with 100 bundles
            ),
        }
    }
}
```

#### Request Handler

```rust
async fn handle_request(request: JsonRpcRequest, state: Arc<DaemonState>) -> JsonRpcResponse {
    match request.method.as_str() {
        "ping" => ...,
        "search" => ...,
        "context" => {
            let params: ContextParams = match serde_json::from_value(...) { ... };
            match execute_context(state, params).await {
                Ok(bundle) => JsonRpcResponse::success(id, bundle),
                Err(e) => JsonRpcResponse::error(id, -32000, e.to_string(), None),
            }
        }
        _ => ...
    }
}

async fn execute_context(
    state: Arc<DaemonState>,
    params: ContextParams,
) -> Result<serde_json::Value> {
    let chunk_id = params.chunk_id.parse::<i64>()?;
    let options = ExpandOptions {
        callers: params.expand.callers,
        callees: params.expand.callees,
        tests: params.expand.tests,
        docs: params.expand.docs,
        config: params.expand.config,
        max_depth: params.expand.max_depth,
        routes: params.expand.routes,
        hooks: params.expand.hooks,
        jsx_parents: params.expand.jsx_parents,
        jsx_children: params.expand.jsx_children,
        ..Default::default()
    };

    // Use state's assembler (not new instance) to benefit from caching
    let bundle = state.context_assembler
        .assemble(chunk_id, params.budget_tokens, options)
        .await?;

    Ok(serde_json::to_value(bundle)?)
}
```

### 3. MCP Context Tool Update

**Location:** `packages/maproom-mcp/src/tools/context.ts`

Replace PostgreSQL-based implementation with daemon client call:

```typescript
import { DaemonClient } from '@crewchief/daemon-client'

export async function handleContextTool(
  params: unknown,
  daemonClient: DaemonClient
): Promise<ContextBundle> {
  const validatedParams = validateContextParams(params)

  // Call Rust daemon instead of PostgreSQL
  const result = await daemonClient.call('context', {
    chunk_id: validatedParams.chunk_id,
    budget_tokens: validatedParams.budget_tokens,
    expand: validatedParams.expand,
  })

  return result as ContextBundle
}
```

### 4. Response Schema and Mapping

#### Schema Synchronization

The Rust and TypeScript schemas must stay in sync:

| Field | Rust Location | TypeScript Location |
|-------|---------------|---------------------|
| `ExpandOptions` | `crates/maproom/src/context/types.rs` | `packages/maproom-mcp/src/tools/context_schema.ts` |
| `ContextBundle` | `crates/maproom/src/context/types.rs` | `packages/maproom-mcp/src/tools/context.ts` |

#### Rust ContextBundle (Daemon Response)

```rust
// crates/maproom/src/context/types.rs
pub struct ContextBundle {
    pub items: Vec<ContextItem>,
    pub total_tokens: usize,
    pub truncated: bool,
}
```

#### MCP ContextBundle (Enhanced for Clients)

```typescript
// packages/maproom-mcp/src/tools/context.ts
interface ContextBundle {
  items: ContextItem[]
  total_tokens: number
  budget_tokens: number      // Computed: from request params
  budget_remaining: number   // Computed: budget_tokens - total_tokens
  truncated: boolean
  metadata: {                // Computed: from first item
    worktree: string
    repo: string
  }
  warnings?: string[]        // Optional: any assembly warnings
}
```

#### Mapping Layer (CTXCLI-3002)

The MCP tool must map the Rust response to the enhanced format.

**Important**: The Rust `ContextItem` struct does NOT contain `worktree` or `repo` fields. These must be passed from the MCP request context (derived from prior search results or chunk_id lookup).

```typescript
function mapRustToMcpBundle(
  rustBundle: RustContextBundle,
  requestParams: ContextParams,
  requestContext: { worktree: string; repo: string }  // From MCP request context
): ContextBundle {
  const budgetTokens = requestParams.budget_tokens ?? 6000
  return {
    items: rustBundle.items,
    total_tokens: rustBundle.total_tokens,
    budget_tokens: budgetTokens,
    budget_remaining: budgetTokens - rustBundle.total_tokens,
    truncated: rustBundle.truncated,
    metadata: {
      worktree: requestContext.worktree,
      repo: requestContext.repo,
    },
  }
}
```

#### Example Daemon Response

```json
{
  "items": [
    {
      "relpath": "src/components/Auth.tsx",
      "range": { "start": 10, "end": 45 },
      "role": "primary",
      "reason": "Target chunk requested by user",
      "content": "export function Auth() { ... }",
      "tokens": 250
    },
    {
      "relpath": "src/hooks/useAuth.ts",
      "range": { "start": 5, "end": 30 },
      "role": "hook",
      "reason": "Hook used by primary component",
      "content": "export function useAuth() { ... }",
      "tokens": 150
    }
  ],
  "total_tokens": 400,
  "truncated": false
}
```

## Data Flow

### CLI Flow
```
User runs: crewchief-maproom context --chunk-id 12345 --budget 6000 --callers

1. Parse CLI arguments → ExpandOptions
2. Create SqliteStore connection
3. Create BasicContextAssembler
4. assembler.assemble(chunk_id, budget, options)
5. Print formatted output or JSON
```

### MCP Flow
```
MCP client calls context tool with chunk_id

1. MCP server receives request
2. Daemon client sends JSON-RPC: {"method": "context", "params": {...}}
3. Rust daemon receives request
4. Parse params → ExpandOptions
5. BasicContextAssembler.assemble()
6. Return JSON-RPC response
7. MCP server returns result to client
```

## Caching Strategy

The `BasicContextAssembler` already includes LRU caching via `ContextCache`:

```rust
// Cache key: (chunk_id, budget, expand_options_hash)
// Cache size: 100 bundles (default)
// TTL: None (eviction by LRU)
```

The daemon maintains the assembler instance across requests, enabling cache reuse.

## Error Handling

### Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error (invalid JSON) |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32000 | Chunk not found |
| -32001 | File not found |
| -32002 | Budget exceeded |

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Chunk not found with id 12345",
    "data": {
      "chunk_id": 12345,
      "hint": "Use the search tool to find valid chunks"
    }
  }
}
```

## File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `crates/maproom/src/main.rs` | Modify | Add `Context` command variant |
| `crates/maproom/src/daemon/types.rs` | Modify | Add `ContextParams`, `ExpandConfig` |
| `crates/maproom/src/daemon/mod.rs` | Modify | Add `context` handler, `execute_context()` |
| `packages/maproom-mcp/src/tools/context.ts` | Modify | Replace PostgreSQL with daemon client |
| `packages/maproom-mcp/src/tools/context_schema.ts` | Modify | Add React-specific expand options |

## Testing Strategy

1. **Unit Tests** - Test `execute_context()` with mock store
2. **Integration Tests** - Test CLI command with test database
3. **E2E Tests** - Test MCP tool via daemon client
