# PROVFIX Ticket Index

## Project Overview

**Project:** PROVFIX - Maproom Provider Configuration Fixes
**Total Tickets:** 8
**Estimated Effort:** 5-6 hours
**Status:** Ready for implementation

## Problem Summary

Critical bugs in Maproom embedding provider configuration:
- OpenAI provider attempts connection to Ollama endpoint (localhost:11434)
- Database updates fail due to missing `updated_at` column
- CLI contains workaround code masking underlying Rust bugs
- Docker Compose defaults cause cross-provider endpoint pollution

## Solution Approach

Fix root causes in Rust, remove CLI workarounds, address database schema, clean up Docker configuration.

---

## Phase 1: Rust Core Fixes (Critical Path)

**Objective:** Fix endpoint resolution bug in Rust codebase
**Estimated Effort:** 2-3 hours
**Can Start:** Immediately

### PROVFIX-1001: Fix Rust Endpoint Resolution
- **Status:** ⏳ Ready to start
- **File:** `PROVFIX-1001_fix-rust-endpoint-resolution.md`
- **Agent:** general-purpose (or rust-specialist)
- **Summary:** Fix `EmbeddingConfig::from_env()` to validate endpoints match provider domain, prevent cross-provider pollution
- **Key Changes:** Modify config.rs to add provider-aware endpoint loading with domain validation
- **Dependencies:** None (first ticket)
- **Blocks:** PROVFIX-3001 (CLI cleanup depends on this fix)

### PROVFIX-1002: Add Endpoint Unit Tests
- **Status:** ⏳ Ready after PROVFIX-1001
- **File:** `PROVFIX-1002_add-endpoint-unit-tests.md`
- **Agent:** general-purpose (or rust-specialist)
- **Summary:** Add 8 comprehensive unit tests to verify endpoint resolution, including critical regression test
- **Key Tests:** `test_openai_ignores_ollama_endpoint` (THE BUG TEST)
- **Dependencies:** Requires PROVFIX-1001
- **Blocks:** PROVFIX-3001 (tests must prove fix works before removing workarounds)

### PROVFIX-1901: Test Critical Path (Phase 1 Test Ticket)
- **Status:** ⏳ Ready after PROVFIX-1001, 1002, 2001
- **File:** `PROVFIX-1901_test-critical-path.md`
- **Agent:** unit-test-runner
- **Summary:** Quick critical path validation - run unit tests + optional smoke tests (< 5 min)
- **Key Focus:** Verify bug fix works, database schema correct
- **Dependencies:** Requires PROVFIX-1001, 1002, 2001
- **Notes:** MVP test strategy - focused on preventing regression only

---

## Phase 2: Database Schema Fix (Independent)

**Objective:** Add missing `updated_at` column to chunks table
**Estimated Effort:** 1 hour
**Can Start:** Immediately (parallel with Phase 1)

### PROVFIX-2001: Add updated_at Column
- **Status:** ⏳ Ready to start
- **File:** `PROVFIX-2001_add-updated-at-column.md`
- **Agent:** general-purpose
- **Summary:** Create migration to add `updated_at TIMESTAMPTZ` column with auto-update trigger
- **Key Changes:** New migration file in `migrations/` folder
- **Dependencies:** None (independent of Phase 1)
- **Can Run:** In parallel with PROVFIX-1001/1002

---

## Phase 3: Remove CLI Workarounds

**Objective:** Clean up CLI code after Rust fixes proven
**Estimated Effort:** 30 minutes
**Can Start:** After Phase 1 complete

### PROVFIX-3001: Remove CLI Workarounds
- **Status:** ⏳ Ready after PROVFIX-1001, 1002
- **File:** `PROVFIX-3001_remove-cli-workarounds.md`
- **Agent:** general-purpose (JavaScript/Node.js)
- **Summary:** Remove explicit endpoint-setting workaround from 3 CLI functions
- **Key Changes:** Remove `EMBEDDING_API_ENDPOINT` assignment in runScan, runSetup, upsertFiles
- **Dependencies:** Requires PROVFIX-1001 (Rust fix), PROVFIX-1002 (tests)
- **Blocks:** PROVFIX-5001 (integration testing needs clean code)

---

## Phase 4: Docker Compose Cleanup

**Objective:** Remove Docker defaults that caused bug
**Estimated Effort:** 15 minutes
**Can Start:** After Phase 3 complete

### PROVFIX-4001: Clean Docker Defaults
- **Status:** ⏳ Ready after PROVFIX-3001
- **File:** `PROVFIX-4001_clean-docker-defaults.md`
- **Agent:** general-purpose (YAML config)
- **Summary:** Remove or clear default `EMBEDDING_API_ENDPOINT` from docker-compose.yml
- **Key Changes:** Change default from `http://ollama:11434` to empty string
- **Dependencies:** Requires PROVFIX-3001 (CLI cleanup)
- **Testing:** Verify OpenAI and Ollama still work

---

## Phase 5: Integration Testing

**Objective:** Verify complete fix across all scenarios
**Estimated Effort:** 1 hour
**Can Start:** After all implementation phases complete

### PROVFIX-5001: Integration Testing
- **Status:** ⏳ Ready after PROVFIX-1001, 1002, 2001, 3001, 4001
- **File:** `PROVFIX-5001_integration-testing.md`
- **Agent:** integration-tester (or general-purpose)
- **Summary:** Comprehensive end-to-end testing across 4 critical scenarios
- **Key Tests:** OpenAI clean env, Ollama defaults, environment precedence, database verification
- **Dependencies:** Requires ALL previous tickets complete
- **Blocks:** PROVFIX-6001 (documentation should reflect tested behavior)
- **Quality Gate:** All scenarios must pass before marking complete

---

## Phase 6: Documentation Updates

**Objective:** Document fixes and how provider config works
**Estimated Effort:** 30 minutes
**Can Start:** After integration testing passes

### PROVFIX-6001: Update Documentation
- **Status:** ⏳ Ready after PROVFIX-5001
- **File:** `PROVFIX-6001_update-documentation.md`
- **Agent:** general-purpose (technical writing)
- **Summary:** Update README, add code comments, update CHANGELOG with bug fixes
- **Key Changes:** Environment variables section, troubleshooting guide, code comments in config.rs
- **Dependencies:** Requires PROVFIX-5001 (document actual tested behavior)
- **Final Ticket:** Marks project complete

---

## Execution Order

### Critical Path (Must be sequential):
1. PROVFIX-1001 (Fix Rust endpoint resolution)
2. PROVFIX-1002 (Add unit tests)
3. PROVFIX-3001 (Remove CLI workarounds)
4. PROVFIX-4001 (Clean Docker defaults)
5. PROVFIX-5001 (Integration testing)
6. PROVFIX-6001 (Update documentation)

### Can Run in Parallel:
- PROVFIX-2001 (Database schema) - Independent of Phase 1

### Test Tickets:
- PROVFIX-1901 (Critical path test) - After Phase 1 + Phase 2 implementation

---

## Dependencies Graph

```
[Phase 1]
PROVFIX-1001 (Rust fix)
    ↓
PROVFIX-1002 (Unit tests)
    ↓
    ├─→ PROVFIX-1901 (Test critical path) ←─┐
    │                                        │
    ↓                                        │
[Phase 3]                                    │
PROVFIX-3001 (CLI cleanup)                   │
    ↓                                        │
[Phase 4]                                    │
PROVFIX-4001 (Docker cleanup)                │
    ↓                                        │
    └───────────────┬────────────────────────┘
                    ↓
[Phase 5]           ↓
PROVFIX-5001 (Integration testing)
    ↓
[Phase 6]
PROVFIX-6001 (Documentation)

[Phase 2 - Independent]
PROVFIX-2001 (Database schema)
    ↓
    └─→ PROVFIX-1901 (Test critical path)
```

---

## Success Metrics

### Before Fixes:
- ❌ OpenAI: Connection refused to localhost:11434
- ❌ Database: Column updated_at does not exist
- ⚠️ CLI: Workaround code in 3 places

### After Fixes:
- ✅ OpenAI: Embeddings generate successfully
- ✅ Database: Updates persist without errors
- ✅ CLI: Clean code, no workarounds
- ✅ All providers work with clear precedence rules

---

## Planning Documents

All planning documents are in: `.agents/projects/PROVFIX_maproom-provider-fixes/planning/`

- **analysis.md** - Deep dive into bugs, root causes, industry context
- **architecture.md** - Proposed solutions, implementation details
- **plan.md** - Phased implementation plan with effort estimates
- **quality-strategy.md** - Pragmatic testing approach, test plans
- **security-review.md** - Security implications (low risk, improves posture)

---

## Quick Start

To begin implementation:

1. **Start with Phase 1:** Assign PROVFIX-1001 to general-purpose or rust-specialist agent
2. **Parallel work:** Assign PROVFIX-2001 to another agent (database work independent)
3. **Follow critical path:** Each phase depends on previous (except Phase 2)
4. **Test checkpoints:** Run PROVFIX-1901 after Phase 1+2, PROVFIX-5001 before Phase 6
5. **Document last:** PROVFIX-6001 captures actual tested behavior

---

## Risk Summary

**Overall Risk:** Low
- Changes localized to config loading
- Workarounds provide rollback path
- Existing tests catch regressions

**Technical Risks:** Minimal
- Database migration: Standard PostgreSQL pattern
- Rust changes: Well-tested with comprehensive unit tests
- CLI cleanup: Simple removal of workaround code

**Security Impact:** Improved
- Prevents unintended endpoint usage
- Reduces configuration complexity
- API keys sent to correct provider only

---

## Total Ticket Count: 8

- Phase 1 Implementation: 2 tickets (PROVFIX-1001, 1002)
- Phase 1 Testing: 1 ticket (PROVFIX-1901)
- Phase 2: 1 ticket (PROVFIX-2001)
- Phase 3: 1 ticket (PROVFIX-3001)
- Phase 4: 1 ticket (PROVFIX-4001)
- Phase 5: 1 ticket (PROVFIX-5001)
- Phase 6: 1 ticket (PROVFIX-6001)

**Status:** All tickets created and ready for assignment.
