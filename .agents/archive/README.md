# Archive

This directory contains completed projects and historical records. Projects are archived here after all tickets are complete and key learnings have been synthesized into the main `/docs` directory.

## Archived Projects

### HYBRID_SEARCH_hybrid-retrieval-system - Hybrid Retrieval System
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

| Project | Tickets | Status |
|---------|---------|--------|
| HYBRID_SEARCH_hybrid-retrieval-system | 22 | ✅ Complete |
| MPEMBED_multi-provider-embeddings | 33 | ✅ Complete |
| CONTEXT_ASM_context-assembly-engine | 14 | ✅ Complete |
| INC_INDEX_incremental-indexing | 8 | ✅ Complete |
| LANG_PARSE_multi-language-support | 20 | ✅ Complete |
| MCP_CORE_mcp-server-core | 6 | ✅ Complete |
| MD_ENHANCE_markdown-enhancement | 8 | ✅ Complete |
| PERF_OPT_performance-optimization | 10 | ✅ Complete |
| MAPROOM_misc-fixes | 3 | ✅ Complete |
| CODE_QUALITY_code-quality-improvements | 1 | ✅ Complete |
| **Total** | **125** | **All Complete** |

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
