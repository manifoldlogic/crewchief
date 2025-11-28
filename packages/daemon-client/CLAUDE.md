# CLAUDE.md - daemon-client

## Package Status

**Internal package only** - This package is not published to npm. It is a shared internal dependency used by other packages in the monorepo (maproom-mcp, vscode-maproom).

## Overview

TypeScript client library for communicating with the `crewchief-maproom` daemon via JSON-RPC 2.0. See [README.md](README.md) for full API documentation.

## Key Points

- Provides 20-50x performance improvement over process spawning
- Auto-restart with exponential backoff and circuit breaker
- Used internally by `@anthropic/maproom-mcp` and `vscode-maproom`
- Changes here affect dependent packages but don't require npm publishing
