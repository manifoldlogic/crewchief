# Ticket: MCP_CORE-1001: Context Tool Implementation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the Context tool for the MCP server that integrates with ContextAssembler to retrieve and assemble code context bundles. This tool enables clients to request context for a specific code chunk with configurable token budgets and expansion strategies.

## Background
The Context tool is a core component of the MCP_CORE project (Phase 1, Week 1, Core Tools, Task 1). It provides the ability to retrieve contextually relevant code sections around a given chunk, which is essential for AI assistants to understand code in context. The tool integrates with the CONTEXT_ASM project's ContextAssembler component to intelligently assemble code sections while respecting token budget constraints.

This is the first of the core MCP tools to be implemented and sets the pattern for parameter validation, error handling, and database integration that other tools will follow.

## Acceptance Criteria
- [ ] Context tool is functional and can successfully retrieve context bundles
- [ ] Parameter validation working correctly for all inputs (chunk_id, budget_tokens, expand)
- [ ] Edge cases handled gracefully (missing chunk, budget exceeded, invalid chunk_id)
- [ ] Tests passing with real data from the database
- [ ] Tool returns properly structured ContextBundle responses
- [ ] Integration with ContextAssembler from CONTEXT_ASM project is complete
- [ ] Error messages are clear and actionable for clients

## Technical Requirements
- Integrate with ContextAssembler from CONTEXT_ASM project
- Implement Zod schema for parameter validation:
  - `chunk_id` (required): string UUID identifying the target chunk
  - `budget_tokens` (optional, default 6000): number controlling context size
  - `expand` (optional): configuration object for expansion strategy
- Return ContextBundle with assembled code sections including:
  - Primary chunk content
  - Related context (imports, dependencies, callers)
  - Token count information
  - File and line number metadata
- Validate chunk_id exists in database before processing
- Handle budget overflow gracefully with appropriate warnings
- Follow error handling patterns defined in MCP_CORE_ARCHITECTURE.md (lines 166-192)
- Implement as a ToolHandler compatible with the MCP server base (lines 14-33)

## Implementation Notes
The Context tool implementation should follow the architecture pattern outlined in `/workspace/crewchief_context/maproom/MCP_CORE/MCP_CORE_ARCHITECTURE.md` (lines 67-78).

**Key architectural considerations:**
1. **Validation Layer** (see lines 150-164): Use Zod schemas for parameter validation before execution
2. **Error Handling** (see lines 166-192): Implement proper error types (ValidationError, DatabaseError) with appropriate error codes
3. **ContextAssembler Integration**: The tool delegates to ContextAssembler for the core assembly logic - do not duplicate this logic
4. **Database Connection**: Use the shared database pool from the MCP server base
5. **Budget Management**: Let ContextAssembler handle budget logic, but validate budget_tokens parameter is positive and reasonable (e.g., < 100000)

**Edge cases to handle:**
- Invalid chunk_id format (not a valid UUID)
- Chunk_id not found in database
- Budget_tokens too low (< 100) or unreasonably high (> 100000)
- Database connection failures
- ContextAssembler errors during assembly

**Testing approach:**
- Unit tests with mocked database and ContextAssembler
- Integration tests with real database using test fixtures
- Edge case tests for all validation scenarios
- Performance tests with various budget sizes

## Dependencies
- **CONTEXT_ASM project** - Provides ContextAssembler class for context assembly logic
- **MCP server base implementation** - Provides ToolHandler interface and database pool
- **Database schema** - Requires chunks table to be populated with test data
- No prerequisite tickets - this is the first Phase 1 task

## Risk Assessment
- **Risk**: ContextAssembler integration may have undocumented API changes or incompatibilities
  - **Mitigation**: Review CONTEXT_ASM documentation early, establish communication channel with CONTEXT_ASM team, write integration tests to catch breaking changes

- **Risk**: Database queries may be slow for large codebases affecting tool responsiveness
  - **Mitigation**: Implement query timeouts, add database indexes if needed, monitor query performance in tests

- **Risk**: Budget calculation edge cases could lead to excessive token usage or truncated context
  - **Mitigation**: Comprehensive test coverage for budget scenarios, implement maximum budget cap, return token count metadata in responses

- **Risk**: Error handling may not cover all failure modes leading to poor client experience
  - **Mitigation**: Follow error handling patterns strictly, test all error paths, provide clear error messages with actionable guidance

## Files/Packages Affected
- **NEW**: `packages/maproom-mcp/src/tools/context.ts` - Context tool handler implementation
- **NEW**: `packages/maproom-mcp/src/tools/context_schema.ts` - Zod schema for parameter validation
- **NEW**: `packages/maproom-mcp/tests/tools/context_test.ts` - Unit and integration tests
- **MODIFY**: `packages/maproom-mcp/src/server.ts` - Register Context tool in tool registry (if not already present)
- **MODIFY**: `packages/maproom-mcp/src/tools/index.ts` - Export Context tool (if not already present)
