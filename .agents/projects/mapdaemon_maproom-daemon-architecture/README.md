# Project: MAPDAEMON - Maproom Daemon Architecture

## Overview
**MAPDAEMON** is a strategic architectural project to transition the Maproom Rust core from a CLI-only tool to a persistent daemon service. This change is critical for enabling high-performance, low-latency search capabilities by leveraging connection pooling and in-memory caching.

## Problem Statement
Currently, the Maproom MCP server spawns a new Rust process for every search request. This results in:
*   **High Latency:** Process startup and database handshake overhead for every query.
*   **Resource Inefficiency:** No ability to pool database connections or cache embeddings.
*   **Scalability Limits:** The database is hammered with new connections under load.

## Solution
Implement a `serve` command in the `crewchief-maproom` binary that runs a persistent JSON-RPC 2.0 server over Standard IO (stdin/stdout). This allows the MCP server to spawn the process once and reuse it for thousands of requests.

## Key Features
*   **Persistent Process:** Long-running lifecycle managed by the parent process.
*   **JSON-RPC 2.0:** Standard, robust communication protocol.
*   **Connection Pooling:** `sqlx` connection pool initialized once and shared.
*   **Stdio Transport:** Secure, simple local IPC without port conflicts.

## Planning Documents
*   [Analysis](planning/analysis.md)
*   [Architecture](planning/architecture.md)
*   [Quality Strategy](planning/quality-strategy.md)
*   [Security Review](planning/security-review.md)
*   [Execution Plan](planning/plan.md)

## Execution
This project is executed by the **Antigravity** agent.
