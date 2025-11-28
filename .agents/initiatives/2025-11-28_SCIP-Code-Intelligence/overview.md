# Initiative: SCIP-Based Code Intelligence for Maproom

Created: 2025-11-28

## Vision Statement

Give AI coding agents precise, IDE-quality code navigation (go-to-definition, find-references, symbol info) without requiring running language servers. By consuming pre-computed SCIP indexes, agents can answer structural code questions in milliseconds, enabling more accurate edits, safer refactors, and deeper codebase understanding.

## Conceptual Frame

**The Problem Space:**

AI coding agents like Claude Code currently navigate codebases through grep, glob, and heuristic search. This works but produces false positives, misses renamed imports, and cannot answer precise structural questions like "what calls this function?" or "where is this symbol defined?"

IDEs solve this with Language Server Protocol (LSP), but LSP servers are heavy (500MB-2GB RAM each), slow to start (5-60 seconds), and require the project to be fully buildable. This friction makes them impractical for many AI agent workflows.

**The Opportunity:**

SCIP (Sourcegraph Code Intelligence Protocol) indexers exist for 8+ languages under Apache 2.0 licenses. They generate precise code intelligence data offline. However, **no tool consumes these indexes and exposes them to AI agents**. Sourcegraph kept this capability proprietary.

This initiative fills that gap by building a SCIP consumption layer for Maproom, enabling zero-config code intelligence that works offline, uses minimal resources, and integrates seamlessly with the existing semantic search capabilities.

**Why This Matters:**

- **Accuracy**: Compiler-accurate navigation vs. grep approximation
- **Speed**: Millisecond queries vs. seconds for LSP startup
- **Resources**: ~100MB SQLite vs. gigabytes of LSP servers
- **Simplicity**: `maproom scan` handles everything vs. manual LSP configuration

## Domain Coherence

**Core Domain Concepts (18):**

1. **SCIP Index** - Pre-computed code intelligence data in protobuf format
2. **Symbol** - Unique identifier for a code entity (function, class, variable)
3. **Occurrence** - A location where a symbol appears in source
4. **Definition** - The occurrence where a symbol is declared
5. **Reference** - An occurrence where a symbol is used (not defined)
6. **Position** - File + line + column in source code
7. **Location** - A span (start position to end position)
8. **SymbolInfo** - Metadata: kind, signature, documentation
9. **SymbolKind** - Type classification (function, class, method, etc.)
10. **Document** - A single source file in the index
11. **Indexer** - Tool that generates SCIP data (scip-typescript, rust-analyzer)
12. **Query Engine** - Component that answers questions from index data
13. **MCP Tool** - Interface exposed to AI agents
14. **Import Pipeline** - Process of loading SCIP into queryable storage
15. **SQLite Schema** - Database structure for indexed data
16. **Moniker** - Cross-repository symbol identifier
17. **Relationship** - Link between symbols (implements, extends)
18. **Project Detection** - Identifying what languages/configs exist in a repo

**Unified System Model:**

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SCIP Code Intelligence System                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   INDEX TIME (once per scan)                                        │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐         │
│   │   Indexers   │───▶│  .scip file  │───▶│   SQLite DB  │         │
│   │ (per lang)   │    │  (protobuf)  │    │  (queryable) │         │
│   └──────────────┘    └──────────────┘    └──────────────┘         │
│                                                                      │
│   QUERY TIME (per agent request)                                    │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐         │
│   │  AI Agent    │───▶│  MCP Tools   │───▶│ Query Engine │         │
│   │ (Claude Code)│    │ (goto_def)   │    │   (SQLite)   │         │
│   └──────────────┘    └──────────────┘    └──────────────┘         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

All 18 concepts fit within this single model. No competing ontologies.

## Directional Clarity

**Desired End State:**

> "When this initiative succeeds, AI agents using Maproom will be able to ask 'where is X defined?' and 'what calls Y?' and receive compiler-accurate answers in under 100ms, for TypeScript, Rust, and Python codebases, with zero manual configuration beyond `maproom scan`."

**Success Signals:**

- [ ] **Signal 1: Accuracy** — `scip_goto_definition` returns the same location as VSCode's "Go to Definition" for 95%+ of test cases
- [ ] **Signal 2: Coverage** — Code intelligence works for TypeScript, Rust, and Python projects
- [ ] **Signal 3: Performance** — All queries complete in <100ms (p99)
- [ ] **Signal 4: Zero-Config** — Running `maproom scan` on a fresh repo auto-detects languages and generates indexes
- [ ] **Signal 5: Agent Adoption** — Claude Code correctly chooses SCIP tools over grep when appropriate (observed in testing)

## Scope Boundaries

**In Scope:**
- SCIP index consumption and storage
- Definition, references, and symbol info queries
- TypeScript, Rust, and Python language support
- Integration with `maproom scan` command
- MCP tools for AI agent access

**Out of Scope:**
- Call hierarchy analysis (future initiative)
- Go/Java language support (future work)
- VSCode extension integration (future work)
- Custom indexer development (use existing tools)
- Cross-repository symbol resolution

## Derived Projects

| # | Project | Dependencies | Deliverable |
|---|---------|--------------|-------------|
| 1 | **Schema & Import Foundation** | None | SQLite schema + SCIP import pipeline |
| 2 | **Query Layer** | Project 1 | Rust API for definition/reference queries |
| 3 | **MCP Tools** | Project 2 | AI-usable tools via MCP protocol |
| 4 | **Multi-Language Support** | Projects 1-2 | Validated Rust + Python support |
| 5 | **Scan Integration** | Projects 1-4 | Zero-config `maproom scan` experience |

**Dependency Graph:**

```
Project 1 ──────────────────────┐
    │                           │
    ▼                           │
Project 2                       │
    │                           │
    ├───────────┐               │
    ▼           ▼               │
Project 3   Project 4 ◀─────────┘
    │           │
    └─────┬─────┘
          ▼
      Project 5
```

**Value Unlocked:**
- After Project 3: AI agents can use code intelligence
- After Project 5: Zero-config experience for end users

## Status

- [x] Research complete
- [x] Analysis complete
- [x] Decomposition complete
- [ ] Projects created

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| **SCIP data insufficient** | Projects 2-5 built on bad foundation | Checkpoint 1 validates data quality before continuing |
| **Tool descriptions confuse agents** | Agents don't use tools correctly | Checkpoint 2 validates usability; iterate on descriptions |
| **Indexer availability varies** | Users can't generate indexes | Graceful degradation; clear messaging about missing indexers |
| **Scope creep to call hierarchy** | Timeline extends | Call hierarchy explicitly out of scope; future initiative |
| **Multi-language complexity** | Schema changes mid-stream | Project 4 validates before integration in Project 5 |

## Validation Checkpoints

### Checkpoint 1: Data Quality (After Project 1)

**Question:** Is the SCIP data useful and complete?

**Test:** Inspect imported SQLite database
```sql
SELECT kind, COUNT(*) FROM scip_symbols GROUP BY kind;
```

**Go/No-Go:** If data is sparse or malformed, stop and investigate before continuing.

### Checkpoint 2: Agent Usability (After Project 3)

**Question:** Do AI agents correctly use the tools?

**Test:** Give Claude Code tasks requiring code navigation. Does it choose SCIP tools appropriately?

**Go/No-Go:** If tool descriptions confuse agents, revise before proceeding.

### Checkpoint 3: User Experience (After Project 5)

**Question:** Is the zero-config experience smooth?

**Test:** Clone fresh repo, run `maproom scan`, verify code intelligence works.

**Go/No-Go:** If setup friction is high, improve detection and messaging.

## Alternative Considered

**Existing MCP-LSP bridges** (e.g., isaacphi/mcp-language-server) wrap live language servers and already work with Claude Code.

| Aspect | Live LSP | SCIP (This Initiative) |
|--------|----------|------------------------|
| Setup | Install + configure each LSP | `maproom scan` |
| Memory | 500MB-2GB per language | ~100MB SQLite |
| Startup | 5-60 seconds | Instant |
| Accuracy | Real-time, current | Snapshot at index time |
| Offline | No | Yes |
| Effort | 0 (use existing) | 2-3 weeks |

**Decision:** Build SCIP approach because:
1. Aligns with Maproom's local-first philosophy
2. Complements existing semantic search (hybrid navigation)
3. Better resource profile for constrained environments
4. Enables future index sharing across team members

## Key Constraints

1. **Interface Stability**: SCIP protobuf format is frozen for this initiative
2. **Language Scope**: TypeScript, Rust, Python only (Go/Java are future work)
3. **Feature Scope**: Definition, references, symbol info only (call hierarchy is future)
4. **Integration Scope**: Maproom MCP only (VSCode extension is future)

## Research & Discovery Materials

This initiative is backed by:

1. **Industry Research** — Analysis of GitHub Copilot, Sourcegraph Cody, Cursor, Continue.dev architectures
2. **SCIP Ecosystem Survey** — Identified Apache 2.0 indexers for 8+ languages
3. **MCP-LSP Landscape** — Catalogued 9 existing MCP servers wrapping LSP
4. **Gap Analysis** — Confirmed no tool consumes SCIP indexes for AI agent queries
5. **Technical Feasibility** — SCIP protobuf schema reviewed; Rust bindings available

## Project Artifacts

| Project | Document |
|---------|----------|
| Project 1 | [project-1-scip-schema-import.md](./decomposition/project-summaries/project-1-scip-schema-import.md) |
| Project 2 | [project-2-scip-query-layer.md](./decomposition/project-summaries/project-2-scip-query-layer.md) |
| Project 3 | [project-3-scip-mcp-tools.md](./decomposition/project-summaries/project-3-scip-mcp-tools.md) |
| Project 4 | [project-4-multi-language-scip.md](./decomposition/project-summaries/project-4-multi-language-scip.md) |
| Project 5 | [project-5-scan-integration.md](./decomposition/project-summaries/project-5-scan-integration.md) |
