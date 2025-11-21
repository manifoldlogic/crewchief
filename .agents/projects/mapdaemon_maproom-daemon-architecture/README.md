# Project: MAPDAEMON - Maproom Daemon Architecture

## Overview
This project transitions Maproom from an ephemeral CLI tool to a persistent daemon architecture. This enables connection pooling and caching, significantly improving performance for high-frequency search operations.

## Problem Statement
Spawning a new process for every search request is inefficient and prevents optimization techniques like connection pooling.

## Solution
Implement a `serve` command that starts a long-running process, communicating via JSON-RPC over standard I/O, to handle search requests efficiently.

## Links
- [Analysis](planning/analysis.md)
- [Architecture](planning/architecture.md)
- [Quality Strategy](planning/quality-strategy.md)
- [Security Review](planning/security-review.md)
- [Plan](planning/plan.md)
