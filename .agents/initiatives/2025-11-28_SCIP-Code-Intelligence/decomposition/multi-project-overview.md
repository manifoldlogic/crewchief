# SCIP Code Intelligence for Maproom - Project Overview

## Vision

Give AI agents (Claude Code, etc.) precise, IDE-quality code navigation without running heavy language servers. By consuming pre-computed SCIP indexes, agents can answer "where is this defined?" and "what calls this?" in milliseconds.

## The Opportunity

SCIP (Sourcegraph Code Intelligence Protocol) indexers exist for 8+ languages under Apache 2.0 licenses. They generate precise code intelligence data (definitions, references, type info). However, **no tool exists to consume these indexes and expose them to AI agents via MCP**.

This project fills that gap.

## Project Breakdown

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Project Dependency Graph                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│    Project 1: Schema & Import ──────────────────────┐               │
│    (3-5 days)                                        │               │
│         │                                            │               │
│         ▼                                            │               │
│    Project 2: Query Layer                            │               │
│    (3-4 days)                                        │               │
│         │                                            │               │
│         ├────────────────┐                           │               │
│         ▼                ▼                           │               │
│    Project 3:       Project 4:                       │               │
│    MCP Tools        Multi-Language                   │               │
│    (2-3 days)       (2-3 days)                       │               │
│         │                │                           │               │
│         └────────┬───────┘                           │               │
│                  ▼                                   │               │
│         Project 5: Scan Integration                  │               │
│         (3-4 days)                                   │               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Projects Summary

| # | Project | Duration | Value Unlocked |
|---|---------|----------|----------------|
| 1 | Schema & Import | 3-5 days | Foundation - data in SQLite |
| 2 | Query Layer | 3-4 days | CLI-usable code intelligence |
| 3 | MCP Tools | 2-3 days | **AI agents can use it** |
| 4 | Multi-Language | 2-3 days | Rust + Python support |
| 5 | Scan Integration | 3-4 days | Zero-config experience |

**Total: ~2-3 weeks** to complete all projects

**Usable by AI agents: ~1.5 weeks** (after Project 3)

## Validation Checkpoints

### After Project 1 (Day 3-5)
**Question:** Is the SCIP data useful and complete?

```sql
-- Inspect what we have
SELECT kind, COUNT(*) FROM scip_symbols GROUP BY kind;
SELECT COUNT(*) FROM scip_occurrences WHERE role = 1; -- Definitions
SELECT COUNT(*) FROM scip_occurrences WHERE role = 2; -- References
```

**Go/No-Go:** If data looks sparse or wrong, investigate before continuing.

### After Project 3 (Day 8-12)
**Question:** Do AI agents actually use this correctly?

Test with Claude Code:
- "Find where authenticate is defined"
- "What functions call validateToken?"
- "Show me the type signature of User"

**Go/No-Go:** If agents don't use tools correctly, revise tool descriptions.

### After Project 5 (Day 14-19)
**Question:** Is the zero-config experience smooth?

Test on fresh repository:
```bash
git clone <some-repo>
cd <some-repo>
npm install  # or cargo build
maproom scan
# Should auto-detect and run indexers
```

**Go/No-Go:** If setup friction is high, improve detection/messaging.

## Alternative: Use Existing MCP-LSP Bridge

Before committing to building this, consider:

**isaacphi/mcp-language-server** (BSD license, 1.2k stars)
- Wraps live LSP servers
- Already works with Claude Code
- Supports TypeScript, Rust, Python, Go

**Tradeoffs:**

| Aspect | Live LSP (existing) | SCIP (this project) |
|--------|---------------------|---------------------|
| Setup | Install + configure LSP | Run `maproom scan` |
| Memory | 500MB-2GB per language | ~50-200MB SQLite |
| Startup | 5-60 seconds | Instant |
| Accuracy | Real-time, current | Snapshot at index time |
| Effort | 0 (use existing) | 2-3 weeks |

**Recommendation:** If you have RAM to spare and can tolerate startup time, try the existing LSP bridge first. Build SCIP if:
- Startup latency is painful
- Memory is constrained
- You want index sharing across team
- You want offline capability

## Files Produced

```
/mnt/user-data/outputs/
├── project-1-scip-schema-import.md    # Foundation project
├── project-2-scip-query-layer.md      # Query API project
├── project-3-scip-mcp-tools.md        # MCP integration project
├── project-4-multi-language-scip.md   # Language support project
├── project-5-scan-integration.md      # User experience project
└── scip-projects-overview.md          # This file
```

## Quick Reference: SCIP Indexers

| Language | Indexer | Install | Run |
|----------|---------|---------|-----|
| TypeScript/JS | scip-typescript | `npm i -g @sourcegraph/scip-typescript` | `scip-typescript index` |
| Rust | rust-analyzer | (usually installed) | `rust-analyzer scip .` |
| Python | scip-python | `npm i -g @sourcegraph/scip-python` | `scip-python index . --project-name=X` |
| Java/Kotlin | scip-java | (gradle plugin) | `./gradlew scip` |
| Go | scip-go | `go install github.com/sourcegraph/scip-go` | `scip-go` |

All indexers produce `index.scip` (protobuf format).

## Next Steps

1. **Decide:** Build this or try existing LSP bridge?
2. **If building:** Start with Project 1 to validate the approach
3. **After Project 1:** Inspect data, decide if it's worth continuing
4. **After Project 3:** Test with Claude Code, gather feedback
5. **After Project 5:** Document and ship

## Questions?

Key uncertainties to resolve:
- What's your actual pain point with code navigation today?
- How much RAM does your typical dev environment have?
- Is startup latency (5-60s for LSP) acceptable?
- Do you need offline/CI support?

Answers to these help determine if SCIP is the right investment.