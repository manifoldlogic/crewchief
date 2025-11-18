# OPNFIX: Open Tool Path Resolution Fix

**Status:** Ready for Implementation
**Priority:** 🔴 Critical
**Timeline:** 2-3 days (revised from 3-5 days)
**Size:** XS (15 tickets)

## Problem Summary

The `mcp__maproom__open` tool is completely broken due to path resolution bugs. When users attempt to retrieve file contents, the tool constructs invalid paths by duplicating path segments:

```
Expected: /workspace/crates/maproom/src/main.rs
Actual:   /workspace/crates/maproom/crates/maproom/src/main.rs
                                    ^^^^^^^^^^^^^^^^^^^ DUPLICATED
```

This breaks the fundamental MCP workflow: **search → get chunk → open file → read contents**.

## Root Cause

Database pollution with inconsistent worktree `abs_path` values. Multiple indexing runs from different directories created conflicting path relationships that the open tool trusts blindly.

**The Bug:**
```typescript
// open.ts queries database for first match
SELECT w.abs_path FROM maproom.worktrees w
WHERE w.name = $1 LIMIT 1

// Returns wrong abs_path (pollution)
// Joins with relpath creating duplicate
path.join('/workspace/crates/maproom', 'crates/maproom/src/main.rs')
// Result: /workspace/crates/maproom/crates/maproom/src/main.rs ❌
```

## Solution

Add defensive programming to validate paths against the filesystem:

1. **Query all candidates** (not just first match)
2. **Order deterministically** (newest first)
3. **Validate each candidate** against filesystem
4. **Return first valid path** (automatic fallback)
5. **Provide clear errors** when all fail

**Result:** Works with both clean and polluted databases.

## Project Documents

All planning documents are in the `planning/` directory:

### 📋 [analysis.md](planning/analysis.md)
**Deep investigation of the bug and why it exists**
- Detailed failure analysis
- Root cause explanation
- Database schema review
- Impact assessment

### 🏗️ [architecture.md](planning/architecture.md)
**Solution design and implementation approach**
- Current vs. proposed architecture
- Key design decisions
- Error handling strategy
- Performance considerations

### 🧪 [quality-strategy.md](planning/quality-strategy.md)
**Why tests didn't catch this and how to prevent recurrence**
- Detailed test gap analysis
- Autopsy of skipped integration tests
- New test requirements
- Prevention strategies

**Key Finding:** End-to-end tests were skipped with comment "implement when test fixtures are available" - they were never implemented.

### 🔒 [security-review.md](planning/security-review.md)
**Security analysis and validation strategy**
- Threat modeling
- Path traversal prevention
- Symlink handling
- Security test requirements

### 📅 [plan.md](planning/plan.md)
**5-phase implementation plan with 15 tickets**
- Phase 1: Core fix (4-6 hours)
- Phase 2: Security enhancements (3-4 hours)
- Phase 3: Comprehensive tests (8-10 hours)
- Phase 4: Documentation (2-3 hours)
- Phase 5: Verification and deployment (2-3 hours)

## Key Insights

### Why This Bug Happened

1. **Database pollution** - Multiple scans with different root paths
2. **Blind trust** - Code trusted database data without validation
3. **Skipped tests** - E2E tests were never implemented
4. **No contract validation** - Database ↔ Code contract was implicit, untested

### Why Tests Didn't Catch It

**Unit tests:** Only tested validation functions in isolation with mocked data.

**Integration tests:** The critical E2E tests were **skipped**:

```typescript
// packages/maproom-mcp/tests/tools/open.int.test.ts:199-207
it.skip('should handle full workflow: filesystem read', async () => {
  // This would require a fully set up test environment with database data
  // Marked as skip for now - implement when test fixtures are available
})
```

**Result:** High coverage of individual functions, zero coverage of complete workflow.

## Success Criteria

This project is complete when:

- ✅ Open tool successfully reads files with database data
- ✅ Path resolution handles pollution automatically via fallback
- ✅ All security validations block path traversal attacks
- ✅ Comprehensive E2E test suite validates workflows
- ✅ No integration tests are skipped
- ✅ Error messages are clear and actionable
- ✅ Performance impact is <10ms per operation
- ✅ Documentation is updated

## Agent Assignments

**Primary:** general-purpose or vscode-extension-specialist
- TypeScript/Node.js implementation
- MCP tool modifications
- Unit test updates

**Testing:** integration-tester
- E2E test suite creation
- Security test implementation
- Un-skipping integration tests

**Verification:** verify-ticket
- Final acceptance criteria validation
- Manual workflow verification
- Performance testing

## Dependencies

**Required:**
- PostgreSQL database (✅ available)
- Test database setup (✅ available)

**Blockers:** None

**Related Projects:**
- Project 3: Index Stale Worktree Cleanup (prevents future pollution)
- Project 1: Search Exact Match Priority (unblocks context tool)

## Timeline

**Total:** 2-3 days (13-18 hours)

**Revision Note:** Timeline reduced from original 3-5 days (19-26 hours) after identifying existing test infrastructure (`tests/helpers/database.ts`, `tests/fixtures/`) that eliminates need to build setup utilities from scratch. Saves 4-6 hours in Phase 3.

**Critical Path:**
```
Day 1: Core fix (Phase 1) → Day 1-2: Security (Phase 2)
Day 2: Tests (Phase 3) → Day 2-3: Docs (Phase 4) → Day 3: Deploy (Phase 5)
```

## Quick Start

To begin implementation:

```bash
# 1. Review all planning documents
cd .agents/projects/OPNFIX_open-path-fix/planning

# 2. Read in order:
#    - analysis.md (understand the bug)
#    - architecture.md (understand the solution)
#    - quality-strategy.md (understand test requirements)
#    - security-review.md (understand security concerns)
#    - plan.md (understand implementation phases)

# 3. Create tickets from plan.md phases

# 4. Execute with /single-ticket for each ticket
```

## Risk Assessment

**Technical Risk:** Low
- Simple code changes
- No database migrations
- No breaking API changes
- Easy rollback

**Schedule Risk:** Low
- Well-scoped tasks
- Clear acceptance criteria
- No external dependencies

**Quality Risk:** Low (with tests)
- Comprehensive test plan
- Security validation
- Manual verification phase

## Stakeholder Impact

**Users:**
- ✅ Open tool becomes functional
- ✅ Can read files from search results
- ✅ Context tool becomes usable (depends on open)

**Developers:**
- ✅ Better test coverage prevents regressions
- ✅ Clear error messages aid debugging
- ✅ Documentation explains behavior

**Operations:**
- ✅ Tool works even with database pollution
- ✅ Clear logs for troubleshooting
- ✅ Monitoring metrics available

## Next Steps

1. **Review planning documents** - Understand full context
2. **Approve project scope** - Confirm approach is correct
3. **Create tickets** - From plan.md phases
4. **Assign to agent** - Begin with Phase 1
5. **Execute systematically** - Use /single-ticket workflow

**This project is ready for implementation.**

---

## Document Index

- **README.md** (this file) - Project overview and quick start
- **planning/analysis.md** - Bug investigation and root cause
- **planning/architecture.md** - Solution design
- **planning/quality-strategy.md** - Test strategy and gap analysis
- **planning/security-review.md** - Security validation
- **planning/plan.md** - 5-phase implementation plan
- **tickets/** - Individual work tickets (to be created)

## Contact

For questions about this project, refer to the planning documents or the original failure analysis:
- `.agents/reports/2025-11-18_maproom-mcp-context-tool-failure-analysis.md`
- `.agents/reports/2025-11-18_maproom-mcp-projects-breakdown.md`
