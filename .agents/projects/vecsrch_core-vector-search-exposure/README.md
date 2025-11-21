# Project: VECSRCH - Core Vector Search Exposure

## Overview
This project aims to expose the latent `VectorExecutor` capabilities within the `crates/maproom` Rust library to the CLI. This will enable semantic search functionality for the CrewChief toolset.

## Problem Statement
The powerful vector search logic exists in Rust but is inaccessible to the CLI and MCP server, creating a "split brain" where features are implemented but unusable.

## Solution
We will implement a new CLI command (e.g., `search`) that directly invokes the `VectorExecutor`, returning results in a structured JSON format.

## Links
- [Analysis](planning/analysis.md)
- [Architecture](planning/architecture.md)
- [Quality Strategy](planning/quality-strategy.md)
- [Security Review](planning/security-review.md)
- [Plan](planning/plan.md)
