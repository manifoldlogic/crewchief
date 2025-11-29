# Semantic Entry Point Ranking

**Project Slug:** SEMRANK
**Status:** Planning
**Priority:** 🔴 Critical - Blocks effective context tool usage

## Problem Statement

Maproom's full-text search (FTS) currently returns test files and documentation ahead of actual implementations when searching for functions by name. This breaks the core search → context → relationship graph workflow because users get the wrong chunk_id to start their exploration.

When an AI agent searches for "validate_provider", it should find `fn validate_provider` (the implementation), not `test_validate_provider` or documentation mentioning it. Starting from the wrong entry point causes context() to traverse test relationships instead of implementation relationships, fundamentally breaking maproom's value proposition.

## Proposed Solution

Enhance FTS ranking to leverage code semantics (chunk kind, symbol names) that maproom already extracts during indexing. Instead of competing with grep on speed, we'll compete on *correctness of entry points* for relationship graph traversal.

**Core Implementation:**
1. Kind-based boosting: Implementations rank above tests/docs regardless of term frequency
2. Symbol name exact matching: Boost when query matches symbol_name field
3. Combined scoring: Multiply FTS base score by semantic multipliers

This positions maproom as "structure-aware search for graph traversal," not "faster grep."

## Value Proposition Alignment

From maproom's product vision: *"Understands code as a living system of relationships, meanings, and contexts"*

This project ensures search returns the RIGHT entry points for context() to build the RIGHT relationship graph. It leverages metadata (kind, symbol_name) that text-based tools like grep fundamentally don't have.

## Key Success Metrics

- Search for "validate_provider" returns implementation as #1 result (not tests)
- Search for "authentication" returns AuthService.authenticate (entry point for graph traversal)
- AI agents can reliably find correct starting nodes for concept exploration
- Hybrid mode (already operational with RRF fusion) benefits immediately from improved FTS signal

## Relevant Agents

**Implementation:** database-engineer (Rust search ranking, SQL scoring)
**Testing:** integration-tester (search quality validation, ranking verification)
**Verification:** verify-ticket (acceptance criteria validation)
**Commit:** commit-ticket (conventional commit creation)

## Planning Documents

- [Analysis](planning/analysis.md) - Problem deep-dive and current state
- [Architecture](planning/architecture.md) - Scoring system design
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security considerations
- [Plan](planning/plan.md) - Execution phases and milestones

## Timeline

**Estimated Duration:** 3.5-4.5 weeks
**Estimated Tickets:** 21 tickets (including Phase 0 for MCP tool creation)
**Project Size:** Small-Medium (includes prerequisite MCP tool creation)

## Dependencies

**Blocks:** Effective use of context tool, VS Code integration exploration mode
**Blocked by:** None - TypeScript MCP search tool created in Phase 0
**Complements:** Operational hybrid search (improves RRF fusion's lexical component immediately)
