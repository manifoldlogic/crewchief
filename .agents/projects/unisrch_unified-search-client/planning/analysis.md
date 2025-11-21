# Analysis: Unified Search Client

## Problem Definition
The Maproom MCP server (`packages/maproom-mcp`) currently contains legacy or placeholder code for search functionality. This creates a maintenance burden and disconnects the MCP server from the high-performance Rust core. The "split brain" problem means MCP users don't benefit from the vector search capabilities.

## Context
- **Current State**: TypeScript implementation in MCP, potentially duplicating or mocking logic.
- **Desired State**: MCP server acts as a thin client, delegating all search logic to the Rust CLI (`vecsrch` project output).

## Research Findings
- The Rust CLI (from `vecsrch`) will output JSON.
- Node.js `child_process` or `execa` can be used to invoke the CLI.
- This pattern ensures a single source of truth for search logic.

## Strategic Value
Unified logic reduces bugs and ensures that improvements to the Rust core immediately benefit all clients (CLI and MCP).
