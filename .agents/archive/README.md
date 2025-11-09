# Archive

This directory contains completed projects and historical records. Projects are archived here after all tickets are complete and key learnings have been synthesized into the main `/docs` directory.

## Archived Projects

### Recently Archived (2025-11-09)

#### AGENTOPT_ai-agent-query-optimization - AI Agent Query Optimization
**Completed:** 14 tickets
**Summary:** Optimized agent query performance and search efficiency

**Contents:**
- [Planning Docs](./projects/AGENTOPT_ai-agent-query-optimization/planning/)
- [Tickets](./projects/AGENTOPT_ai-agent-query-optimization/tickets/) (14 completed)

---

#### BLOBSHA_content-addressed-chunk-storage - Content-Addressed Chunk Storage
**Completed:** 11 tickets
**Summary:** Blob SHA computation and embedding deduplication for 70-90% cost savings

**Contents:**
- [Planning Docs](./projects/BLOBSHA_content-addressed-chunk-storage/planning/)
- [Tickets](./projects/BLOBSHA_content-addressed-chunk-storage/tickets/) (11 completed)

**Key Achievements:**
- Zero data loss during migration
- 70-90% embedding cost savings via deduplication
- HNSW index on code_embeddings table

---

#### BRANCHX_branch-aware-indexing - Branch-Aware Indexing
**Completed:** 18 tickets
**Summary:** Worktree tracking and incremental updates (5-10x faster than full scans)

**Contents:**
- [Planning Docs](./projects/BRANCHX_branch-aware-indexing/planning/)
- [Tickets](./projects/BRANCHX_branch-aware-indexing/tickets/) (18 completed)

**Key Achievements:**
- Incremental updates 5-10x faster than full scans
- Tree SHA optimization <100ms for unchanged repos
- JSONB worktree_ids with GIN index
- Query filtering by worktree

---

#### BRWATCH_branch-switch-detection - Branch Switch Detection
**Completed:** 16 tickets
**Summary:** Automatic branch switch detection and re-indexing (<1s detection latency)

**Contents:**
- [Planning Docs](./projects/BRWATCH_branch-switch-detection/planning/)
- [Tickets](./projects/BRWATCH_branch-switch-detection/tickets/) (16 completed)

**Key Achievements:**
- 100% branch switch detection reliability
- Auto-triggering incremental updates
- Resource usage <5% CPU, <20MB RAM
- Graceful shutdown

---

#### MCPREL_mcp-release-scripts - MCP Release Scripts
**Completed:** 4 tickets
**Summary:** Automated MCP server release and publishing workflows

**Contents:**
- [Planning Docs](./projects/MCPREL_mcp-release-scripts/planning/)
- [Tickets](./projects/MCPREL_mcp-release-scripts/tickets/) (4 completed)

---

#### MRPROG_maproom-progress-ux - Maproom Progress UX
**Completed:** 17 tickets
**Summary:** Enhanced user experience for maproom indexing progress

**Contents:**
- [Planning Docs](./projects/MRPROG_maproom-progress-ux/planning/)
- [Tickets](./projects/MRPROG_maproom-progress-ux/tickets/) (17 completed)

---

#### TESTDES_test-design-framework - Test Design Framework (Stub)
**Completed:** 0/1 tickets (orphaned stub)
**Summary:** Contained ecological validation implementation summary (part of TESTDES_grep-impossible-task-design)

**Contents:**
- [Ticket](./projects/TESTDES_test-design-framework/tickets/) (1 implementation summary)

**Note:** This was an orphaned project stub. The actual test design work was completed in the archived TESTDES_grep-impossible-task-design project.

---

### Previously Archived

#### HYBRID_SEARCH_hybrid-retrieval-system - Hybrid Retrieval System
**Completed:** Phase 1 (22 tickets)
**Summary:** Combined FTS (full-text search) and vector similarity for semantic code search

**Contents:**
- [Planning Docs](./projects/HYBRID_SEARCH_hybrid-retrieval-system/planning/)
- [Tickets](./projects/HYBRID_SEARCH_hybrid-retrieval-system/tickets/) (22 completed)

**Key Learnings:** Documented in `/docs/architecture/`

---

### MPEMBED_multi-provider-embeddings - Multi-Provider Embeddings
**Completed:** All phases (33 tickets)
**Summary:** Support for multiple embedding providers (OpenAI, Ollama, Google Vertex AI)

**Contents:**
- [Planning Docs](./projects/MPEMBED_multi-provider-embeddings/planning/)
- [Tickets](./projects/MPEMBED_multi-provider-embeddings/tickets/) (33 completed)

**Key Learnings:** Provider abstraction patterns, dimension-based column storage

---

### CONTEXT_ASM_context-assembly-engine - Context Assembly
**Completed:** (14 tickets)
**Summary:** Context assembly engine for gathering related code chunks

**Contents:**
- [Planning Docs](./projects/CONTEXT_ASM_context-assembly-engine/planning/)
- [Tickets](./projects/CONTEXT_ASM_context-assembly-engine/tickets/)

---

### INC_INDEX_incremental-indexing - Incremental Indexing
**Completed:** (8 tickets)
**Summary:** Incremental file indexing and change detection

**Contents:**
- [Planning Docs](./projects/INC_INDEX_incremental-indexing/planning/)
- [Tickets](./projects/INC_INDEX_incremental-indexing/tickets/)

---

### LANG_PARSE_multi-language-support - Multi-Language Support
**Completed:** (20 tickets)
**Summary:** Tree-sitter integration for multiple programming languages

**Contents:**
- [Planning Docs](./projects/LANG_PARSE_multi-language-support/planning/)
- [Tickets](./projects/LANG_PARSE_multi-language-support/tickets/)

---

### MCP_CORE_mcp-server-core - MCP Server Core
**Completed:** (6 tickets)
**Summary:** Model Context Protocol server implementation

**Contents:**
- [Planning Docs](./projects/MCP_CORE_mcp-server-core/planning/)
- [Tickets](./projects/MCP_CORE_mcp-server-core/tickets/)

---

### MD_ENHANCE_markdown-enhancement - Markdown Enhancement
**Completed:** (8 tickets)
**Summary:** Enhanced markdown parsing with tree-sitter

**Contents:**
- [Planning Docs](./projects/MD_ENHANCE_markdown-enhancement/planning/)
- [Tickets](./projects/MD_ENHANCE_markdown-enhancement/tickets/)

---

### PERF_OPT_performance-optimization - Performance Optimization
**Completed:** (10 tickets)
**Summary:** Query performance optimization and indexing strategies

**Contents:**
- [Planning Docs](./projects/PERF_OPT_performance-optimization/planning/)
- [Tickets](./projects/PERF_OPT_performance-optimization/tickets/)

---

### MAPROOM_misc-fixes - Misc Maproom Fixes
**Completed:** (3 tickets)
**Summary:** General bug fixes and improvements

**Contents:**
- [Tickets](./projects/MAPROOM_misc-fixes/tickets/)

---

### CODE_QUALITY_code-quality-improvements - Code Quality Improvements
**Completed:** (1 ticket)
**Summary:** Rust compiler warnings and code quality fixes

**Contents:**
- [Tickets](./projects/CODE_QUALITY_code-quality-improvements/tickets/)

---

## Session Summaries

Historical session summaries and progress reports:

- [Session Summary 2025-10-28](./sessions/SESSION_SUMMARY_2025-10-28.md)
- [Session Continuation 2025-10-28](./sessions/SESSION_SUMMARY_2025-10-28_CONTINUATION.md)
- [Final Summary 2025-10-28](./sessions/FINAL_SESSION_SUMMARY_2025-10-28.md)
- [Ticket Status Update 2025-10-28](./sessions/TICKET_STATUS_UPDATE_2025-10-28.md)

## Master Indexes

- [INDEX_BY_PROJECT.md](./INDEX_BY_PROJECT.md) - Master project and ticket index

## Statistics

| Project | Tickets | Archived | Status |
|---------|---------|----------|--------|
| AGENTOPT_ai-agent-query-optimization | 14 | 2025-11-09 | ✅ Complete |
| BLOBSHA_content-addressed-chunk-storage | 11 | 2025-11-09 | ✅ Complete |
| BRANCHX_branch-aware-indexing | 18 | 2025-11-09 | ✅ Complete |
| BRWATCH_branch-switch-detection | 16 | 2025-11-09 | ✅ Complete |
| MCPREL_mcp-release-scripts | 4 | 2025-11-09 | ✅ Complete |
| MRPROG_maproom-progress-ux | 17 | 2025-11-09 | ✅ Complete |
| TESTDES_test-design-framework | 0 | 2025-11-09 | ✅ Archived (stub) |
| TESTDES_grep-impossible-task-design | 21 | 2025-11-07 | ✅ Complete |
| HYBRID_SEARCH_hybrid-retrieval-system | 22 | (earlier) | ✅ Complete |
| MPEMBED_multi-provider-embeddings | 33 | (earlier) | ✅ Complete |
| CONTEXT_ASM_context-assembly-engine | 14 | (earlier) | ✅ Complete |
| INC_INDEX_incremental-indexing | 8 | (earlier) | ✅ Complete |
| LANG_PARSE_multi-language-support | 20 | (earlier) | ✅ Complete |
| MCP_CORE_mcp-server-core | 6 | (earlier) | ✅ Complete |
| MD_ENHANCE_markdown-enhancement | 8 | (earlier) | ✅ Complete |
| PERF_OPT_performance-optimization | 10 | (earlier) | ✅ Complete |
| MAPROOM_misc-fixes | 3 | (earlier) | ✅ Complete |
| CODE_QUALITY_code-quality-improvements | 1 | (earlier) | ✅ Complete |
| **Total** | **226** | | **All Complete** |

## Archive Structure

```
archive/
├── projects/                  # Completed projects
│   └── {SLUG}_{descriptive-name}/
│       ├── README.md         # Project overview (optional)
│       ├── planning/         # Planning documents
│       └── tickets/          # Completed tickets
├── sessions/                  # Historical session summaries
└── INDEX_BY_PROJECT.md       # Master index
```

## Using Archived Content

Archived projects provide valuable context for:
- Understanding historical design decisions
- Learning from past implementation approaches
- Finding similar patterns for new work
- Onboarding new team members

```bash
# Search for specific topics in archived tickets
grep -r "embedding" .agents/archive/projects/MPEMBED_multi-provider-embeddings/tickets/

# Review architecture decisions
cat .agents/archive/projects/HYBRID_SEARCH_hybrid-retrieval-system/planning/architecture.md
```

---

For active projects, see [Projects](../projects/README.md).
