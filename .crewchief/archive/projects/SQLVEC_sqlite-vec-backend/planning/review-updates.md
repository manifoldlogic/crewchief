# Project Review Updates

**Original Review Date:** November 25, 2025
**Updates Completed:** November 25, 2025
**Update Status:** Complete

## Critical Issues Addressed

### Issue 1: Vector Dimension Limits
**Mitigation Applied:**
- **plan.md**: Added **Phase 0: Prototype Build** (Ticket 0) to specifically verify 1536-dim support and static linking *before* starting the main refactor.
- **architecture.md**: Added validation note for `vec0` table.

### Issue 2: FTS Dialect Incompatibility
**Mitigation Applied:**
- **architecture.md**: Explicitly noted that SQL strings cannot be shared. The `VectorStore` implementation must handle dialect-specific query construction. Added `query_builder.rs` concept (impl-specific).

## High-Risk Mitigations Implemented

### Risk 1: Concurrency & Locking (SQLITE_BUSY)
**Mitigation Applied:**
- **architecture.md**: Added requirement for `PRAGMA journal_mode=WAL` and connection pooling (`r2d2` / `deadpool`).
- **plan.md**: Added WAL mode setup to Ticket 4.

### Risk 2: Build Complexity
**Mitigation Applied:**
- **plan.md**: Ticket 0 covers the cross-platform build verification.

## Verification

**Next Steps:**
1. Run `/create-project-tickets SQLVEC`.

**Success Metrics:**
- [x] Phase 0 ticket added to derisk build.
- [x] Concurrency strategy defined (WAL).
- [x] FTS dialect strategy defined (Separate impls).
