# Ticket: CONTEXT_ASM-1004: Content Formatting

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-context-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement content formatting system that takes code chunks from the context assembly pipeline and formats them with metadata, role annotations, reason explanations, and intelligent summaries for large chunks.

## Background
This ticket implements Phase 1, Week 2, Task 2 from the CONTEXT_ASM project plan. After chunks are selected by the graph traversal and priority queue, they need to be formatted into ContextItem structures with appropriate metadata. The formatter is responsible for adding semantic information (roles like 'primary', 'test', 'caller', 'callee', 'config', 'hook', 'route') and generating human-readable explanations for why each chunk is included. For large chunks that exceed token budgets, the formatter must generate useful summaries that preserve signatures and docstrings while truncating implementation details.

The ContentFormatter is a critical component in the context assembly pipeline, transforming raw database chunks into semantically annotated context items that provide clear value to LLM consumers.

## Acceptance Criteria
- [ ] Chunks are formatted with complete ContextItem metadata (relpath, range, role, reason, content, tokens)
- [ ] Clear role labels are assigned based on chunk relationship type ('primary', 'test', 'caller', 'callee', 'config', 'hook', 'route')
- [ ] Reason explanations are generated that clearly explain why each chunk is included in the context
- [ ] Useful summaries are generated for chunks exceeding the token budget
- [ ] Signatures and docstrings are preserved during truncation
- [ ] Token counting is accurate for formatted content
- [ ] All formatter logic has comprehensive unit tests

## Technical Requirements
- Implement `ContentFormatter` struct in Rust with the following methods:
  - `format(chunk: &Chunk, role: &str) -> ContextItem`
  - `get_reason(chunk: &Chunk, role: &str) -> String`
  - `truncate_if_needed(content: &str, max_tokens: usize) -> String`
  - `count_tokens(content: &str) -> usize`
- Implement `ContextItem` struct matching the architecture specification:
  - `relpath: String` - relative file path
  - `range: LineRange` - start/end line numbers
  - `role: String` - semantic role annotation
  - `reason: String` - explanation of why included
  - `content: String` - actual code content (possibly truncated)
  - `tokens: usize` - token count of content
- Implement `Summarizer` for intelligent content truncation:
  - Preserve function/class signatures
  - Preserve docstrings and comments
  - Truncate implementation bodies
  - Add truncation markers (e.g., `// ... truncated ...`)
- Use tree-sitter to parse code structure for intelligent truncation
- Support all indexed languages (TypeScript, JavaScript, Rust, etc.)
- Integrate with existing token counting utilities

## Implementation Notes

### Architecture References
- **Primary Documentation**: `/workspace/crewchief_context/maproom/CONTEXT_ASM/CONTEXT_ASM_ARCHITECTURE.md`
  - Lines 169-193: ContentFormatter design
  - Lines 175-186: ContextItem structure definition

### Role Assignment Strategy
The role field indicates the semantic relationship of the chunk to the primary target:
- `'primary'` - The main chunk being requested
- `'test'` - Test file for the primary chunk
- `'caller'` - Function/method that calls the primary chunk
- `'callee'` - Function/method called by the primary chunk
- `'config'` - Configuration file or settings
- `'hook'` - React hook or lifecycle method (framework-specific)
- `'route'` - Route definition (framework-specific)

### Reason Generation
The reason field should provide clear, actionable explanations:
- For primary: "Main implementation being examined"
- For tests: "Tests this function's behavior"
- For callers: "Calls this function from [location]"
- For callees: "Called by this function to [purpose]"
- For config: "Configures [aspect] of this component"

### Truncation Strategy
When content exceeds token budget:
1. Parse the code using tree-sitter to identify structure
2. Extract and preserve:
   - Function/class signature
   - Parameter types and names
   - Return type
   - Docstring/JSDoc comments
3. Truncate:
   - Function body (replace with `// ... implementation truncated ...`)
   - Long literal values
4. Ensure truncated code is syntactically valid (balanced braces)
5. Recalculate token count after truncation

### Token Counting
- Use existing tiktoken-based counting from the codebase
- Count tokens of the final formatted content, not raw chunk content
- Include metadata (role, reason) in the overall bundle budget accounting

### File Organization
```
crates/maproom/src/context/
├── formatter.rs          # ContentFormatter implementation
├── summarizer.rs         # Intelligent truncation logic
└── types.rs             # ContextItem and LineRange structs

crates/maproom/tests/context/
└── formatter_test.rs    # Unit tests for formatter and summarizer
```

### Testing Strategy
Unit tests should cover:
- Basic formatting with all role types
- Reason generation for each role type
- Truncation of various code structures (functions, classes, objects)
- Signature and docstring preservation
- Token counting accuracy
- Edge cases: empty content, very small budgets, malformed code

## Dependencies
- **CONTEXT_ASM-1003** (truncation logic) - Must be completed first as it provides foundational truncation utilities that the formatter builds upon

## Risk Assessment
- **Risk**: Tree-sitter parsing failures on malformed or partial code
  - **Mitigation**: Implement fallback to simple line-based truncation when parsing fails; add error handling and logging

- **Risk**: Token counting inconsistencies between truncation logic and actual LLM usage
  - **Mitigation**: Use the same tiktoken encoding throughout; validate token counts with test cases against known examples

- **Risk**: Over-truncation removing critical information
  - **Mitigation**: Start with conservative truncation (preserve more); add configuration for truncation aggressiveness; include metrics on truncation frequency

- **Risk**: Reason generation being too generic or unhelpful
  - **Mitigation**: Use specific information from graph edges (e.g., actual call sites, test names); iterate on reason templates based on real usage

## Files/Packages Affected
- **New Files**:
  - `crates/maproom/src/context/formatter.rs`
  - `crates/maproom/src/context/summarizer.rs`
  - `crates/maproom/src/context/types.rs`
  - `crates/maproom/tests/context/formatter_test.rs`
- **Modified Files**:
  - `crates/maproom/src/context/mod.rs` - Add module exports
  - `crates/maproom/Cargo.toml` - May need additional dependencies (tree-sitter parsers)
