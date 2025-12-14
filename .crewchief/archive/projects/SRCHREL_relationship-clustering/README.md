# Project: Relationship-Aware Search

**Slug:** SRCHREL
**Status:** Completed
**Created:** 2025-12-14
**Completed:** 2025-12-14
**Initiative:** Maproom Semantic Search Improvements (Phase 2)

## Summary

Extends maproom's search results with lightweight relationship metadata that exposes architectural context for high-confidence results. When users search for code, high-confidence matches automatically include related chunks discovered via graph traversal of import/call relationships, enabling architectural discovery without manual file exploration.

**Example**: Searching for "authentication handler" returns not just `authHandler.ts` but also its imported dependencies (`validateToken.ts`) and callers (`authMiddleware.ts`), providing immediate architectural context.

## Problem Statement

Search results currently return individual code chunks without architectural context. Users cannot see:
- **Related implementations**: What other code this chunk imports or calls
- **Architectural patterns**: Which chunks belong to the same module
- **Cross-cutting concerns**: How chunks cluster across file boundaries
- **Hidden dependencies**: Related code that didn't rank highly enough to appear in top results

**Impact**: Developers must manually piece together architectural relationships by inspecting each file, losing the time-saving benefit of semantic search.

## Proposed Solution

**Confidence-Gated Graph Traversal**:
1. For high-confidence search results (`source_count >= 2` OR `is_exact_match`), perform shallow graph traversal (2 hops maximum)
2. Return top 5 related chunks with metadata (file path, symbol, line range, preview, relevance, relationship type)
3. Weight relationships by edge type (extends/implements > import/call > test relationships) and module proximity (same directory gets 1.2× boost)
4. Opt-in via `include_related=true` parameter (backward compatible)

**Key Design Decisions**:
- **Metadata-only**: Related chunks include pointers, not full content (lightweight, ~200 bytes/chunk)
- **Performance budget**: <20ms overhead via confidence gating (only 20-40% of results expanded)
- **Existing infrastructure**: Reuses `chunk_edges` table and `find_related_chunks()` function (no new DB tables)
- **Type-aware weighting**: Prioritizes production code relationships over test relationships

## Relevant Agents

### Planning Phase
- project-planner: Complete planning documents ✓
- project-reviewer: Review planning for technical feasibility

### Implementation Phase
- rust-expert: Implement relationship expansion module, type definitions
- search-engineer: Integrate into search pipeline, confidence gating
- database-engineer: Verify index performance for depth-2 traversal
- typescript-expert: Type synchronization, daemon client integration
- mcp-engineer: Update MCP tool schema, parameter handling
- test-engineer: Unit tests, integration tests, E2E tests
- performance-engineer: Benchmarks, latency validation, optimization
- technical-writer: User documentation, architecture guide

### Verification Phase
- verify-ticket: Validate acceptance criteria
- commit-ticket: Create commits for completed work

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis, existing infrastructure, constraints, success criteria
- [architecture.md](planning/architecture.md) - Solution design, component architecture, integration points
- [plan.md](planning/plan.md) - 3-phase execution plan (Rust Core → TypeScript Integration → Testing & Docs)
- [quality-strategy.md](planning/quality-strategy.md) - Testing philosophy, critical paths, performance benchmarks
- [security-review.md](planning/security-review.md) - Security assessment (LOW risk, read-only operations)

## Dependencies

### External (COMPLETE)
- **SRCHCONF** (Confidence Scoring): Provides confidence signals for gating relationship expansion
- **SRCHFLTR** (Result Filtering): Cleaner result sets enable type-aware edge weighting

### Internal Phase Dependencies
- Phase 1 (Rust Core) → Phase 2 (TypeScript Integration) → Phase 3 (Testing & Docs)

## Key Metrics

**Must Achieve**:
- [ ] Related chunks appear for high-confidence results (confidence threshold validated)
- [ ] Performance budget: <20ms p95 overhead
- [ ] Backward compatibility: Searches without parameter unchanged
- [ ] Type synchronization: Rust ↔ TypeScript validated

**Should Achieve**:
- [ ] Module proximity boost: Same-directory chunks rank higher
- [ ] Edge weight accuracy: Code→code > code→test relationships
- [ ] Graceful degradation: Graph errors don't fail search

## Timeline

**Total Estimated Duration**: 3-4 weeks

- **Week 1**: Rust Core Infrastructure (types, relationships module, pipeline integration, unit tests)
- **Week 2**: TypeScript Integration (type sync, daemon client, MCP tool, E2E tests)
- **Week 3**: Testing & Documentation (performance validation, edge cases, user docs)
- **Week 4**: Buffer for optimization and refinement

## Next Steps

1. **Review Planning**: Run `/review-project SRCHREL` to validate planning documents
2. **Create Tickets**: Run `/create-project-tickets SRCHREL` to generate implementation tickets
3. **Begin Implementation**: Start with Phase 1 (Rust Core Infrastructure)

## Architecture Highlights

**Component Structure**:
```
Rust (crates/maproom/src/search/):
├── relationships.rs (NEW) - Relationship expansion logic
├── results.rs (MODIFIED) - Add RelatedChunkResult struct
└── pipeline.rs (MODIFIED) - Integrate expansion after confidence scoring

TypeScript (packages/):
├── daemon-client/src/types.ts (MODIFIED) - Mirror RelatedChunkResult type
└── maproom-mcp/src/tools/search_schema.ts (MODIFIED) - Add include_related parameter
```

**Data Flow**:
```
Search Query
  ↓
Score Fusion
  ↓
Confidence Scoring (SRCHCONF)
  ↓
Relationship Expansion (if include_related=true + high confidence)
  ↓
  - Graph Traversal (depth 2)
  - Relevance Weighting (decay × edge_weight × module_boost)
  - Top 5 Selection
  ↓
Final Results with related chunks
```

**Performance Budget**:
- Single result traversal: ~8ms
- 3 high-confidence results: ~24ms (within <20ms target via parallel traversal)
- Response size: ~6.5KB (10 results + 3×5 related chunks)

## Testing Strategy

**Critical Paths** (MUST test):
1. High-confidence result expansion (confidence gating works correctly)
2. Performance budget compliance (<20ms overhead validated)
3. Backward compatibility (searches without parameter unchanged)
4. Type synchronization (Rust ↔ TypeScript match)
5. Graph traversal correctness (depth-2, relevance scoring)
6. Error handling (graceful degradation on failures)

**Test Types**:
- Unit: Edge weights, module proximity, relevance sorting
- Integration: Search with relationships, confidence gating, backward compatibility
- E2E: Real database, real queries, performance validation

## Security Assessment

**Risk Level**: LOW

- **Read-only operations**: No authentication/authorization changes
- **Parameterized queries**: No SQL injection risk
- **DoS prevention**: Depth/count/confidence limits prevent unbounded queries
- **Graceful degradation**: Errors logged server-side, don't expose internal details
- **No new dependencies**: Reuses existing infrastructure

**Accepted Gaps** (low risk):
- No rate limiting (confidence gating provides built-in limiting)
- Preview may contain sensitive comments (already exposed in search results)

## Value Proposition

**For Developers**:
- Discover architectural relationships without manual file browsing
- Understand code context immediately (imports, callers, same-module code)
- Identify cross-cutting concerns through relationship clustering

**For the Product**:
- Differentiates maproom as architecture-aware search tool
- Leverages existing graph infrastructure (no new tables/indexes)
- Ships with minimal performance impact (<20ms overhead)
- Backward compatible (opt-in feature)

## Agent Recommendation

**Custom agents NOT recommended** for this project.

**Rationale**: This project is well-suited for general-purpose agents (rust-expert, typescript-expert, search-engineer). The work is straightforward integration of existing graph traversal infrastructure into the search pipeline with type synchronization. No deep specialized domain expertise is required beyond existing agent capabilities.

**General agents are sufficient for**:
- Rust struct definitions and module creation
- TypeScript type mirroring and validation
- Search pipeline integration patterns
- Database query optimization (existing indexes sufficient)
- Performance benchmarking (standard tooling)

## References

**Initiative Documentation**:
- [Initiative Overview](../../initiatives/2025-12-09_maproom-semantic-search-improvements/overview.md)
- [Multi-Project Decomposition](../../initiatives/2025-12-09_maproom-semantic-search-improvements/decomposition/multi-project-overview.md)

**Related Projects**:
- [SRCHCONF](../SRCHCONF_confidence-scoring/) - Confidence Scoring (COMPLETE, dependency)
- [SRCHFLTR](../../archive/projects/SRCHFLTR_result-filtering/) - Result Filtering (COMPLETE, archived)

**Codebase References**:
- Graph traversal: `crates/maproom/src/context/graph.rs`
- Chunk edges schema: `crates/maproom/src/db/sqlite/schema.rs`
- Context tool pattern: `packages/maproom-mcp/src/tools/context.ts`
