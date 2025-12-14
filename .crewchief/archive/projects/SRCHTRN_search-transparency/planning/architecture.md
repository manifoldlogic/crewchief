# Architecture: Search Transparency

## Overview

Replace generic RPC_ERROR messages with structured, actionable error diagnostics and add query understanding feedback to maproom's search pipeline.

**Core Principle**: Leverage existing data structures, add serialization layer for transparency.

```
┌─────────────────────────────────────────────────────────────┐
│                    Search Request Flow                       │
└─────────────────────────────────────────────────────────────┘

MCP Client (TypeScript)
    ↓ Zod validation (catch client-side errors)
Daemon Client (TypeScript)
    ↓ JSON-RPC over Unix socket
Daemon RPC Handler (Rust)
    ↓ Dispatch to search handler
Search Pipeline (Rust)
    ├→ Query Processing → QueryUnderstanding metadata
    ├→ Search Execution → timing, source counts
    ├→ Score Fusion → fusion strategy
    └→ Result Assembly → enriched results
    ↓
Response with metadata OR structured error
    ↓ JSON-RPC serialization
TypeScript Client
    ↓ Deserialize to typed structures
MCP Tool
    ↓ Format for user display
User sees: actionable errors + query understanding
```

## Existing Infrastructure

**Daemon Infrastructure Verified** (addresses review concern):

The daemon RPC infrastructure exists and is ready for extension:

- **`crates/maproom/src/daemon/mod.rs`** (469 lines)
  - Main daemon logic with JSON-RPC request handling
  - `execute_search()` function (lines 213-354) - entry point for adding error serialization
  - `handle_request()` function (lines 121-211) - where structured errors will be returned

- **`crates/maproom/src/daemon/server.rs`**
  - Unix socket server implementation
  - PID file management with locking
  - Session registry for concurrent clients

- **`crates/maproom/src/daemon/types.rs`**
  - `SearchParams`, `ContextParams` already defined
  - `JsonRpcRequest`, `JsonRpcResponse` structures
  - Extension point: Add `SearchErrorDetails` here

- **`crates/maproom/src/daemon/protocol.rs`**
  - JSON-RPC codec (JsonRpcCodec)
  - Message framing over Unix sockets

- **`crates/maproom/src/daemon/session.rs`**
  - Session management and cleanup

**Current Error Handling** (lines 143-151 in mod.rs):
```rust
Err(e) => {
    error!("Search failed: {}", e);
    JsonRpcResponse::error(
        id,
        -32000,
        "Search failed".to_string(),
        Some(serde_json::json!(e.to_string())),
    )
}
```

**Extension Point**: Replace `e.to_string()` in `data` field with serialized `SearchErrorDetails`.

## Design Decisions

### Decision 1: Additive API Changes Only

**Context**: Need to enhance error reporting without breaking existing clients.

**Decision**: Add optional fields to existing response structures. Extend JSON-RPC error `data` field with structured information.

**Rationale**:
- Maintains backward compatibility
- Existing clients ignore unknown fields
- Progressive enhancement - new clients get better errors
- No version negotiation needed

Example:
```typescript
// Before (still valid)
interface SearchResult {
  hits: Hit[]
  total: number
}

// After (backward compatible)
interface SearchResult {
  hits: Hit[]
  total: number
  metadata?: QueryUnderstanding  // NEW: optional
}
```

### Decision 2: Single Source of Truth (Rust)

**Context**: TypeScript and Rust types must stay in sync. Divergence causes serialization bugs.

**Decision**: Rust defines canonical types. TypeScript types mirror Rust with sync comments.

**Rationale**:
- Rust daemon owns search logic
- Serde serialization is authoritative
- Comments link corresponding types for auditing
- Manual sync acceptable for small number of types (2-3 structures)

Example:
```rust
// crates/maproom/src/search/errors.rs
#[derive(Serialize)]
pub struct SearchErrorDetails {
    pub error_type: ErrorType,
    // ...
}
```

```typescript
// packages/daemon-client/src/types.ts
// Sync with: crates/maproom/src/search/errors.rs::SearchErrorDetails
export interface SearchErrorDetails {
  error_type: ErrorType
  // ...
}
```

### Decision 3: No Over-Engineering

**Context**: Risk of building generic error framework for 5-6 error cases.

**Decision**: Pragmatic conversion functions. String-based suggestions. No abstraction layers.

**Rationale**:
- MVP focus - ship value, not ceremonies
- 5-6 error types don't justify framework
- String suggestions provide 90% of value
- Clear conversion logic is maintainable
- Can add structure later if needed

**Non-Goals**:
- Generic error abstraction layer
- Complex suggestion engine
- Error recovery mechanisms
- Automatic retry logic

### Decision 4: Performance First

**Context**: <10ms overhead budget for metadata assembly.

**Decision**: Use existing in-memory data. No additional database queries. Lazy evaluation where possible.

**Rationale**:
- Query understanding data already computed in `ProcessedQuery`
- Timing data already tracked in `SearchMetadata`
- Error context extracted from existing error types
- Just expose what exists - no new computation

**Measurement**:
- Before/after metrics comparison via Prometheus
- p95 latency must remain <100ms
- Overhead target: ~3ms (well under 10ms budget)

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Error Taxonomy | Rust enum with serde | Type-safe, serializes to JSON, single source of truth |
| Type Sync | Manual with comments | Small number of types, clear audit trail, no build complexity |
| Suggestions | String arrays | Simple, sufficient for MVP, no complex suggestion engine |
| Metadata | Optional fields | Backward compatible, progressive enhancement |
| Serialization | serde_json | Fast, battle-tested, already used everywhere |

**Why Not Auto-Generated Types?**

Considered: typescript-json-schema, ts-rs, json-schema-to-typescript

Decision: Manual sync with documentation comments

Rationale:
- Only 2-3 structures to sync
- Build complexity not worth it for MVP
- Manual sync with comments is clear and auditable
- Can add codegen later if types proliferate

## Component Design

### 1. Error Taxonomy (Rust)

**Location**: `crates/maproom/src/search/errors.rs` (new file)

**Responsibilities**:
- Define structured error types
- Convert PipelineError to SearchErrorDetails
- Generate actionable suggestions

**Note**: Error types will be audited in Phase 1 to verify PipelineError structure contains sufficient context. May require minor refactoring for better context extraction.

**Error Scenario Mapping** (13 observed scenarios → 6 error types):

```
ErrorType::EmbeddingProvider (3 scenarios)
  - OpenAI API timeout
  - Google credentials missing
  - Ollama service not running

ErrorType::Database (4 scenarios)
  - Repository not indexed
  - Worktree not found
  - Corrupted SQLite database
  - Database connection timeout

ErrorType::Validation (2 scenarios)
  - Empty query
  - Query too long (>1000 chars)

ErrorType::Timeout (1 scenario)
  - Search execution timeout

ErrorType::NotFound (2 scenarios)
  - Repository not found
  - No meaningful content in query

ErrorType::Unknown (1+ scenarios)
  - Unexpected errors not matching above categories
```

**Interface**:
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchErrorDetails {
    pub error_type: ErrorType,
    pub stage: PipelineStage,
    pub context: HashMap<String, String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    EmbeddingProvider,
    Database,
    Validation,
    Timeout,
    NotFound,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    QueryProcessing,
    SearchExecution,
    ScoreFusion,
    ResultAssembly,
}

impl SearchErrorDetails {
    pub fn from_pipeline_error(error: &PipelineError) -> Self;
}
```

**Key Logic**:
- Pattern match on `PipelineError` variants
- Extract context from error types
- Map to high-level error types
- Generate 2-3 actionable suggestions per error

### 2. Query Understanding Metadata (Rust)

**Location**: `crates/maproom/src/search/results.rs` (extend existing)

**Responsibilities**:
- Assemble query understanding from ProcessedQuery
- Collect timing breakdown
- Track filters applied

**Interface**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryUnderstanding {
    pub mode: SearchMode,
    pub tokens: Vec<String>,
    pub expanded_terms: Vec<String>,
    pub filters: QueryFilters,
    pub fusion_strategy: String,
    pub timing: TimingBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilters {
    pub repo_id: i64,
    pub worktree_id: Option<i64>,
    pub file_types: Vec<String>,
    pub recency_threshold: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingBreakdown {
    pub query_processing_ms: f64,
    pub search_execution_ms: f64,
    pub score_fusion_ms: f64,
    pub result_assembly_ms: f64,
    pub total_ms: f64,
}

// Extend existing SearchMetadata
pub struct SearchMetadata {
    // ... existing fields ...

    #[serde(skip_serializing_if = "Option::is_none")]
    pub understanding: Option<QueryUnderstanding>,
}
```

**Key Logic**:
- Assemble from existing `ProcessedQuery` data
- Copy timing data from `SearchTiming`
- No new computation - just expose existing data

### 3. JSON-RPC Error Serialization (Rust)

**Location**: `crates/maproom/src/daemon/server.rs` (modify existing handler)

**Responsibilities**:
- Catch PipelineError
- Convert to SearchErrorDetails
- Serialize as JSON-RPC error with data field

**Interface**:
```rust
async fn handle_search_request(
    params: SearchParams,
    state: Arc<DaemonState>,
    request_id: i64,
) -> JsonRpcResponse {
    match execute_search(params, &state).await {
        Ok(results) => {
            JsonRpcResponse::success(
                request_id,
                serde_json::to_value(results)?,
            )
        }
        Err(error) => {
            let error_details = SearchErrorDetails::from_pipeline_error(&error);
            JsonRpcResponse::error(
                request_id,
                -32000, // Application error code
                error.to_string(),
                Some(serde_json::to_value(error_details)?),
            )
        }
    }
}
```

**Key Logic**:
- Use JSON-RPC 2.0 error structure
- Put SearchErrorDetails in `data` field
- Preserve human-readable message for backward compat

### 4. TypeScript Error Types (daemon-client)

**Location**: `packages/daemon-client/src/types.ts` (new file)

**Responsibilities**:
- Define TypeScript interfaces matching Rust types
- Document sync points with comments

**Interface**:
```typescript
// Sync with: crates/maproom/src/search/errors.rs

export type ErrorType =
  | 'embedding_provider'
  | 'database'
  | 'validation'
  | 'timeout'
  | 'not_found'
  | 'unknown'

export type PipelineStage =
  | 'query_processing'
  | 'search_execution'
  | 'score_fusion'
  | 'result_assembly'

export interface SearchErrorDetails {
  error_type: ErrorType
  stage: PipelineStage
  context: Record<string, string>
  suggestions: string[]
}

export interface QueryUnderstanding {
  mode: 'code' | 'text' | 'auto'
  tokens: string[]
  expanded_terms: string[]
  filters: QueryFilters
  fusion_strategy: string
  timing: TimingBreakdown
}
```

**Key Logic**:
- Exact match with Rust types
- Comments link to Rust source of truth
- Export for use in daemon-client and maproom-mcp

### 5. TypeScript Error Deserialization (daemon-client)

**Location**: `packages/daemon-client/src/rpc.ts` (extend existing)

**Responsibilities**:
- Parse SearchErrorDetails from JSON-RPC error data
- Provide helper methods for user-friendly messages

**Interface**:
```typescript
export class RpcError extends DaemonError {
  public readonly rpcCode: number
  public readonly details?: SearchErrorDetails

  constructor(message: string, rpcCode: number, data?: unknown) {
    super(message, 'RPC_ERROR')
    this.rpcCode = rpcCode

    // Attempt to parse structured error details
    if (data && typeof data === 'object') {
      this.details = data as SearchErrorDetails
    }
  }

  getDetails(): SearchErrorDetails | undefined {
    return this.details
  }

  getUserMessage(): string {
    // Format error with context and suggestions
  }
}
```

**Key Logic**:
- Parse `data` field from JSON-RPC error
- Provide `getUserMessage()` for formatting
- Fallback to generic error if no details

### 6. MCP Tool Error Formatting (maproom-mcp)

**Location**: `packages/maproom-mcp/src/tools/search.ts` (modify existing)

**Responsibilities**:
- Format structured errors for MCP protocol
- Display query understanding in success responses

**Note**: Client-side validation (Zod) already catches common errors before RPC. Phase 3 improves error message quality, not validation coverage.

**Interface**:
```typescript
export function formatSearchError(error: unknown): MCPError {
  if (error instanceof RpcError) {
    const details = error.getDetails()

    if (details) {
      return {
        isError: true,
        content: [
          {
            type: 'text',
            text: JSON.stringify(
              {
                error: details.error_type,
                stage: details.stage,
                message: error.message,
                context: details.context,
                suggestions: details.suggestions,
              },
              null,
              2
            ),
          },
        ],
      }
    }
  }

  // Fallback to generic error
}
```

**Key Logic**:
- Check for RpcError with details
- Format structured response
- Fallback to existing error handling

## Data Flow

### Success Case: Query Understanding

```
User: "authenticate user"
  ↓
MCP Tool: Zod validation ✓
  ↓
Daemon Client: JSON-RPC request
  ↓
Rust Pipeline:
  1. QueryProcessor:
     - tokens=["authenticate", "user"]
     - mode=Auto
     - expanded=["auth", "login", "authentication"]
  2. SearchExecutors:
     - FTS: 10 results
     - Vector: 8 results
     - Graph: 2 results
     - Signals: 0 results
  3. ScoreFusion:
     - Strategy: reciprocal_rank_fusion
     - Weights: {fts: 0.6, vector: 0.4}
  4. ResultAssembly:
     - 10 results enriched with chunk details
  ↓
Response JSON:
{
  "hits": [...],
  "total": 10,
  "metadata": {
    "understanding": {
      "mode": "auto",
      "tokens": ["authenticate", "user"],
      "expanded_terms": ["auth", "login", "authentication"],
      "filters": {
        "repo_id": 1,
        "worktree_id": 2,
        "file_types": [],
        "recency_threshold": null
      },
      "fusion_strategy": "reciprocal_rank_fusion",
      "timing": {
        "query_processing_ms": 4.2,
        "search_execution_ms": 35.8,
        "score_fusion_ms": 2.1,
        "result_assembly_ms": 6.4,
        "total_ms": 48.5
      }
    }
  }
}
```

### Error Case: Embedding Provider Unavailable

```
User: "authenticate user" (vector mode)
  ↓
MCP Tool: Zod validation ✓
  ↓
Daemon Client: JSON-RPC request
  ↓
Rust Pipeline:
  1. QueryProcessor:
     - tokenization: ✓
     - embedding: ✗ (OpenAI API timeout)
     - Error: PipelineError::QueryProcessing(Embedding(timeout))
  ↓
Error Handler:
  - Convert to SearchErrorDetails
  - error_type: "embedding_provider"
  - stage: "query_processing"
  - suggestions: ["Check credentials", "Verify network", "Try FTS mode"]
  ↓
Error Response JSON:
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Query processing failed: Embedding generation failed: request timeout",
    "data": {
      "error_type": "embedding_provider",
      "stage": "query_processing",
      "context": {
        "provider_error": "request timeout"
      },
      "suggestions": [
        "Check your embedding provider credentials",
        "Verify network connectivity",
        "Try FTS mode while debugging: --mode fts"
      ]
    }
  },
  "id": 1
}
  ↓
TypeScript Client:
  - Deserialize to RpcError
  - Parse details from data field
  ↓
MCP Tool:
  - Format structured error
  - Include suggestions
  ↓
User sees:
"Search failed at query_processing: Embedding generation failed: request timeout

Context:
  provider_error: request timeout

Suggestions:
  - Check your embedding provider credentials
  - Verify network connectivity
  - Try FTS mode while debugging: --mode fts"
```

## Integration Points

### With Existing Codebase

**Search Pipeline** (`crates/maproom/src/search/pipeline.rs`):
- Add `QueryUnderstanding` assembly after query processing
- Collect timing breakdown (extend existing timing)
- Wrap errors with `SearchErrorDetails::from_pipeline_error()`
- Changes: ~50 lines

**Daemon Server** (`crates/maproom/src/daemon/server.rs`):
- Modify search handler to serialize error details
- Add `data` field to JSON-RPC error responses
- Changes: ~20 lines

**Daemon Client** (`packages/daemon-client/src/rpc.ts`):
- Extend `RpcError` with `details` field
- Add `getUserMessage()` helper
- Changes: ~30 lines

**MCP Tool** (`packages/maproom-mcp/src/tools/search.ts`):
- Update `formatSearchError()` to use structured details
- Add query understanding to success responses
- Changes: ~40 lines

### Type Synchronization

**Manual Sync Points**:
1. `SearchErrorDetails` (Rust) ↔ `SearchErrorDetails` (TypeScript)
2. `QueryUnderstanding` (Rust) ↔ `QueryUnderstanding` (TypeScript)
3. Error type enums must match exactly
4. Pipeline stage enums must match exactly

**Documentation Pattern**:
```typescript
// Sync with: crates/maproom/src/search/errors.rs::SearchErrorDetails
export interface SearchErrorDetails {
  // Field comments also synced
  error_type: ErrorType
  // ...
}
```

**Validation Strategy**:
- Integration tests serialize/deserialize across boundary
- Test cases for each error type
- CI validation of type consistency

## Performance Considerations

### Budget Allocation

**Total Overhead Target**: <10ms

**Breakdown**:
- Query understanding assembly: ~1ms (copying existing data from ProcessedQuery)
- Timing collection: ~0ms (already tracked)
- Error detail assembly: ~0.5ms (only on error path)
- JSON serialization: ~2ms (serde is fast)
- Network overhead: ~0ms (already paid)

**Total**: ~3.5ms << 10ms budget ✓

### Optimization Strategies

1. **Lazy Evaluation**: Only assemble metadata if needed
2. **Reuse Existing Data**: Zero new computation
3. **Skip on Errors**: Error path can afford slightly higher latency
4. **Struct Cloning**: Cheap for small structures

### Performance Validation

**Before/After Metrics**:
- Measure p50, p95, p99 latency
- Compare with and without metadata
- Use existing Prometheus metrics
- Target: no regression beyond noise

**Test Scenarios**:
- Empty query (validation error)
- FTS search (fast path)
- Vector search (embedding overhead)
- Hybrid search (full pipeline)

## Maintainability

### Type Sync Maintenance

**Onboarding Checklist**:
1. Read CLAUDE.md for type sync rules
2. Check for sync comments when modifying types
3. Update both Rust and TypeScript
4. Run integration tests

**IDE Support**:
- Comments link to source of truth
- TypeScript LSP shows documentation
- Rust docs generate API reference

### Extension Points

**Adding New Error Types**:
1. Add variant to `ErrorType` enum (Rust)
2. Add conversion case in `from_pipeline_error()`
3. Add suggestions for new error
4. Mirror changes in TypeScript
5. Add test case

**Adding New Metadata Fields**:
1. Add field to `QueryUnderstanding` (Rust)
2. Populate in pipeline assembly
3. Mirror in TypeScript
4. Optional field for backward compat
5. Update documentation

### Testing Strategy

**Unit Tests**:
- Error conversion logic
- Suggestion generation
- Metadata assembly

**Integration Tests**:
- End-to-end error serialization
- TypeScript deserialization
- Type consistency validation

**Manual Tests**:
- Error scenarios (embedding offline, repo not found)
- Query understanding visibility
- Performance regression checks
