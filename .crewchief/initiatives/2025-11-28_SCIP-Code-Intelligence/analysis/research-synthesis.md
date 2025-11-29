# Research Synthesis: SCIP-Based Code Intelligence

## Key Findings

### SCIP Ecosystem

1. **Mature indexers available** under Apache 2.0:
   - `scip-typescript` — Official Sourcegraph indexer
   - `rust-analyzer` — Native SCIP output support
   - `scip-python` — Tree-sitter based Python indexer
   - `scip-java`, `scip-ruby`, `scip-dotnet` — Additional languages

2. **Protobuf schema is stable** — SCIP v3 format frozen, backwards compatible

3. **Rust bindings exist** — `scip` crate provides protobuf deserialization

### MCP-LSP Landscape

Surveyed 9 existing MCP servers wrapping live LSP:
- `mcp-language-server` (isaacphi)
- `lsp-mcp-server` (various)
- Language-specific wrappers

**Gap identified:** None consume pre-computed indexes. All require running LSP servers.

### Industry Approaches

| Tool | Code Intelligence Approach |
|------|---------------------------|
| GitHub Copilot | Repository-level RAG, no structural queries |
| Sourcegraph Cody | SCIP indexes (proprietary consumption) |
| Cursor | Real-time LSP integration |
| Continue.dev | Optional LSP, primarily RAG |

**Insight:** Sourcegraph built SCIP but kept consumption layer proprietary.

### Performance Benchmarks (from SCIP docs)

- Index generation: 10-60 seconds for medium projects
- Index size: ~10% of source code size
- Query latency: <10ms for SQLite with proper indexes

## Open Questions

### Technical

1. **How to handle partial indexes?** If indexer fails on some files, should we:
   - Skip entirely?
   - Import what succeeded?
   - Mark files as "unindexed"?

2. **Incremental updates?** Can we update only changed files, or must re-index entire project?

3. **Symbol disambiguation?** Multiple symbols with same name (e.g., overloads) — how to present?

### UX

1. **Tool naming?** Should tools be `scip_*` or integrate into existing Maproom tools?

2. **Error messaging?** When index is stale or missing, what should agent see?

3. **Confidence indicators?** Should results include staleness or accuracy metadata?

## Assumptions

### Validated

- [x] SCIP indexers produce consistent output across platforms
- [x] SQLite can handle typical project index sizes (~100MB)
- [x] Rust protobuf parsing is performant for our scale

### Unvalidated (to test in Project 1)

- [ ] All symbol kinds we need are captured in SCIP output
- [ ] Import/export chains are fully resolved
- [ ] Cross-file references are complete

### Accepted Limitations

- Index reflects scan time, not live edits
- Dynamic dispatch targets not captured
- Macro-generated code may be incomplete
- Dependencies indexed only if requested

## Recommendations

1. **Start with TypeScript** — Largest user base, most mature indexer, good test coverage

2. **Validate early** — Checkpoint 1 must confirm data quality before building query layer

3. **Simple schema first** — Avoid premature optimization; add indexes based on actual query patterns

4. **Clear error states** — Agents should know when index is missing/stale vs. symbol not found

5. **Preserve flexibility** — Schema should accommodate future call hierarchy without migration
