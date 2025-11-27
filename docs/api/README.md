# API Reference

Documentation for Maproom programmatic interfaces.

## MCP Tools

The primary interface for AI agents and IDE integrations.

- **[MCP Tools Reference](mcp-tools.md)** - Complete tool documentation
  - `status` - Check index health
  - `search` - Semantic code search
  - `open` - Retrieve file content
  - `context` - Get related code
  - `scan` - Index repository
  - `upsert` - Update files
  - `explain` - Symbol documentation

## Quick Start

```json
// 1. Check what's indexed
{"method": "tools/call", "params": {"name": "status"}}

// 2. Search for code
{"method": "tools/call", "params": {
  "name": "search",
  "arguments": {"repo": "myproject", "query": "authentication"}
}}

// 3. Get full context
{"method": "tools/call", "params": {
  "name": "context",
  "arguments": {"chunk_id": "...from search results..."}
}}
```

## Protocol

Maproom MCP server communicates via:
- **Transport**: stdio (JSON-RPC 2.0)
- **Protocol version**: 2024-11-05

See [Architecture Overview](../architecture/overview.md) for system design.
