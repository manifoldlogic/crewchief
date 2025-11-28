# Project Review Updates

**Original Review Date:** 2025-01-28
**Updates Completed:** 2025-01-28
**Update Status:** Complete

## Critical Issues Addressed

### Issue 1: Components Not Exported
**Original Problem:** `setup_head_watcher()`, `DebouncedHandler`, and `BranchSwitchEvent` exist in `indexer/mod.rs` but are marked `#[allow(dead_code)]` and not exported publicly.
**Changes Made:**
- architecture.md: Added "Prerequisites: Module Exports (Phase 0)" section explaining export requirements
- architecture.md: Updated "Existing Components to Reuse" table with Status and Action Required columns
- plan.md: Added Phase 0 with explicit export tasks before implementation
- README.md: Updated "Existing Components to Reuse" to show export status
**Result:** Issue resolved - explicit Phase 0 task added to export components before use

### Issue 2: handle_branch_switch Function Removed
**Original Problem:** The `handle_branch_switch()` function was removed during IDXABS-2001 SQLite migration.
**Changes Made:**
- architecture.md: Updated "Branch Switch Handler" section with **NOTE** that this is NEW code
- architecture.md: Added complete SQLite-specific implementation with SqliteStore methods
- architecture.md: Added error handling pattern (log and continue, don't crash)
- plan.md: Added "Codebase State Notes" section documenting removed function
- plan.md: Clarified Phase 3 implements NEW function with full specification reference
**Result:** Issue resolved - planning docs now reflect this requires new implementation

### Issue 3: Disabled Unit Tests
**Original Problem:** Multiple UNIWATCH-prefixed tests are disabled with `#[cfg(disabled_postgresql_test)]`.
**Changes Made:**
- quality-strategy.md: Added "Disabled Tests Status" section with tables of tests to enable vs working tests
- quality-strategy.md: Added Test 5 for Detached HEAD state
- plan.md: Added Phase 4 Task 1 specifically for enabling/rewriting disabled tests
- plan.md: Updated Agent Assignments to show rust-indexer-engineer handles test migration
**Result:** Issue resolved - test migration explicitly planned with responsible agent

## High-Risk Mitigations Implemented

### Risk 1: E2E Test Uses PostgreSQL
**Mitigation Applied:**
- quality-strategy.md: Added "E2E Test Script Migration" section with before/after examples
- quality-strategy.md: Showed SQLite equivalents for `psql` commands
- plan.md: Phase 4 Task 3 explicitly mentions "Update E2E test script for SQLite"
**Risk Level:** Reduced from High to Low

### Risk 2: Race Condition Between File and Branch Events
**Mitigation Applied:**
- architecture.md: Added "Event Ordering Strategy" section explaining tokio::select! behavior
- architecture.md: Documented atomic state update and worst-case scenario
- architecture.md: Added timeline diagram showing sequential processing
**Risk Level:** Reduced from Medium to Low

### Risk 3: Detached HEAD State Handling
**Mitigation Applied:**
- architecture.md: Added "Detached HEAD Handling" section with 4-step process
- architecture.md: Specified 8-character SHA via `git rev-parse --short=8 HEAD`
- quality-strategy.md: Added Test 5 for detached HEAD scenario
- plan.md: Phase 3 includes "Handle detached HEAD: if branch == HEAD, use 8-char commit SHA"
**Risk Level:** Reduced from Medium to Low

## Gaps Filled

### Requirements Gaps
- **NDJSON Event Destination:** Added "NDJSON Output Destination" section in architecture.md
  - Confirmed stdout is correct (for VSCode extension, CLI piping)
  - Noted tracing goes to stderr for separation
- **Error Recovery Strategy:** Added "Error Recovery Strategy" section in architecture.md
  - Database errors: Log and continue with old worktree_id
  - Git errors: Log and continue, retry on next event
  - Indexing errors: Non-fatal, continue watching
  - Added code example showing error handling pattern

### Technical Gaps
- **RwLock Type:** Added explicit note in architecture.md "Dynamic Worktree State" section
  - "Use `std::sync::RwLock`, NOT `tokio::sync::RwLock`"
  - Referenced existing codebase patterns
  - Added imports: `use std::sync::{Arc, RwLock};`
- **Database Methods:** Updated components table to show `get_or_create_worktree()` is public and ready
- **Verified:** `get_repo_id()` method exists (confirmed via grep during review)

### Process Gaps
- **Agent Handoff:** Updated Agent Assignments table in plan.md
  - Phase 4 Task 1 (Enable tests) → rust-indexer-engineer
  - Phase 4 Tasks 2-3 (Integration Tests) → integration-tester
  - Clear sequential handoff

## Scope Adjustments

### Clarified Boundaries
- **Phase 0 added:** Module exports prerequisite for all other phases
- **Phase 4 restructured:** 4 sub-tasks with clear agent assignments
- **Out of scope unchanged:** multi-repository watching, new CLI commands, schema changes

### No Scope Creep
- Detached HEAD handling was implicit, now explicit (no new scope)
- Error handling was implicit, now explicit (no new scope)

## Document Change Summary

### architecture.md
- Lines modified: ~100
- Key changes:
  - Added "Prerequisites: Module Exports (Phase 0)" section
  - Added `std::sync::RwLock` specification
  - Updated "Branch Switch Handler" with NEW implementation note and SQLite code
  - Added "Error Recovery Strategy" section
  - Added "Detached HEAD Handling" section
  - Added "Event Ordering Strategy" section
  - Added "NDJSON Output Destination" section
  - Updated components table with Status and Action Required columns

### plan.md
- Lines modified: ~50
- Key changes:
  - Added "Codebase State Notes" section
  - Added "Phase 0: Module Exports" with explicit tasks
  - Updated Phase 1 with RwLock import specification
  - Updated Phase 3 with NEW function note and full specification
  - Restructured Phase 4 with 4 sub-tasks including test migration
  - Updated File Changes table
  - Updated Existing Code to Reuse with two tables (needs export / ready)
  - Updated Agent Assignments with Notes column

### quality-strategy.md
- Lines modified: ~70
- Key changes:
  - Added "Disabled Tests Status" section with test tables
  - Updated Test Pyramid to show 7 unit tests
  - Added Test 5 for Detached HEAD state
  - Updated Manual Testing Checklist with 2 new items
  - Updated Error Scenarios table
  - Added "E2E Test Script Migration" section
  - Updated Test Execution section

### README.md
- Lines modified: ~25
- Key changes:
  - Added "Updates from Project Review" section
  - Split components table into "needs export" and "ready to use"
  - Updated Agents table with Notes column

## Verification

**Success Metrics:**
- [x] All critical issues resolved (3/3)
- [x] High-risk areas mitigated (3/3)
- [x] Requirements specific and measurable
- [x] Scope appropriate for MVP
- [x] Plan ready for ticket creation

**Next Steps:**
1. Run `/review-project UNIWATCH` to verify improvements (optional)
2. Proceed to `/create-project-tickets UNIWATCH` to generate tickets
3. Execute tickets sequentially using assigned agents
