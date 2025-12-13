# Backlog: Maproom Semantic Search Improvements

Ideas identified during research but not yet ready for project creation.

## Ideas

| Idea | Source | Notes | Status |
|------|--------|-------|--------|
| Natural Language Query Expansion | 2025 research (QEIEF model) | Use developer descriptions to expand queries. Requires ML model training and query corpus. | Deferred - Phase 2+ |
| Multi-Repository Federated Search | User need extrapolation | Search across multiple indexed repositories simultaneously. Requires federation architecture, result merging, cross-repo ranking. | Separate Initiative |
| User Behavior-Based Re-Ranking | Cursor case study | Track click-through, dwell time, result selection. Re-rank based on implicit feedback. Requires observability infrastructure + privacy design. | Deferred - Observability first |
| Search History and Saved Searches | Developer tool UX pattern | Allow users to recall past searches, save common queries. Requires persistence layer, UI design. | Deferred - Post-MVP |
| Personalized Search | Industry trend | Adapt results to user's role, recent work, codebase familiarity. Requires user modeling, extensive instrumentation. | Deferred - Future phase |
| Code Clone Detection via Search | Semantic similarity use case | Find duplicate/similar code patterns using vector search. Requires specialized scoring, UI for comparison. | Separate Feature |
| Search Result Previews | UX enhancement | Show code snippet with syntax highlighting inline. Requires rendering pipeline, format negotiation. | Client-Side Feature |
| Search Shortcuts and Macros | Power user feature | Define reusable search patterns (e.g., "recent auth changes"). Requires DSL or shortcut syntax. | Deferred - Low priority |
| Cross-Language Symbol Search | Advanced use case | Find equivalent concepts across languages (e.g., Python `@property` ↔ Rust getter). Requires language-aware mapping. | Deferred - Complex |
| Time-Travel Search | Historical queries | Search code as it existed at a specific commit/date. Requires versioned indexing, historical graph. | Deferred - Infrastructure heavy |
| Dependency Graph Search | Architecture discovery | Find all code that depends on X, or X depends on. Leverage edge relationships more deeply. | Phase 2 - Relationship Clustering |
| Code Quality Filters | Advanced filtering | Filter by complexity, test coverage, doc coverage, recent churn. Requires metric computation during indexing. | Deferred - Separate initiative |
| Search Within Search Scope | Iterative refinement | Use previous results as scope for next query. Requires result set persistence. | Deferred - After filtering |
| AI-Assisted Query Suggestions | LLM integration | Suggest better queries based on initial results. Requires LLM integration, prompt engineering. | Deferred - Post-transparency |
| Collaborative Search Sessions | Multi-user feature | Share search context with team members. Requires session management, collaboration infrastructure. | Out of Scope |

## Prioritization

### Near-Term (May Become Projects in Initiative)

- **Dependency Graph Search**: Natural extension of Phase 2 relationship clustering
- **Search History**: Low-hanging fruit after observability foundations

### Medium-Term (Future Initiative Candidates)

- **Multi-Repository Federated Search**: Separate initiative, clear user value
- **Natural Language Query Expansion**: Requires ML infrastructure, separate initiative
- **User Behavior Re-Ranking**: Builds on observability, medium complexity

### Long-Term (Research / Exploration)

- **Personalized Search**: Requires extensive user modeling
- **Time-Travel Search**: Heavy infrastructure investment
- **Cross-Language Symbol Search**: Interesting but niche use case
- **AI-Assisted Query Suggestions**: LLM integration, separate effort

### Out of Scope

- **Search Result Previews**: Client-side concern (MCP/VSCode)
- **Code Clone Detection**: Separate feature, different UX
- **Collaborative Search**: Multi-user complexity, different problem space

## Parking Lot

Ideas mentioned but not pursued:

1. **Query Auto-Correction** (e.g., "authenitcate" → "authenticate")
   - Reason: Low ROI, modern IDEs have autocomplete
   - May revisit: If typos become common pain point

2. **Search Analytics Dashboard**
   - Reason: Internal tooling, not user-facing
   - May revisit: If search quality monitoring becomes critical

3. **Integration with External Code Search** (Sourcegraph, GitHub Code Search)
   - Reason: Separate tool, different architecture
   - May revisit: If users request cross-tool workflows

4. **Voice-Activated Search**
   - Reason: Not applicable to CLI/MCP context
   - May revisit: Never (wrong modality)

5. **Search Within Documentation Only**
   - Reason: Subsumed by result type filtering (`type: 'docs'`)
   - Status: Already addressed in Phase 1

## Ideas From User Feedback

Original proposals from user session (status tracked):

| Proposal | Addressed In | Status |
|----------|-------------|--------|
| Query understanding with diagnostic hints | Phase 1 - SRCHTRNSP | In Scope |
| Result type filtering (docs vs code vs tests) | Phase 1 - SRCHFLTR | In Scope |
| Relationship-aware search (cluster related chunks) | Phase 2 - SRCHREL | In Scope |
| Progressive search results (confidence cutoff) | Phase 2 - SRCHCONF | In Scope |
| Search history & learning | Backlog - Near-term | Deferred |
| Multi-repository context | Backlog - Separate initiative | Deferred |
| Semantic understanding tests | Phase 3 - SRCHTST | In Scope |
| Architectural discovery tests | Phase 3 - SRCHTST | In Scope |
| Cross-cutting concern tests | Phase 3 - SRCHTST | In Scope |
| Code evolution queries (recent changes) | Backlog - Quality filters | Deferred |
| "Grep-impossible" task validation | Phase 3 - SRCHTST | In Scope |
| Performance & scale tests | Phase 3 - SRCHTST | In Scope |

## New Ideas During Research

Ideas discovered during codebase/industry research:

1. **Score Normalization Across Modes** (FTS vs Vector vs Hybrid)
   - Current: Different scoring systems, hard to compare
   - Opportunity: Normalize to 0-100 scale for consistency
   - Status: Included in Phase 2 confidence scoring

2. **Smart Mode Selection** (Auto-detect FTS vs Vector vs Hybrid)
   - Current: User must specify mode
   - Opportunity: Auto-select based on query characteristics
   - Status: Backlog - requires heuristics research

3. **Progressive Enhancement of Debug Mode**
   - Current: Boolean flag (on/off)
   - Opportunity: Levels (minimal, standard, verbose)
   - Status: Backlog - nice-to-have

4. **Filtering by Worktree/Branch**
   - Current: Search across all worktrees
   - Opportunity: Scope to specific worktree or branch
   - Status: Existing feature (worktree parameter)

5. **Result Clustering by File/Module**
   - Current: Flat list of chunks
   - Opportunity: Group results by file or module
   - Status: Included in Phase 2 relationship clustering

## Rejected Ideas

Ideas considered but explicitly rejected:

1. **Machine Learning-Based Ranking**
   - Reason: Requires training data, models, infrastructure
   - Better approach: Start with rule-based confidence scoring

2. **Full Database Schema Redesign**
   - Reason: Production migration complexity
   - Better approach: Work within existing schema, use JSON columns

3. **Real-Time Indexing**
   - Reason: Different problem space (indexing vs search)
   - Better approach: Separate initiative for indexing improvements

4. **GraphQL Search API**
   - Reason: Solves wrong problem (API format vs search quality)
   - Better approach: Improve current JSON-RPC interface

5. **Search Result Caching**
   - Reason: Premature optimization, daemon already fast
   - Better approach: Benchmark first, optimize if needed
