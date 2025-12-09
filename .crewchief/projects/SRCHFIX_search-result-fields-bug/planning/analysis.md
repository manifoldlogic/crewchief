# Analysis: Search Result Fields Bug

## Problem Definition

Search results from the Maproom MCP server are missing critical metadata fields that exist in the database and are needed by clients:

1. **chunk_id always 0**: Prevents context retrieval, linking results to specific code chunks
2. **symbol_name always empty**: Hides function/class/method names in search results
3. **kind always empty**: Loses symbol type information (function, class, method, etc.)

These fields are essential for:
- **Context retrieval**: chunk_id is required to fetch related code via the context tool
- **Result quality**: symbol_name and kind provide semantic information for ranking and filtering
- **User experience**: Showing function names and types helps users understand what they found

## Context and Background

### How We Got Here

This bug was introduced during the daemon migration (DAEMON project). The migration moved from direct database queries in TypeScript to a Rust daemon that communicates via JSON-RPC.

**Pre-daemon architecture** (working):
```
MCP Server → Direct DB queries → Full SearchHit data
```

**Post-daemon architecture** (broken):
```
MCP Server → Daemon (JSON-RPC) → DB queries
            ↑ Type mismatch here
```

The Rust daemon correctly queries the database and has all the data in the `SearchHit` struct, but the JSON serialization and TypeScript interface were incompletely updated.

### Existing Code Structure

**Rust side** (`crates/maproom/src/db/mod.rs:86-101`):
```rust
pub struct SearchHit {
    pub chunk_id: i64,          // ✓ Has the data
    pub score: f64,
    pub file_relpath: String,
    pub symbol_name: Option<String>,  // ✓ Has the data
    pub kind: String,           // ✓ Has the data
    pub start_line: i32,
    pub end_line: i32,
    // ... debug fields
}
```

**Rust daemon JSON serialization** (`crates/maproom/src/daemon/mod.rs:332-340`):
```rust
.map(|hit| {
    serde_json::json!({
        "score": hit.score,
        "start_line": hit.start_line,
        "end_line": hit.end_line,
        "symbol_name": hit.symbol_name,  // ✓ Serialized
        "kind": hit.kind,                // ✓ Serialized
        "file_path": hit.file_relpath,
        // ❌ Missing: "chunk_index" (or "chunk_id")
    })
})
```

**TypeScript interface** (`packages/daemon-client/src/client.ts:29-41`):
```typescript
export interface SearchResult {
  hits: Array<{
    file_path: string
    chunk_index: number  // ✓ Field exists
    start_line: number
    end_line: number
    content: string
    score: number
    // ❌ Missing: symbol_name
    // ❌ Missing: kind
  }>
  total: number
  // ...
}
```

**TypeScript mapping code** (`packages/maproom-mcp/src/tools/search.ts:307-318`):
```typescript
const rustOutput: RustSearchOutput = {
  hits: daemonResult.hits.map((hit) => ({
    file_relpath: hit.file_path,
    start_line: hit.start_line,
    end_line: hit.end_line,
    symbol_name: '', // ❌ Hardcoded empty
    kind: '',        // ❌ Hardcoded empty
    score: hit.score,
    // ...
  })),
}
```

**Obsolete fallback code** (`packages/maproom-mcp/src/tools/search.ts:323-334`):
```typescript
// Chunk IDs are not available from SQLite daemon
// Legacy PostgreSQL code path has been removed
const chunkIdMap = new Map<string, number>() // ❌ Always empty map
```

## Current State

The system is currently in a broken state where:

1. Database has all the data (chunk_id, symbol_name, kind)
2. Rust queries retrieve all the data into SearchHit struct
3. Daemon JSON serialization omits chunk_id (but includes symbol_name and kind)
4. TypeScript interface expects chunk_index but omits symbol_name and kind
5. Mapping code hardcodes empty strings for symbol_name and kind
6. Mapping code uses empty Map for chunk_id lookup, resulting in 0

## Research Findings

### Root Cause Analysis

1. **Incomplete type synchronization**: When creating the daemon JSON-RPC interface, the TypeScript types weren't fully aligned with Rust struct fields

2. **Migration artifact**: The comment "Daemon doesn't return symbol_name yet (Phase 2 enhancement)" indicates this was intentionally deferred, but the Rust code was already serializing these fields

3. **Incorrect assumption about chunk_id**: The code assumes chunk_id isn't available from the daemon, when it actually is available in the Rust SearchHit struct

### Field Name Mismatch

- Rust uses `chunk_id` in the struct
- TypeScript expects `chunk_index` (based on the interface)
- Neither is being serialized in the daemon JSON response

### Misleading Comments

Comments in search.ts suggested these were "Phase 2 enhancements", discouraging fixes and perpetuating the assumption that the daemon doesn't support these fields.

## Existing Solutions

This is a project-specific bug with no industry patterns to reference. The solution is straightforward: complete the type synchronization that was started during the daemon migration.

## Constraints

### Technical Constraints

1. **Type synchronization**: Must maintain consistency between Rust and TypeScript types (documented requirement in CLAUDE.md)

2. **Backward compatibility**: Existing consumers of the MCP server may expect certain field names

3. **Field naming convention**: Need to decide between `chunk_id` (Rust) vs `chunk_index` (TypeScript)

### Business Constraints

1. **High priority**: This blocks context retrieval functionality entirely (chunk_id=0 is invalid)

2. **Low risk**: Changes are localized to serialization/deserialization layer

3. **No data migration**: Database already has all the data, just need to expose it

## Success Criteria

### Functional Requirements

1. **chunk_id/chunk_index present**: Search results include valid chunk IDs from database
2. **symbol_name populated**: Function/class/method names appear in results
3. **kind populated**: Symbol types (function, class, method, etc.) appear in results

### Validation Requirements

1. **Integration test**: Search returns all three fields with non-default values
2. **Context retrieval works**: Can fetch context using chunk_id from search results
3. **Type safety**: TypeScript compilation succeeds with updated types

### Non-Functional Requirements

1. **No performance impact**: Serialization of existing fields should have negligible cost
2. **Clear field naming**: Settle on consistent naming convention (document the choice)
3. **Remove misleading comments**: Update comments to reflect actual daemon capabilities

## Assumptions

1. **Rust SearchHit is complete**: The Rust struct has all necessary fields populated from database queries
2. **Database has data**: symbol_name and kind are populated in the chunks table
3. **Field name flexibility**: Can choose either chunk_id or chunk_index as the canonical name
4. **No breaking changes**: Adding fields to JSON response won't break existing clients (fields are additive)
