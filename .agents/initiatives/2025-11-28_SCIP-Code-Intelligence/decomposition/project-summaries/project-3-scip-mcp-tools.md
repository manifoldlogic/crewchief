# Project: SCIP MCP Tools

## Project Summary

Expose the SCIP query layer (from Project 2) via MCP tools so AI agents like Claude Code can use precise code intelligence. This is the integration layer that makes the pre-computed SCIP index useful for AI-assisted development.

The tools provide "go to definition", "find references", and "symbol info" capabilities without requiring a running language server—queries execute against a pre-indexed SQLite database in milliseconds.

## Core Criteria Assessment

### Interface Stability 🔒

**External Interfaces:**
- **MCP Protocol**: Stable, you already use it in Maproom
- **Query Layer** (Project 2): Under your control, stable before starting
- **Tool Schemas**: Defined upfront in this project

**Stability Commitment:** ✅ All interfaces are stable or under your control

**Risk Areas:** None. MCP protocol is mature, query layer is internal.

### Context Coherence 📦

**Domain Concepts:** 5
1. **MCP Tool** - A callable function exposed to AI agents
2. **Tool Arguments** - Input schema for each tool
3. **Tool Response** - Output format for results
4. **Error Response** - How failures are communicated
5. **Tool Description** - Help text that guides agent usage

**Core Modules:**
- `tools/scip_definition.ts` - Go to definition tool
- `tools/scip_references.ts` - Find references tool
- `tools/scip_info.ts` - Symbol info tool
- `tools/scip_common.ts` - Shared utilities

**Context Size:** ~250 words, thin integration layer

### Testable Completion 🎯

**Success Criteria:**
- [ ] All three tools registered and callable via MCP
- [ ] Tool descriptions enable AI agent to use correctly (test with Claude)
- [ ] Response format includes file, line, preview text
- [ ] Graceful error when SCIP index doesn't exist
- [ ] End-to-end test: query returns correct definition

**Verification Method:**
- MCP protocol tests (tool registration)
- Integration tests calling tools and verifying responses
- Manual test with Claude Code

## Scope Definition

### In Scope
- `scip_goto_definition` MCP tool
- `scip_find_references` MCP tool
- `scip_symbol_info` MCP tool
- Tool descriptions optimized for AI agent comprehension
- Error responses for common failure cases
- Response formatting with file paths, line numbers, previews

### Out of Scope
- Call hierarchy tools (future enhancement)
- Type hierarchy tools (future enhancement)
- Automatic index refresh / staleness detection
- Multi-repo / multi-index support
- Fallback to live LSP when index is stale

### Edge Cases
- SCIP index doesn't exist: Return helpful error suggesting `maproom scan`
- Symbol not found: Return empty results, not error
- Position not in index: Return "no symbol at position" message
- Index is for different worktree: Warn but attempt query

## Technical Design

### Tool Definitions

#### scip_goto_definition

```typescript
{
  name: "scip_goto_definition",
  description: `Find where a symbol is defined using pre-indexed code intelligence.

USE THIS WHEN:
- You need to find where a function, class, or variable is declared
- You have a file path and line/column position
- You want precise, compiler-accurate results (not grep)

ADVANTAGES OVER GREP:
- Handles renamed imports correctly
- Follows re-exports
- Works across files and packages
- No false positives from comments or strings

EXAMPLE:
Input: { file: "src/routes/login.ts", line: 15, column: 20 }
Output: Definition found at src/auth/authenticate.ts:42:1

NOTE: Requires SCIP index. Run 'maproom scan --scip' if missing.`,
  
  inputSchema: {
    type: "object",
    properties: {
      file: {
        type: "string",
        description: "Relative file path from repository root"
      },
      line: {
        type: "number",
        description: "Line number (1-indexed)"
      },
      column: {
        type: "number",
        description: "Column number (1-indexed, character position)"
      }
    },
    required: ["file", "line", "column"]
  }
}
```

#### scip_find_references

```typescript
{
  name: "scip_find_references",
  description: `Find all usages of a symbol across the codebase using pre-indexed code intelligence.

USE THIS WHEN:
- You need to understand the impact of changing a function/class
- You want to find all callers of a function
- You're assessing how widely something is used
- You need to do a safe rename or refactor

ADVANTAGES OVER GREP:
- Only finds actual usages, not string matches
- Distinguishes definition from references
- Handles aliased imports
- No false positives from comments or similar names

EXAMPLE:
Input: { file: "src/auth.ts", line: 42, column: 10 }
Output: Found 12 references across 5 files

NOTE: Requires SCIP index. Run 'maproom scan --scip' if missing.`,
  
  inputSchema: {
    type: "object",
    properties: {
      file: {
        type: "string",
        description: "File path containing the symbol"
      },
      line: {
        type: "number",
        description: "Line number where symbol appears (1-indexed)"
      },
      column: {
        type: "number",
        description: "Column number (1-indexed)"
      },
      include_definition: {
        type: "boolean",
        description: "Include the definition in results (default: true)",
        default: true
      }
    },
    required: ["file", "line", "column"]
  }
}
```

#### scip_symbol_info

```typescript
{
  name: "scip_symbol_info",
  description: `Get detailed information about a symbol (type signature, documentation, kind).

USE THIS WHEN:
- You need to know a function's type signature
- You want to read a symbol's documentation
- You need to know if something is a class, function, variable, etc.
- You're exploring unfamiliar code

EXAMPLE:
Input: { file: "src/auth.ts", line: 42, column: 10 }
Output: 
  Kind: function
  Signature: (token: string) => Promise<User>
  Documentation: "Validates a JWT token and returns the associated user."

NOTE: Requires SCIP index. Run 'maproom scan --scip' if missing.`,
  
  inputSchema: {
    type: "object",
    properties: {
      file: {
        type: "string",
        description: "File path containing the symbol"
      },
      line: {
        type: "number", 
        description: "Line number (1-indexed)"
      },
      column: {
        type: "number",
        description: "Column number (1-indexed)"
      }
    },
    required: ["file", "line", "column"]
  }
}
```

### Response Formats

**Successful definition lookup:**
```json
{
  "found": true,
  "definition": {
    "file": "src/auth/authenticate.ts",
    "line": 42,
    "column": 1,
    "end_line": 42,
    "end_column": 58,
    "preview": "export async function authenticate(token: string): Promise<User> {"
  },
  "symbol": "npm pkg `@myapp/auth` > authenticate#function"
}
```

**Successful references lookup:**
```json
{
  "symbol": "authenticate",
  "total_count": 12,
  "references": [
    {
      "file": "src/auth/authenticate.ts",
      "line": 42,
      "column": 1,
      "role": "definition",
      "preview": "export async function authenticate(token: string): Promise<User> {"
    },
    {
      "file": "src/routes/login.ts",
      "line": 15,
      "column": 20,
      "role": "reference",
      "preview": "  const user = await authenticate(req.headers.authorization);"
    }
  ]
}
```

**No symbol found:**
```json
{
  "found": false,
  "message": "No symbol found at src/routes/login.ts:15:20. The position may be whitespace, a keyword, or not indexed."
}
```

**Index missing:**
```json
{
  "error": "scip_index_missing",
  "message": "SCIP code intelligence index not found for this repository. Run 'maproom scan --scip' to generate it.",
  "suggestion": "If you've just indexed, try reloading the MCP server."
}
```

### Implementation Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    MCP Request                               │
│  { tool: "scip_goto_definition", args: { file, line, col }} │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                 Tool Handler (TypeScript)                    │
│  1. Validate arguments                                       │
│  2. Convert 1-indexed to 0-indexed positions                │
│  3. Call Rust query layer                                    │
│  4. Format response for AI consumption                       │
│  5. Add preview text from source files                       │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│              Rust Query Layer (Project 2)                    │
│  ScipQueryEngine.goto_definition(pos)                       │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│              SQLite Database (Project 1)                     │
│  SELECT ... FROM scip_occurrences WHERE ...                 │
└─────────────────────────────────────────────────────────────┘
```

### Integration with Maproom MCP

```typescript
// In packages/maproom-mcp/src/tools/index.ts

import { scipGotoDefinition } from './scip_definition';
import { scipFindReferences } from './scip_references';
import { scipSymbolInfo } from './scip_info';

export const tools = [
  // Existing tools
  search,
  open,
  scan,
  context,
  
  // New SCIP tools
  scipGotoDefinition,
  scipFindReferences,
  scipSymbolInfo,
];
```

## Implementation Plan

### Ticket 1: Tool Infrastructure
- Create `packages/maproom-mcp/src/tools/scip_common.ts`
- Add SCIP database detection (does index exist?)
- Add position conversion utilities (1-indexed ↔ 0-indexed)
- Add preview text extraction helper
- Unit tests for utilities

### Ticket 2: Goto Definition Tool
- Create `packages/maproom-mcp/src/tools/scip_definition.ts`
- Implement tool handler
- Wire up to Rust query layer
- Format response with preview
- Integration test

### Ticket 3: Find References Tool
- Create `packages/maproom-mcp/src/tools/scip_references.ts`
- Implement tool handler
- Handle include_definition option
- Paginate if > 50 references
- Integration test

### Ticket 4: Symbol Info Tool
- Create `packages/maproom-mcp/src/tools/scip_info.ts`
- Implement tool handler
- Format documentation nicely
- Handle missing documentation gracefully
- Integration test

### Ticket 5: End-to-End Testing
- Manual test with Claude Code
- Verify tool descriptions are clear
- Test error scenarios
- Update Maproom MCP README with SCIP tools

## Dependencies

**Requires:** 
- Project 1 (SCIP Schema & Import) - SQLite database
- Project 2 (SCIP Query Layer) - Rust query API

**Required By:**
- Project 5 (Scan Integration) - Will generate the index these tools query

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tool descriptions confuse agent | Medium | Test with Claude, iterate on wording |
| Position indexing off-by-one | Medium | Clear docs, unit tests for conversion |
| Large reference lists slow | Low | Paginate, limit to 50 per response |
| Preview extraction fails | Low | Gracefully omit preview, don't error |

## Estimated Effort

- **Duration:** 2-3 days
- **Tickets:** 5
- **Files Created:** 4-5 new files
- **Dependencies:** Existing Maproom MCP infrastructure

## Testing Strategy

### Unit Tests
```typescript
describe('scip_goto_definition', () => {
  it('returns definition location for valid position', async () => {
    const result = await scipGotoDefinition({
      file: 'src/auth.ts',
      line: 15,
      column: 20
    });
    
    expect(result.found).toBe(true);
    expect(result.definition.file).toBe('src/auth/authenticate.ts');
  });
  
  it('returns not found for whitespace position', async () => {
    const result = await scipGotoDefinition({
      file: 'src/auth.ts',
      line: 1,
      column: 1  // Likely whitespace
    });
    
    expect(result.found).toBe(false);
    expect(result.message).toContain('No symbol found');
  });
  
  it('returns helpful error when index missing', async () => {
    // Mock: no SCIP database
    const result = await scipGotoDefinition({
      file: 'src/auth.ts',
      line: 15,
      column: 20
    });
    
    expect(result.error).toBe('scip_index_missing');
    expect(result.suggestion).toContain('maproom scan');
  });
});
```

### Integration Tests
```typescript
describe('SCIP MCP Tools Integration', () => {
  beforeAll(async () => {
    // Import real SCIP data from fixture
    await importScipFixture();
  });
  
  it('end-to-end: goto definition via MCP', async () => {
    const response = await mcpClient.callTool('scip_goto_definition', {
      file: 'src/routes/login.ts',
      line: 15,
      column: 20
    });
    
    expect(response.definition.file).toBe('src/auth/authenticate.ts');
  });
});
```

### Manual Testing Checklist
- [ ] Claude Code can call `scip_goto_definition`
- [ ] Claude Code understands when to use SCIP tools vs grep
- [ ] Error messages guide user to run `maproom scan`
- [ ] Results include useful preview text
- [ ] Position 1-indexing works correctly

## Success Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Tool usability | Claude uses correctly 90%+ | Manual testing sessions |
| Response latency | < 100ms p99 | Benchmark tool calls |
| Error clarity | User knows how to fix | Review error messages |
| AI comprehension | Agent chooses right tool | Test scenarios where SCIP vs grep matters |