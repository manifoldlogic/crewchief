# Analysis: Maproom Daemon Architecture

## Problem Definition
The current architecture spawns a new Rust process for every search request. This incurs significant overhead:
- **Process Startup**: OS process creation cost.
- **DB Connection**: Establishing a new connection to PostgreSQL for every query.
- **Cold Cache**: No opportunity to cache compiled regexes or prepared statements in memory.

## Context
- **Current State**: Ephemeral CLI invocations.
- **Desired State**: A persistent daemon (`maproom serve`) that maintains state and connections.

## Research Findings
- **Performance**: Connection pooling alone can improve throughput by 10x-100x for high-frequency queries.
- **Protocol**: JSON-RPC over Stdio is a standard pattern for LSP and similar tools, fitting well here.

## Strategic Value
This is the foundation for high-performance, real-time search required for responsive agent interactions.
