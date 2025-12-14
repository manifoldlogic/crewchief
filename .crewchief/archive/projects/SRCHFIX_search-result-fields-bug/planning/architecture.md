# Architecture: Search Result Fields Bug Fix

## Overview

This is a straightforward data plumbing fix in the daemon JSON-RPC layer. The architecture doesn't change - we're just completing the type synchronization that was partially done during the daemon migration.

```
┌─────────────────┐
│   MCP Client    │
│  (TypeScript)   │
└────────┬────────┘
         │ JSON-RPC over stdio
         │
┌────────▼────────┐
│  Rust Daemon    │
│  (crewchief-    │
│   maproom)      │
└────────┬────────┘
         │ SQL queries
         │
┌────────▼────────┐
│ SQLite Database │
│  (maproom.db)   │
└─────────────────┘
```

**Fix location**: The JSON serialization layer between Rust daemon and TypeScript client.

## Design Decisions

### Decision 1: Field Naming Convention

**Options considered**:

1. **Use `chunk_id` everywhere** (Rust convention)
   - ✓ Matches Rust struct field name
   - ✓ Matches database column name
   - ✓ More descriptive (clearly an ID, not an index)
   - ✗ Requires updating TypeScript interface

2. **Use `chunk_index` everywhere** (current TypeScript)
   - ✓ Matches existing TypeScript interface
   - ✗ Misleading name (it's an ID, not an array index)
   - ✗ Inconsistent with Rust and database

3. **Support both names** (aliasing)
   - ✗ Complexity for minimal benefit
   - ✗ Ambiguity in documentation

**Decision**: Use `chunk_id` everywhere.

**Rationale**:
- Rust is the source of truth for daemon types (per CLAUDE.md)
- `chunk_id` is more semantically accurate (it's a database ID, not an index)
- The TypeScript interface was scaffolded during migration but never used correctly
- No known consumers depend on `chunk_index` (the field was always 0)

### Decision 2: JSON Field Names

**Decision**: Match Rust struct field names exactly in JSON.

**Field mapping**:
```
Rust struct field → JSON field name
----------------------------------------
chunk_id          → "chunk_id"
symbol_name       → "symbol_name"
kind              → "kind"
file_relpath      → "file_path"  (existing, keep for compatibility)
```

**Rationale**:
- Minimizes cognitive load (TypeScript mirrors Rust)
- Follows existing pattern in codebase
- `file_path` already differs from `file_relpath` for backwards compatibility

### Decision 3: Null Handling

**Decision**: Preserve Rust's Option<String> semantics in JSON.

**Mapping**:
```rust
// Rust
symbol_name: Option<String>  // None for anonymous chunks

// JSON
"symbol_name": null  // or "symbol_name": "actual_name"

// TypeScript
symbol_name: string | null
```

**Rationale**:
- Database schema allows NULL for symbol_name (anonymous code blocks)
- JSON null is more accurate than empty string for "no value"
- TypeScript can handle union types properly

## Technology Choices

No new technologies introduced. Using existing stack:

| Component | Choice | Rationale |
|-----------|--------|-----------|
| JSON Serialization | serde_json | Already used in Rust daemon |
| JSON Parsing | Native TypeScript | Already used in daemon-client |
| Type Validation | TypeScript compiler | Compile-time type safety |
| Communication | JSON-RPC over stdio | Existing daemon protocol |

## Component Design

### Component 1: Rust Daemon JSON Serialization

**File**: `crates/maproom/src/daemon/mod.rs`

**Current code** (line 332-340):
```rust
.map(|hit| {
    serde_json::json!({
        "score": hit.score,
        "start_line": hit.start_line,
        "end_line": hit.end_line,
        "symbol_name": hit.symbol_name,
        "kind": hit.kind,
        "file_path": hit.file_relpath,
    })
})
```

**Updated code**:
```rust
.map(|hit| {
    serde_json::json!({
        "chunk_id": hit.chunk_id,        // ADD
        "score": hit.score,
        "start_line": hit.start_line,
        "end_line": hit.end_line,
        "symbol_name": hit.symbol_name,  // Already present
        "kind": hit.kind,                // Already present
        "file_path": hit.file_relpath,
    })
})
```

**Change**: Add one line to serialize `chunk_id`.

### Component 2: TypeScript Daemon Client Interface

**File**: `packages/daemon-client/src/client.ts`

**Current interface** (line 29-41):
```typescript
export interface SearchResult {
  hits: Array<{
    file_path: string
    chunk_index: number
    start_line: number
    end_line: number
    content: string
    score: number
  }>
  total: number
  // ...
}
```

**Updated interface**:
```typescript
/**
 * Search result from daemon
 *
 * Sync with: crates/maproom/src/db/mod.rs SearchHit
 */
export interface SearchResult {
  hits: Array<{
    chunk_id: number           // RENAME from chunk_index
    file_path: string
    start_line: number
    end_line: number
    symbol_name: string | null // ADD
    kind: string               // ADD
    content: string
    score: number
  }>
  total: number
  query_embedding_time_ms?: number
  search_time_ms?: number
}
```

**Changes**:
1. Rename `chunk_index` → `chunk_id`
2. Add `symbol_name: string | null`
3. Add `kind: string`
4. Add sync comment pointing to Rust struct

### Component 3: RustSearchHit Interface (Already Correct)

**File**: `packages/maproom-mcp/src/tools/search.ts` (line 108-118)

**Status**: This interface already has the correct structure and needs no changes.

**Current interface**:
```typescript
interface RustSearchHit {
  score: number
  file_relpath: string
  symbol_name: string | null  // ✓ Already present
  kind: string                // ✓ Already present
  start_line: number
  end_line: number
  base_score?: number
  kind_mult?: number
  exact_mult?: number
}
```

**Note**: This interface is used for the internal mapping between daemon output and MCP format. The symbol_name and kind fields are already correctly typed - we just need to update the mapping code to use the daemon values instead of hardcoded empty strings.

### Component 4: TypeScript Mapping Code

**File**: `packages/maproom-mcp/src/tools/search.ts`

**Current code** (line 307-318):
```typescript
const rustOutput: RustSearchOutput = {
  hits: daemonResult.hits.map((hit) => ({
    file_relpath: hit.file_path,
    start_line: hit.start_line,
    end_line: hit.end_line,
    symbol_name: '', // Hardcoded
    kind: '',        // Hardcoded
    score: hit.score,
    // ...
  })),
}
```

**Updated code**:
```typescript
const rustOutput: RustSearchOutput = {
  hits: daemonResult.hits.map((hit) => ({
    file_relpath: hit.file_path,
    start_line: hit.start_line,
    end_line: hit.end_line,
    symbol_name: hit.symbol_name || '', // Use actual value, fallback to empty
    kind: hit.kind,                     // Use actual value
    score: hit.score,
    base_score: undefined,
    kind_mult: undefined,
    exact_mult: undefined,
  })),
}
```

**Changes**:
1. Use `hit.symbol_name || ''` (convert null to empty string for backward compatibility)
2. Use `hit.kind` directly

### Component 5: Obsolete Fallback Code Removal

**File**: `packages/maproom-mcp/src/tools/search.ts`

**Current code** (line 323-334):
```typescript
// Chunk IDs are not available from SQLite daemon
// Legacy PostgreSQL code path has been removed
const chunkIdMap = new Map<string, number>()

// Transform Rust hits to SearchResult format
const hits: SearchResult[] = rustOutput.hits.map((hit) => {
  const key = `${hit.file_relpath}:${hit.start_line}:${hit.end_line}`
  const chunk_id = chunkIdMap.get(key) || 0

  if (chunk_id === 0) {
    log.warn({ hit }, 'Chunk ID not found for search result')
  }
  // ...
})
```

**Updated code**:
```typescript
// Transform Rust hits to SearchResult format - daemon provides chunk_id directly
const hits: SearchResult[] = rustOutput.hits.map((hit, index) => {
  const daemonHit = daemonResult.hits[index]

  // Validate chunk_id is present
  if (!daemonHit.chunk_id || daemonHit.chunk_id === 0) {
    log.warn({ hit: daemonHit }, 'Invalid chunk_id in search result')
  }

  // Build SearchResult with optional score_breakdown
  const result: SearchResult = {
    chunk_id: daemonHit.chunk_id,  // Use daemon value directly
    symbol_name: hit.symbol_name,
    kind: hit.kind,
    // ... rest of fields
  }
  // ...
})
```

**Changes**:
1. Remove `chunkIdMap` (no longer needed)
2. Remove misleading comments about chunk IDs not being available
3. Get `chunk_id` directly from daemon response
4. Update warning message to reflect actual issue (invalid ID vs not found)

## Data Flow

### Before (Broken)

```
Database → Rust SearchHit {chunk_id: 123, symbol_name: "foo", kind: "function"}
    ↓
Daemon JSON {"score": 0.9, "start_line": 10, ...}  ← Missing chunk_id
    ↓
TypeScript {chunk_index: 0, symbol_name: '', kind: ''}  ← Hardcoded defaults
    ↓
MCP Response {chunk_id: 0, ...}  ← Broken
```

### After (Fixed)

```
Database → Rust SearchHit {chunk_id: 123, symbol_name: "foo", kind: "function"}
    ↓
Daemon JSON {"chunk_id": 123, "symbol_name": "foo", "kind": "function", ...}
    ↓
TypeScript {chunk_id: 123, symbol_name: 'foo', kind: 'function'}
    ↓
MCP Response {chunk_id: 123, symbol_name: 'foo', kind: 'function'}  ← Fixed
```

## Integration Points

### Existing Systems

1. **MCP Context Tool**: Requires valid chunk_id to retrieve context. This fix unblocks it.

2. **VSCode Extension**: May display symbol_name and kind in search results UI (currently shows empty).

3. **Search Ranking**: kind field is used for semantic ranking multipliers (currently ignored due to empty value).

### No Changes Required

1. **Database schema**: Already has all fields
2. **Search query logic**: Already retrieves all fields
3. **Rust SearchHit struct**: Already has all fields

## Performance Considerations

**Impact**: Negligible

**Rationale**:
- Adding fields to JSON response has minimal serialization cost
- Fields already exist in memory (SearchHit struct)
- No additional database queries needed

## Maintainability

**Improvements**:
1. **Type synchronization**: Adding sync comments prevents future drift
2. **Removing obsolete code**: Cleaner codebase, less confusion
3. **Accurate comments**: Future developers won't be misled

**Documentation**:
- Add sync comment to TypeScript interface pointing to Rust struct
- Remove misleading "Phase 2 enhancement" comments
- Update field names to match Rust conventions
