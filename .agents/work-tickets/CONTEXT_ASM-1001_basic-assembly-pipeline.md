# Ticket: CONTEXT_ASM-1001: Basic Assembly Pipeline

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-context-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the foundational context assembly pipeline that retrieves chunks by ID, loads file content, counts tokens, and returns a basic ContextBundle structure. This is the first component of the Context Assembly Engine (CONTEXT_ASM) that will enable budget-aware, intelligent context gathering for LLMs.

## Background
The Context Assembly Engine needs a core pipeline to retrieve code chunks from the database, load their associated file content, count tokens accurately, and assemble them into a structured ContextBundle format. This foundational work enables all subsequent features like relationship traversal, budget management, and intelligent context selection.

This ticket implements Phase 1, Week 1, Task 1 from the CONTEXT_ASM_PLAN.md, establishing the basic assembly structure before adding complexity like graph traversal and token budget allocation.

## Acceptance Criteria
- [ ] Retrieve and format primary chunk by ID from database
- [ ] Load file content for chunks from filesystem
- [ ] Count tokens accurately using tiktoken or similar library
- [ ] Return basic ContextBundle structure with ContextItems
- [ ] Handle missing chunks gracefully with appropriate errors
- [ ] Handle file read errors with graceful degradation
- [ ] Unit tests verify core functionality
- [ ] Integration test assembles actual chunk from database

## Technical Requirements

### Core Interfaces (from Architecture)
Implement the ContextAssembler interface:
```rust
pub trait ContextAssembler {
    async fn assemble(
        &self,
        chunk_id: i64,
        budget: usize,
        options: ExpandOptions
    ) -> Result<ContextBundle>;
}

pub struct ExpandOptions {
    pub callers: bool,
    pub callees: bool,
    pub tests: bool,
    pub docs: bool,
    pub config: bool,
    pub max_depth: i32,
}
```

### Data Structures
Define core types in `context/types.rs`:
```rust
pub struct ContextBundle {
    pub items: Vec<ContextItem>,
    pub total_tokens: usize,
    pub truncated: bool,
}

pub struct ContextItem {
    pub relpath: String,
    pub range: LineRange,
    pub role: String,
    pub reason: String,
    pub content: String,
    pub tokens: usize,
}

pub struct LineRange {
    pub start: i32,
    pub end: i32,
}
```

### Database Integration
- Query chunks table by ID
- Retrieve chunk metadata (file path, line range, etc.)
- Use existing database connection pool from Maproom

### Token Counting
- Implement token counting utility using tiktoken-rs or similar
- Support multiple encodings (cl100k_base for GPT-4, etc.)
- Accurate counting for TypeScript, JavaScript, Rust, and other languages

### File Content Loading
- Load file content from filesystem using chunk's file path
- Extract specific line ranges from file
- Handle UTF-8 encoding properly
- Stream large files if needed

## Implementation Notes

### Module Structure
Create new module at `crates/maproom/src/context/`:
- `mod.rs` - Module exports
- `types.rs` - Core data structures
- `assembler.rs` - Main ContextAssembler implementation
- `token_counter.rs` - Token counting utility

### Error Handling
Follow Maproom's error handling patterns:
- Use `anyhow::Result` for functions
- Create specific error types if needed
- Graceful degradation for file read errors
- Clear error messages for missing chunks

### Content Formatting (from Architecture)
Implement basic ContentFormatter:
- Format chunks with metadata (relpath, line range)
- Add role annotation (e.g., "primary", "caller", "test")
- Include reason explanation (e.g., "target chunk", "calls this function")
- Calculate and include token count per item

### Initial Simplification
For this first ticket, implement ONLY:
- Single chunk retrieval (no graph traversal yet)
- Basic ContextBundle assembly
- Token counting
- File content loading
- Simple formatting

Future tickets will add:
- Relationship traversal
- Budget management and allocation
- Truncation logic
- Priority ranking

### Testing Strategy
- Unit tests for token counting utility
- Unit tests for ContentFormatter
- Mock database for assembler tests
- Integration test with real database and files
- Test error cases (missing chunk, missing file, invalid UTF-8)

## Dependencies
- Existing chunks table in maproom database schema
- File system access to indexed repository files
- Token counting library (tiktoken-rs or similar)

## Risk Assessment
- **Risk**: Token counting library may not support all languages accurately
  - **Mitigation**: Start with TypeScript/JavaScript focus, add language-specific handling as needed, provide fallback estimation

- **Risk**: Large files may cause memory issues when loading content
  - **Mitigation**: Implement streaming for files >1MB, extract only needed line ranges

- **Risk**: File paths in database may be stale or incorrect
  - **Mitigation**: Validate file existence before reading, return clear errors for missing files

## Files/Packages Affected
- `crates/maproom/src/context/mod.rs` - New module
- `crates/maproom/src/context/types.rs` - Core data structures
- `crates/maproom/src/context/assembler.rs` - Main assembler implementation
- `crates/maproom/src/context/token_counter.rs` - Token counting utility
- `crates/maproom/src/lib.rs` - Export new context module
- `crates/maproom/Cargo.toml` - Add token counting dependency
- `crates/maproom/tests/context/assembler_test.rs` - Unit tests
- `crates/maproom/tests/context/token_counter_test.rs` - Token counter tests

## Planning References
- Architecture: `/workspace/crewchief_context/maproom/CONTEXT_ASM/CONTEXT_ASM_ARCHITECTURE.md`
  - Lines 14-32: ContextAssembler interface
  - Lines 169-193: Content Formatter
  - Lines 89-111: Token Budget Manager (reference for future)
- Plan: `/workspace/crewchief_context/maproom/CONTEXT_ASM/CONTEXT_ASM_PLAN.md`
  - Lines 12-28: Phase 1, Week 1 tasks and acceptance criteria
