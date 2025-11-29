# VECFIX Ticket Index

## Project: vec_chunks Schema Fix

**Project Slug**: VECFIX
**Status**: Ready for Execution
**Total Tickets**: 4

## Phase 1: Code Cleanup and Migration

| Ticket | Title | Agent | Status | Dependencies |
|--------|-------|-------|--------|--------------|
| [VECFIX-1001](VECFIX-1001_remove-vec-chunks-code-migrate-callers.md) | Remove vec_chunks code and migrate callers (ATOMIC) | rust-indexer-engineer | Pending | None |
| [VECFIX-1002](VECFIX-1002_remove-vec-chunks-from-schema.md) | Remove vec_chunks from schema.rs | rust-indexer-engineer | Pending | VECFIX-1001 |

## Phase 2: Testing and Verification

| Ticket | Title | Agent | Status | Dependencies |
|--------|-------|-------|--------|--------------|
| [VECFIX-1003](VECFIX-1003_run-test-suite.md) | Run test suite and fix failures | unit-test-runner | Pending | VECFIX-1001, VECFIX-1002 |
| [VECFIX-1004](VECFIX-1004_e2e-verification.md) | E2E verification | verify-ticket | Pending | VECFIX-1001, VECFIX-1002, VECFIX-1003 |

## Execution Order

```
VECFIX-1001 (code removal + migration)
    ↓
VECFIX-1002 (schema cleanup)
    ↓
VECFIX-1003 (test suite)
    ↓
VECFIX-1004 (E2E verification)
```

## Plan References

- [Analysis](../planning/analysis.md) - Problem investigation and active callers
- [Architecture](../planning/architecture.md) - Solution design with migration path
- [Plan](../planning/plan.md) - Implementation phases and work items
- [Quality Strategy](../planning/quality-strategy.md) - Testing approach
- [Project Review](../planning/project-review.md) - Review status: Ready (95% success probability)

## Success Criteria

1. Zero references to `vec_chunks` in `mod.rs`, `schema.rs`, and `pipeline.rs`
2. Only `upsert_embedding()` (singular) remains - uses correct architecture
3. All tests pass
4. VSCode extension scan and embedding generation complete without errors
5. Embeddings properly stored in `code_embeddings` and synced to `vec_code`

---

🎯 **Next step**: Run `/review-tickets VECFIX` to validate quality or proceed to `/work-on-project VECFIX` to execute tickets
