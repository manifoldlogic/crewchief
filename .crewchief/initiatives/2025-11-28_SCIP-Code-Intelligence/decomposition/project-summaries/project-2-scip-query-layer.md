# Project: SCIP Query Layer

## Project Summary

Build a query API on top of the SCIP SQLite database (from Project 1) that answers the core code intelligence questions: definition lookup, reference finding, and symbol information retrieval. This project creates a Rust API that can be called programmatically and tested independently, without any MCP integration.

The query layer translates developer questions ("where is this defined?", "what calls this function?") into efficient SQLite queries against the pre-computed SCIP index.

## Core Criteria Assessment

### Interface Stability 🔒

**External Interfaces:**
- **Input**: SQLite database from Project 1 (schema under your control)
- **Output**: Rust structs defined in this project

**Stability Commitment:** ✅ All interfaces are internal

**Risk Areas:** None. Both input (Project 1 schema) and output (new Rust API) are under your control.

### Context Coherence 📦

**Domain Concepts:** 6
1. **Position** - File + line + column location in source
2. **Symbol** - Unique identifier for a code entity
3. **Definition** - The location where a symbol is declared
4. **Reference** - A location where a symbol is used
5. **SymbolInfo** - Metadata about a symbol (kind, docs, signature)
6. **Location** - File path + range (reusable struct)

**Core Modules:**
- `scip/query.rs` - Main query interface
- `scip/position.rs` - Position resolution logic
- `scip/types.rs` - Result structs (Location, SymbolInfo, etc.)

**Context Size:** ~300 words, very focused scope

### Testable Completion 🎯

**Success Criteria:**
- [ ] `resolve_symbol_at_position` works for functions, classes, variables
- [ ] `find_definition` returns correct file:line for 20 test cases
- [ ] `find_references` returns all usages (verified against scip-typescript)
- [ ] Query latency < 50ms for any operation
- [ ] Handles "not found" gracefully with proper error types

**Verification Method:**
- Unit tests with fixture database (from Project 1)
- Known positions → expected symbols
- Known symbols → expected reference counts
- Benchmark tests for latency

## Scope Definition

### In Scope
- `resolve_symbol_at_position(file, line, col) → Option<Symbol>`
- `find_definition(symbol) → Option<Location>`
- `find_references(symbol) → Vec<Location>`
- `get_symbol_info(symbol) → Option<SymbolInfo>`
- CLI commands for testing: `maproom scip-query <db> <operation> <args>`
- Proper error handling and Result types
- Query optimization (use indexes effectively)

### Out of Scope
- Call hierarchy (incoming/outgoing calls) - could be Project 2b
- Type hierarchy (supertypes/subtypes)
- MCP integration (Project 3)
- Real-time updates / index refresh
- Fuzzy matching / search
- Cross-repository navigation

### Edge Cases
- Position falls between symbols: Return None, not error
- Symbol has no definition (external): Return None with reason
- Symbol has multiple definitions (overloads): Return all
- File not in index: Return clear error
- Empty database: Return empty results, not crash

## Technical Design

### Core Types

```rust
/// A position in source code
#[derive(Debug, Clone)]
pub struct Position {
    pub file: String,      // Relative path
    pub line: u32,         // 0-indexed
    pub column: u32,       // 0-indexed, UTF-8 byte offset
}

/// A location span in source code
#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Information about a symbol
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub symbol: String,        // Full SCIP symbol string
    pub kind: SymbolKind,      // function, class, method, etc.
    pub display_name: String,  // Human-readable name
    pub documentation: Option<String>,
    pub signature: Option<String>,
    pub definition: Option<Location>,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Function,
    Class,
    Method,
    Variable,
    Constant,
    Interface,
    Module,
    Unknown(String),
}

/// A reference to a symbol with context
#[derive(Debug, Clone)]
pub struct Reference {
    pub location: Location,
    pub role: ReferenceRole,
    pub preview: Option<String>,  // Line of code for context
}

#[derive(Debug, Clone)]
pub enum ReferenceRole {
    Definition,
    Reference,
    Implementation,
    TypeDefinition,
}

/// Query errors
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("File not found in index: {0}")]
    FileNotFound(String),
    
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
    
    #[error("Invalid position: {0}")]
    InvalidPosition(String),
}
```

### Query Interface

```rust
pub struct ScipQueryEngine {
    conn: rusqlite::Connection,
}

impl ScipQueryEngine {
    /// Open a SCIP database
    pub fn open(path: &Path) -> Result<Self, QueryError>;
    
    /// Find what symbol is at the given position
    pub fn resolve_symbol_at_position(
        &self, 
        pos: &Position
    ) -> Result<Option<String>, QueryError>;
    
    /// Find where a symbol is defined
    pub fn find_definition(
        &self, 
        symbol: &str
    ) -> Result<Option<Location>, QueryError>;
    
    /// Find all references to a symbol
    pub fn find_references(
        &self, 
        symbol: &str,
        include_definition: bool,
    ) -> Result<Vec<Reference>, QueryError>;
    
    /// Get detailed information about a symbol
    pub fn get_symbol_info(
        &self, 
        symbol: &str
    ) -> Result<Option<SymbolInfo>, QueryError>;
    
    /// Convenience: definition at position
    pub fn goto_definition(
        &self,
        pos: &Position,
    ) -> Result<Option<Location>, QueryError> {
        if let Some(symbol) = self.resolve_symbol_at_position(pos)? {
            self.find_definition(&symbol)
        } else {
            Ok(None)
        }
    }
}
```

### SQL Queries

**Resolve symbol at position:**
```sql
SELECT s.symbol
FROM scip_occurrences o
JOIN scip_documents d ON o.document_id = d.id
JOIN scip_symbols s ON o.symbol_id = s.id
WHERE d.relative_path = ?
  AND o.start_line <= ?
  AND o.end_line >= ?
  AND (o.start_line < ? OR o.start_col <= ?)
  AND (o.end_line > ? OR o.end_col >= ?)
ORDER BY (o.end_line - o.start_line), (o.end_col - o.start_col)
LIMIT 1;
```

**Find definition:**
```sql
SELECT d.relative_path, o.start_line, o.start_col, o.end_line, o.end_col
FROM scip_occurrences o
JOIN scip_documents d ON o.document_id = d.id
JOIN scip_symbols s ON o.symbol_id = s.id
WHERE s.symbol = ?
  AND o.role = 1  -- Definition role
LIMIT 1;
```

**Find references:**
```sql
SELECT d.relative_path, o.start_line, o.start_col, o.end_line, o.end_col, o.role
FROM scip_occurrences o
JOIN scip_documents d ON o.document_id = d.id
JOIN scip_symbols s ON o.symbol_id = s.id
WHERE s.symbol = ?
ORDER BY d.relative_path, o.start_line;
```

**Get symbol info:**
```sql
SELECT symbol, kind, display_name, documentation, signature
FROM scip_symbols
WHERE symbol = ?;
```

### CLI Interface

```bash
# Resolve symbol at position
maproom scip-query code-intel.db symbol-at src/auth.ts:42:15
# Output: npm pkg `@myapp/auth` > authenticate#function

# Find definition
maproom scip-query code-intel.db definition "npm pkg \`@myapp/auth\` > authenticate#function"
# Output: src/auth.ts:42:1-42:58

# Find references
maproom scip-query code-intel.db references "npm pkg \`@myapp/auth\` > authenticate#function"
# Output:
# src/auth.ts:42:1 (definition)
# src/routes/login.ts:15:5 (reference)
# src/routes/logout.ts:8:12 (reference)
# src/middleware/auth.ts:23:8 (reference)

# Get symbol info
maproom scip-query code-intel.db info "npm pkg \`@myapp/auth\` > authenticate#function"
# Output:
# Kind: function
# Name: authenticate
# Signature: (token: string) => Promise<User>
# Documentation: Validates a JWT token and returns the associated user.
```

## Implementation Plan

### Ticket 1: Core Types
- Create `crates/maproom/src/scip/types.rs`
- Define Position, Location, SymbolInfo, Reference, QueryError
- Add serde derives for JSON output
- Unit tests for type construction

### Ticket 2: Position Resolution
- Create `crates/maproom/src/scip/position.rs`
- Implement `resolve_symbol_at_position`
- Handle edge cases (between symbols, out of bounds)
- Unit tests with various position scenarios

### Ticket 3: Definition Lookup
- Add `find_definition` to query engine
- Handle missing definitions (external symbols)
- Handle multiple definitions (return first or all?)
- Unit tests with fixture database

### Ticket 4: Reference Finding
- Add `find_references` to query engine
- Include/exclude definition option
- Add preview text extraction
- Unit tests verifying complete reference lists

### Ticket 5: Symbol Info & CLI
- Add `get_symbol_info` to query engine
- Implement CLI subcommands
- Add JSON output format option
- Integration tests with real .scip data

## Dependencies

**Requires:** Project 1 (SCIP Schema & Import Foundation)
- SQLite database with populated tables
- Schema must be stable before starting

**Required By:** Project 3 (SCIP MCP Tools)
- This API is what MCP tools will call

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Position resolution ambiguity | Medium | Use smallest enclosing range heuristic |
| Query performance on large DBs | Medium | Ensure indexes exist, benchmark early |
| SCIP symbol strings are complex | Low | Don't parse them, treat as opaque IDs |
| External symbols lack definitions | Low | Return None with clear indication |

## Estimated Effort

- **Duration:** 3-4 days
- **Tickets:** 5
- **Files Created:** 3-4 new files
- **Dependencies:** `rusqlite`, `thiserror`, `serde`

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_resolve_symbol_at_function_name() {
    let db = load_test_fixture();
    let engine = ScipQueryEngine::open(&db).unwrap();
    
    let pos = Position { file: "src/auth.ts", line: 42, column: 10 };
    let symbol = engine.resolve_symbol_at_position(&pos).unwrap();
    
    assert!(symbol.is_some());
    assert!(symbol.unwrap().contains("authenticate"));
}

#[test]
fn test_find_definition_returns_correct_location() {
    let engine = load_test_engine();
    
    let loc = engine.find_definition("test_symbol").unwrap();
    
    assert!(loc.is_some());
    assert_eq!(loc.unwrap().file, "src/test.ts");
    assert_eq!(loc.unwrap().start_line, 10);
}

#[test]
fn test_find_references_includes_all_usages() {
    let engine = load_test_engine();
    
    let refs = engine.find_references("test_symbol", true).unwrap();
    
    assert_eq!(refs.len(), 5);  // 1 definition + 4 references
}
```

### Integration Tests
```rust
#[test]
fn test_end_to_end_goto_definition() {
    // 1. Import real .scip file
    // 2. Query for known position
    // 3. Verify definition matches IDE
}
```

## Success Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Query correctness | 100% match with IDE | Compare 20 cases against VSCode |
| Query latency | < 50ms p99 | Benchmark 1000 random queries |
| Coverage | Functions, classes, variables, methods | Test each symbol kind |
| Error handling | No panics | Fuzz with invalid inputs |