# Ticket: MCP_CORE-2003: Improve Maproom MCP Tool Descriptions for AI Agent Guidance

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Enhance the MCP tool descriptions for `mcp__maproom__search` to provide clear guidance on when to use maproom vs. other tools (particularly Grep), optimal query patterns, and anti-patterns to avoid. This will help AI agents make informed tool choices without trial-and-error.

## Background
During testing, an AI agent was asked to "find incomplete work in the crewchief CLI using maproom." The agent attempted multiple semantic searches that returned no results:
- "TODO planned feature CLI" - no results
- "crewchief CLI incomplete work" - no results
- "unimplemented not implemented" - no results

Eventually, using Grep to find the literal "⚠️" markers was instant and precise. The core issue: maproom's MCP tool descriptions don't clearly communicate when Grep or other tools are better suited for the task.

Maproom's semantic search works well for conceptual queries (e.g., "authentication flow", "message handling"), but struggles with:
- Exact string matching (literal "TODO" or special characters like "⚠️")
- File path searches
- Pattern matching with special characters
- Very specific implementation detail searches

The tool descriptions need to guide AI agents toward the right tool for each use case, reducing wasted search attempts and improving overall efficiency.

## Acceptance Criteria
- [x] Tool description clearly states when NOT to use maproom (with specific examples)
- [x] Tool description explicitly recommends Grep for literal/exact text searches
- [x] Query guidance includes optimal length (1-3 words) and complexity recommendations
- [x] Examples section shows both good query patterns ("auth flow") and what to avoid ("authentication_handler_implementation_v2")
- [x] Anti-patterns section clearly lists: exact strings, special characters, file paths, overly specific queries
- [x] Performance trade-offs are mentioned (semantic search has overhead vs. Grep's speed for exact matches)
- [x] Tool comparison guidance helps AI agents choose between maproom, Grep, and Glob

## Technical Requirements
- Modify MCP tool descriptions in `packages/maproom-mcp/src/index.ts`
- Update the `description` field for the `search` tool
- Consider adding `inputSchema` descriptions with more detailed guidance
- Ensure descriptions follow MCP specification format
- Keep descriptions concise but comprehensive (aim for clarity over brevity)
- Use markdown formatting for readability where supported by MCP clients

## Implementation Notes

### Target File
- **Primary**: `packages/maproom-mcp/src/index.ts` - MCP server tool definitions

### Suggested Structure for Enhanced Description

The enhanced tool description should include:

1. **When to Use Maproom** (positive guidance)
   - Conceptual/semantic searches
   - Exploring unfamiliar code
   - Finding patterns by intent rather than exact text
   - Examples: "authentication logic", "error handling", "state management"

2. **When NOT to Use Maproom** (anti-patterns)
   - Exact string matching: "TODO", "FIXME", "⚠️"
   - Special characters or symbols
   - File paths or file names
   - Very long or complex queries
   - Implementation-specific names

3. **Alternative Tools**
   - Use **Grep** for: exact text, patterns, literal strings, special chars
   - Use **Glob** for: file name patterns, file discovery
   - Use **Read** for: browsing specific known files

4. **Query Best Practices**
   - Keep queries 1-3 words for best results
   - Use simple, conceptual terms
   - Avoid overly specific implementation details
   - Think "what does this code do" not "what is it called"

5. **Examples Section**
   - ✅ Good: "auth flow", "message bus", "error handling"
   - ❌ Avoid: "authentication_handler_implementation_v2", "TODO comments", "⚠️ markers"

### Performance Considerations
- Note that maproom has semantic search overhead (embedding + ranking)
- Grep is faster for simple exact matches
- Guide agents to choose the right tool for the performance/capability trade-off

## Dependencies
- No prerequisite tickets required
- This is a documentation/UX improvement to existing functionality

## Risk Assessment
- **Risk**: Overly verbose descriptions may reduce clarity
  - **Mitigation**: Keep descriptions focused and scannable; use clear formatting
- **Risk**: Changes to tool descriptions might not be reflected in all MCP clients
  - **Mitigation**: Test with Claude Desktop and other common MCP clients to verify rendering
- **Risk**: May need iteration based on real-world AI agent usage patterns
  - **Mitigation**: Consider this a first iteration; gather feedback and refine

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` - MCP server tool definitions

## Implementation Log

### Changes Made (2025-10-25)

Enhanced the `search` tool description in `packages/maproom-mcp/src/index.ts` (lines 116-118) with:

1. **Anti-patterns section (⚠️ NOT FOR)**:
   - Exact string matching examples: "TODO", "FIXME", "⚠️", "console.log"
   - Special characters or symbols warning
   - File paths/names guidance (redirect to Glob)
   - Long query warning (>4 words)

2. **Tool comparison guidance (✅ USE GREP WHEN)**:
   - Explicit recommendation for exact text searches
   - Literal patterns, comments, markers use case
   - Special characters (emojis, symbols, punctuation)
   - Regex pattern matching
   - Performance-critical simple searches

3. **Glob recommendation (✅ USE GLOB WHEN)**:
   - File name pattern examples: "*.test.ts", "components/**/*.tsx"
   - Directory-based file discovery
   - File extension or path-based searches

4. **Query best practices section**:
   - Keep it simple: 1-3 words works best
   - Conceptual terms over implementation names
   - "what does this do" vs "what is it called" guidance
   - Good examples: "error handling", "message bus", "state management"
   - Bad examples: "TODO comments", "find all ⚠️ markers", "src/components/Button.tsx"

### Testing Notes

This change is a documentation/UX improvement with no functional code changes. The MCP server behavior remains unchanged - only the tool descriptions visible to AI agents have been enhanced.

To verify:
1. MCP server should start without errors
2. Tool descriptions should render correctly in MCP clients (Claude Desktop, etc.)
3. AI agents should make better tool choices when encountering literal search tasks
