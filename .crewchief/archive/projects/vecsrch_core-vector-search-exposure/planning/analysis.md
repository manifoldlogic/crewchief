# Analysis: Core Vector Search Exposure

## Problem Definition
The Maproom codebase currently suffers from a "split brain" architecture. The core Rust library (`crates/maproom`) implements a high-performance `VectorExecutor` capable of semantic search using `pgvector` and `tree-sitter`. However, this capability is not exposed to the CLI or the MCP server. Consequently, consumers are forced to use inferior or disconnected search mechanisms, leaving the powerful Rust implementation dormant.

## Context
- **Current State**: `VectorExecutor` exists in `crates/maproom` but is internal or unused by the main CLI entry point.
- **Desired State**: A CLI command (e.g., `maproom search --semantic` or similar) directly invokes `VectorExecutor`, returning semantically relevant code snippets.

## Research Findings
- The `VectorExecutor` is already implemented and likely tested at the unit level within the crate.
- The CLI structure (likely `clap` based) needs to be extended to accept vector search parameters.
- This is a low-risk, high-value integration task.

## Strategic Value
Unlocking this feature is the prerequisite for the "Unified Search Client" project. It immediately provides semantic search capabilities to any tool that can run the CLI.
