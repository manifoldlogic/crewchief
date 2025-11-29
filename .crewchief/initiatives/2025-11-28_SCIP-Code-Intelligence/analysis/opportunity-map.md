# Opportunity Map: SCIP-Based Code Intelligence

## Problem Spaces

### Current State: Approximate Navigation

AI coding agents navigate codebases through:
- **grep/glob** — Text pattern matching, high false positive rate
- **Heuristic search** — Semantic similarity, imprecise for structural queries
- **AST parsing** — Per-file only, no cross-file resolution

**Failure modes:**
- "Find all callers of `processOrder`" returns files containing the string, not actual call sites
- Renamed imports and re-exports are invisible
- Type relationships (implements, extends) cannot be queried

### Alternative: Live LSP

IDEs solve this with Language Server Protocol, but:
- 500MB-2GB RAM per language server
- 5-60 second startup time
- Requires buildable project (dependencies installed)
- One server process per workspace

**Not viable for AI agents** working across many repositories or in constrained environments.

## Goals

1. **Compiler-accurate navigation** — Same results as "Go to Definition" in VSCode
2. **Sub-100ms query latency** — Instant feedback for interactive agent workflows
3. **Zero-config setup** — Works after `maproom scan`, no manual LSP configuration
4. **Minimal resources** — ~100MB SQLite database, no running processes
5. **Offline capability** — All data local, no network dependency

## Constraints

| Constraint | Rationale |
|------------|-----------|
| TypeScript, Rust, Python only | Focus on languages with mature SCIP indexers |
| Definition + references only | Core navigation; call hierarchy deferred |
| Snapshot accuracy | Index reflects scan time, not live edits |
| Existing indexers only | Leverage Apache 2.0 ecosystem, don't build indexers |
| SQLite storage | Consistent with Maproom architecture |

## Opportunities

### Immediate Value

- **Safer refactoring** — Agents can find all references before modifying code
- **Accurate imports** — Trace symbol origin through re-exports
- **Documentation lookup** — Access symbol docstrings from index

### Future Expansion

- **Call hierarchy** — "What calls this?" at arbitrary depth
- **Type hierarchy** — "What implements this interface?"
- **Cross-repo navigation** — Monikers enable inter-repository symbol lookup
- **Index sharing** — Team members share pre-built indexes

### Competitive Positioning

No existing tool:
- Consumes SCIP indexes for AI agent queries
- Provides offline, resource-efficient code intelligence
- Integrates with local-first semantic search

This creates a unique value proposition for Maproom.
