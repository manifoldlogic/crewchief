# Maproom Strategic Evaluation Report

**Date:** 2025-11-21
**Author:** CrewChief Agent (via Maproom Analysis)

## Executive Summary

A comprehensive analysis of the Maproom codebase using semantic search tools has revealed a highly effective core engine with significant architectural gaps in its client-side integration. The system suffers from a "split brain" where powerful features implemented in Rust (Vector Search) are inaccessible to the CLI and MCP layers due to incomplete integration.

This report outlines four strategic projects to bridge these gaps, adhering to the Project Boundary Framework for stable, agent-driven development.

## Analysis Findings

### Strengths
- **High-Performance Core:** The Rust indexer (`crates/maproom`) effectively uses `tokio`, `pgvector`, and `tree-sitter` for async I/O and parsing.
- **Feature Completeness:** Advanced features like `VectorExecutor` are already implemented in the Rust library, merely waiting for exposure.
- **Effective Streaming:** The `watch` command correctly implements NDJSON streaming for real-time updates.

### Critical Gaps
1. **Disconnected Vector Search:** The Rust CLI does not expose the existing `VectorExecutor`. The MCP server contains dead placeholder code for vector search instead of delegating to Rust.
2. **Process Overhead:** The MCP server spawns a new process for *every search request*, preventing connection pooling and caching.
3. **Security Hygiene:** Known vulnerabilities in dependencies (Prometheus) are explicitly accepted but should be remediated.

## Strategic Projects

We have defined 4 projects to address these findings, structured according to the Stable Context Triangle (Interface Stability, Context Coherence, Testable Completion).

### 1. VECSRCH: Core Vector Search Exposure
**Goal:** Enable the latent vector search capabilities in the Rust CLI.
- **Scope:** `crates/maproom/src/main.rs`, `crates/maproom/src/cli/`.
- **Boundary:** Exposing existing internal Rust types to the CLI surface.
- **Value:** Unlocks semantic search for all clients immediately.

### 2. UNISRCH: Unified Search Client
**Goal:** Eliminate legacy TypeScript search logic in favor of Rust delegation.
- **Scope:** `packages/maproom-mcp/src/index.ts`, `packages/maproom-mcp/src/tools/search.ts`.
- **Boundary:** Clean cut between MCP (protocol layer) and Rust (logic layer).
- **Value:** Ensures single source of truth for search logic; removes "split brain".

### 3. SECHARD: Security Hardening
**Goal:** Remediation of known vulnerabilities and dependency hygiene.
- **Scope:** `crates/maproom/Cargo.toml`, `packages/*/package.json`.
- **Boundary:** Dependency manifest files.
- **Value:** Reduces attack surface and ensures long-term maintainability.

### 4. MAPDAEMON: Maproom Daemon Architecture
**Goal:** Transition from ephemeral process spawning to a persistent server model.
- **Scope:** `crates/maproom/src/daemon/`, `crates/maproom/src/main.rs`.
- **Boundary:** New `serve` command implementation; no client-side changes in this phase.
- **Value:** 10-100x performance improvement for high-frequency search (connection pooling, caching).

## Recommendations for Future Direction

Beyond these immediate fix/refactor projects, the tool should evolve toward **Graph Retrieval (RAG)**.
- **Concept:** Use Tree-sitter to map symbol relationships (calls, inheritance).
- **Implementation:** Leverage Rust's graph traversal capabilities to return "connected context" rather than just text matches.
- **Benefit:** Provides AI agents with the *dependency graph* of a function, not just its definition, drastically improving code modification safety.

