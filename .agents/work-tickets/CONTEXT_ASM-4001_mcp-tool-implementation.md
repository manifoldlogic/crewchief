# Ticket: CONTEXT_ASM-4001: MCP Tool Implementation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Integrate the context assembler with the MCP context tool to enable AI assistants to retrieve contextual code with proper budget management and expansion options. This ticket implements the handler, parameter validation, response formatting, and error handling for the MCP context tool.

## Background
Phase 4 (Week 6, Task 1) of the Context Assembly System focuses on MCP integration. The context assembler (completed in CONTEXT_ASM-1001 through CONTEXT_ASM-3003) provides powerful context gathering capabilities, but needs to be exposed via the MCP (Model Context Protocol) interface for AI assistants like Claude to use. The MCP context tool skeleton was created in MCP_CORE-1001, and this ticket completes the implementation by wiring up the assembler to that tool interface.

This integration enables AI assistants to:
- Request context for specific code chunks by ID
- Control context size via token budgets (default 6000, range 1000-20000)
- Configure expansion options (include siblings, parents, children, imports)
- Receive properly formatted ContextBundle responses
- Get meaningful error messages when requests fail

## Acceptance Criteria
- [ ] MCP context tool handler is functional and calls the context assembler
- [ ] Parameter validation working for chunk_id, budget_tokens, and expand options
- [ ] Response formatting returns ContextBundle as properly structured JSON
- [ ] Error handling comprehensive for missing chunks, budget exceeded, and database errors
- [ ] Integration tests pass demonstrating end-to-end functionality
- [ ] Documentation updated with tool usage examples and parameter descriptions

## Technical Requirements
- Integrate Rust context assembler with Node.js MCP server (handle FFI bindings if needed)
- Implement tool handler in `packages/maproom-mcp/src/tools/context.ts`
- Parameter validation:
  - `chunk_id`: Required, must exist in database
  - `budget_tokens`: Optional, default 6000, range 1000-20000
  - `expand`: Optional object with boolean flags (siblings, parents, children, imports)
- Response format: Return ContextBundle as JSON matching schema:
  ```typescript
  {
    primary_chunk: CodeChunk,
    additional_chunks: CodeChunk[],
    metadata: {
      total_tokens: number,
      budget_used: number,
      truncated: boolean
    }
  }
  ```
- Error handling:
  - Missing chunk_id → "Chunk not found: {id}"
  - Invalid budget → "Budget must be between 1000 and 20000"
  - Budget exceeded during assembly → Return partial results with truncated flag
  - Database errors → "Failed to retrieve context: {error}"

## Implementation Notes

### Approach
1. **FFI Integration** (if needed):
   - Assess if context assembler is already exposed via FFI or if new bindings needed
   - Use neon-bindings or similar for Rust↔Node.js communication
   - Ensure proper error propagation across FFI boundary

2. **Tool Handler Implementation**:
   - Update `packages/maproom-mcp/src/tools/context.ts` to call assembler
   - Validate all parameters before calling assembler
   - Transform assembler output to MCP tool response format
   - Add comprehensive error handling with user-friendly messages

3. **Testing Strategy**:
   - Unit tests for parameter validation logic
   - Integration tests that:
     - Index sample code
     - Request context for known chunk IDs
     - Verify ContextBundle structure and content
     - Test budget enforcement
     - Test expansion options
     - Test error conditions

### Considerations
- **Performance**: Context assembly can be expensive; ensure reasonable timeouts
- **Token Counting**: Verify token counting matches expected model tokenizer (cl100k_base for GPT-4/Claude)
- **Caching**: Consider caching frequently requested chunks (future optimization)
- **Logging**: Add detailed logging for debugging context assembly decisions
- **Backwards Compatibility**: Ensure changes don't break existing MCP tools

## Dependencies
- **CONTEXT_ASM-1001**: Basic assembly pipeline (MUST be complete)
- **CONTEXT_ASM-1002**: Relationship queries (MUST be complete)
- **CONTEXT_ASM-1003**: Budget management (MUST be complete)
- **CONTEXT_ASM-1004**: Content formatting (MUST be complete)
- **CONTEXT_ASM-2xxx and 3xxx**: All Phase 2 and 3 tickets (MUST be complete)
- **MCP_CORE-1001**: Context tool skeleton in MCP server (MUST exist)

## Risk Assessment
- **Risk**: FFI complexity if Rust assembler not already exposed
  - **Mitigation**: Review existing FFI patterns in codebase (maproom already has FFI), reuse proven approach

- **Risk**: Token counting mismatch between assembler and actual model usage
  - **Mitigation**: Use standardized tokenizer (tiktoken), add tests comparing token counts, document any known discrepancies

- **Risk**: Performance degradation with large codebases or complex expansion
  - **Mitigation**: Implement timeouts, add performance logging, consider optimization work in future tickets

- **Risk**: Breaking changes to MCP tool interface
  - **Mitigation**: Follow MCP protocol standards, version the tool schema, add integration tests

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/context.ts` - Main implementation
- `packages/maproom-mcp/src/types/context.ts` - Type definitions for ContextBundle (if needed)
- `packages/maproom-mcp/tests/tools/context_integration_test.ts` - Integration tests
- `packages/maproom-mcp/tests/tools/context_unit_test.ts` - Unit tests (if needed)
- `crates/maproom/src/context/ffi.rs` - FFI bindings (if new bindings needed)
- `packages/maproom-mcp/README.md` - Documentation with tool usage examples
