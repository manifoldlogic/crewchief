# Project: UNISRCH - Unified Search Client

## Overview
This project unifies the search logic by modifying the Maproom MCP server to delegate all search requests to the Rust CLI. This eliminates the "split brain" architecture and ensures consistency.

## Problem Statement
The MCP server uses a disconnected, legacy implementation for search, missing out on the advanced vector search capabilities of the Rust core.

## Solution
Refactor the MCP server to act as a client for the Rust CLI, invoking it via a subprocess for search operations.

## Links
- [Analysis](planning/analysis.md)
- [Architecture](planning/architecture.md)
- [Quality Strategy](planning/quality-strategy.md)
- [Security Review](planning/security-review.md)
- [Plan](planning/plan.md)
