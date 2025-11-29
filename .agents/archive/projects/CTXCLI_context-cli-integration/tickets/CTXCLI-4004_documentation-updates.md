# Ticket: CTXCLI-4004: Documentation Updates

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation only)
- [x] **Verified** - by the verify-ticket agent

## Agents
- technical-researcher
- verify-ticket
- commit-ticket

## Summary
Update CLAUDE.md files and other documentation for the new context command and MCP tool changes.

## Background
This is the final ticket of Phase 4 (Testing & Polish). Documentation must be updated to reflect the new CLI context command and the MCP tool's switch from PostgreSQL to daemon.

Reference: [planning/plan.md](../planning/plan.md) - CTXCLI-4004

## Acceptance Criteria
- [x] `crates/maproom/CLAUDE.md` updated with context command usage
- [x] `packages/maproom-mcp/CLAUDE.md` updated with context tool changes
- [x] Common errors and troubleshooting documented
- [x] Examples for CLI usage provided
- [x] Examples for MCP tool usage provided
- [x] Documentation is clear and accurate

## Technical Requirements
- Follow existing CLAUDE.md format and style
- Include both CLI and MCP usage examples
- Document error codes and troubleshooting steps
- Cross-reference architecture documentation

## Implementation Notes

### crates/maproom/CLAUDE.md Updates

Add section for context command:

```markdown
## Context Command

Retrieve a context bundle for a specific code chunk:

### Basic Usage

\`\`\`bash
# Get context for chunk #12345
crewchief-maproom context --chunk-id 12345

# Output as JSON
crewchief-maproom context --chunk-id 12345 --json
\`\`\`

### Expand Options

\`\`\`bash
# Include callers and callees
crewchief-maproom context --chunk-id 12345 --callers --callees

# Include tests and documentation
crewchief-maproom context --chunk-id 12345 --tests --docs

# Custom budget and depth
crewchief-maproom context --chunk-id 12345 --budget 4000 --max-depth 3

# All options
crewchief-maproom context --chunk-id 12345 \
  --callers --callees --tests --docs --config \
  --budget 6000 --max-depth 2 --json
\`\`\`

### Daemon Context Method

The daemon also exposes context via JSON-RPC:

\`\`\`json
{
  "jsonrpc": "2.0",
  "method": "context",
  "params": {
    "chunk_id": "12345",
    "budget_tokens": 6000,
    "expand": {
      "callers": true,
      "callees": true,
      "tests": true
    }
  },
  "id": 1
}
\`\`\`

### Error Codes

| Code | Meaning |
|------|---------|
| -32000 | Chunk not found |
| -32001 | File not found on disk |
| -32002 | Budget exceeded |
| -32602 | Invalid parameters |
```

### packages/maproom-mcp/CLAUDE.md Updates

Add/update context tool section:

```markdown
## Context Tool

The context tool retrieves contextually relevant code around a specific chunk.

### Changes from Previous Version

- **Now uses daemon**: Context assembly happens in the Rust daemon, not TypeScript
- **React-specific options**: Added hooks, jsx_parents, jsx_children expand options
- **Improved caching**: Daemon maintains LRU cache across requests

### Usage

\`\`\`typescript
const result = await server.callTool('context', {
  chunk_id: '12345',
  budget_tokens: 6000,
  expand: {
    callers: true,
    callees: true,
    tests: true,
    hooks: true,  // React-specific
  },
})
\`\`\`

### Response Format

\`\`\`typescript
{
  items: ContextItem[],
  total_tokens: number,
  budget_tokens: number,
  budget_remaining: number,
  truncated: boolean,
  metadata: {
    worktree: string,
    repo: string,
  }
}
\`\`\`

### Troubleshooting

#### "Daemon not available" Error

The daemon must be running. Start it with:

\`\`\`bash
crewchief-maproom serve
\`\`\`

#### "Chunk not found" Error

The chunk_id may be invalid. Use the search tool to find valid chunks:

\`\`\`typescript
const searchResult = await server.callTool('search', {
  query: 'function authenticate',
})
```

## Dependencies
- All previous tickets (implementation must be complete)

## Risk Assessment
- **Risk**: Documentation out of sync with implementation
  - **Mitigation**: Review actual code behavior before documenting

## Files/Packages Affected
- `crates/maproom/CLAUDE.md` (modify)
- `packages/maproom-mcp/CLAUDE.md` (modify)
