# CLI-Maproom Alignment Project

**Project ID:** CLIMAP
**Status:** Planning
**Created:** 2025-01-10

## Overview

Comprehensive update of the `/workspace/packages/cli` package to align with current maproom and maproom-mcp implementations, including fixing command naming conventions.

## Problem Statement

The CLI package is significantly outdated compared to maproom:
1. Uses deprecated `PG_DATABASE_URL` instead of `MAPROOM_DATABASE_URL`
2. Missing documentation for embedding provider setup
3. New maproom features not exposed (branch-watch, cache management, parallel processing)
4. **Command naming inconsistency**: Uses `maproom:scan` pattern instead of `maproom scan` subcommands like other CLI features

## Proposed Solution

1. **Fix environment variable documentation** - Update all references to use `MAPROOM_DATABASE_URL`
2. **Add missing documentation** - Document embedding providers, new flags, schema evolution
3. **Register new commands** - Expose branch-watch, cache, generate-embeddings
4. **Refactor command naming** - Convert `maproom:scan` → `maproom scan` pattern for consistency
5. **Add environment validation** - Check configuration before running Rust binary

## Key Changes

### Command Structure (Clean Break)
```bash
# Old inconsistent pattern (REMOVED)
crewchief maproom:scan
crewchief maproom:search "query"

# New consistent subcommand pattern
crewchief maproom scan
crewchief maproom search "query"
crewchief maproom upsert file.ts
crewchief maproom branch-watch
crewchief maproom cache clear
```

**Note:** No backward compatibility layer - clean implementation with no legacy cruft

## Relevant Agents

- **typescript-engineer** - CLI code refactoring and command registration
- **technical-writer** - README documentation updates
- **unit-test-runner** - Test execution and verification
- **integration-tester** - End-to-end command testing

## Planning Documents

- [Analysis](planning/analysis.md) - Problem space and research
- [Architecture](planning/architecture.md) - Solution design
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security assessment
- [Plan](planning/plan.md) - Execution roadmap

## Timeline

**Estimated Effort:** 1-1.5 days

**Phases:**
1. Documentation fixes (3-4 hours)
2. Command refactoring (2-3 hours) - no backward compat needed
3. Environment validation (4-6 hours)
4. Testing (2-2.5 hours)
