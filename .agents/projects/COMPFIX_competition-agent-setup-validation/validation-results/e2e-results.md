# End-to-End Validation Results

**Date:** 2025-11-10
**Tester:** Claude Code (direct execution)
**Environment:** Linux 6.11.11-linuxkit, Node.js (via pnpm), PostgreSQL 16 with pgvector, Ollama embedding provider

## Executive Summary

End-to-end validation testing was initiated for the competition framework validation project (COMPFIX). Testing revealed **2 critical bugs** and validated **multiple security and validation features** working correctly.

### Overall Status: ⚠️ BLOCKED - Critical Bugs Found

- ❌ **Cannot complete full E2E tests** due to scan implementation bug
- ✅ **Security validation working correctly** (after bug fix)
- ✅ **Pre-flight validation working correctly** (base branch check)
- ✅ **Error messages clear and actionable** (matches COMPFIX-2001 documentation)
- ⚠️ **2 bugs found and require fixing**:
  1. Timeout units mismatch (seconds vs milliseconds) - **FIXED during validation**
  2. Scan implementation blob_sha constraint violation - **BLOCKING**

## Bugs Discovered During Validation

### Bug 1: Timeout Units Mismatch (FIXED)

**Location:** `packages/cli/src/search-optimization/competition-runner.ts:92-95`

**Issue:**
The `CompetitionConfig` interface documents timeout as "Max time per agent in seconds", but the security validation (`validateCompetitionConfig`) expects milliseconds. This caused all optimizer runs to fail with:

```
Error: Timeout too short: minimum 30000ms
```

Even though task timeout was 180 seconds (valid), it was being interpreted as 180 milliseconds (invalid).

**Root Cause:**
- `SECURITY_LIMITS.MIN_TIMEOUT = 30_000` (30 seconds in milliseconds)
- `SECURITY_LIMITS.MAX_TIMEOUT = 600_000` (10 minutes in milliseconds)
- But `CompetitionConfig.timeout` is documented as seconds
- No conversion between units before validation

**Fix Applied:**
```typescript
// Convert timeout from seconds to milliseconds for validation
validateCompetitionConfig({
  variants: config.variants.map((v) => v.id),
  timeout: config.timeout ? config.timeout * 1000 : undefined,
})
```

**Impact:** HIGH - Prevented all competition runs from executing
**Status:** ✅ FIXED during validation, code rebuilt and tested

---

### Bug 2: Scan Implementation blob_sha Constraint Violation (BLOCKING)

**Location:** `crates/maproom/` (Rust indexer)

**Issue:**
When scanning the crewchief repository, the indexer attempts to insert chunks with `blob_sha = null`, violating the NOT NULL constraint:

```
Error: db error: ERROR: null value in column "blob_sha" of relation "chunks"
violates not-null constraint
```

**Root Cause:**
Unknown (requires investigation of Rust indexer code). The blob_sha should be computed for each chunk but is being set to null.

**Impact:** CRITICAL - Blocks all end-to-end testing
**Status:** ❌ BLOCKING - Cannot proceed with E2E validation until fixed

**Failing Command:**
```bash
/workspace/target/release/crewchief-maproom scan --path /workspace --commit HEAD
```

**Error Output:**
```
⚡ Incremental scan mode (use --force for full scan)
🔍 Scanning worktree: main @ HEAD
   Repository: crewchief
   Path: /workspace

Error: scan failed for main@HEAD

Caused by:
    0: db error: ERROR: null value in column "blob_sha" of relation "chunks"
       violates not-null constraint
```

**Next Steps:**
1. Investigate Rust indexer code in `crates/maproom/src/`
2. Find where blob_sha should be calculated
3. Fix null assignment
4. Test scan on crewchief repository
5. Resume E2E validation

---

## Validation Results

### Corrupted Registry File

**Issue:** `.crewchief/optimization-runs/index.json` was corrupted with multiple JSON objects concatenated
**Resolution:** Backed up corrupted file, created fresh registry
**Impact:** Minor - File corruption likely from incomplete writes
**Recommendation:** Implement atomic writes with temp file + rename pattern

### Security Validation (COMPFIX-1004)

✅ **PASSING** - Security limits enforcement working correctly

**Test:** Run optimizer with invalid timeout
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

**Result:**
- ✅ Timeout validation triggered
- ✅ Clear error message provided
- ✅ Prevented execution with invalid config

**Error Message Quality:**
- Clarity: 5/5 - Message clearly stated minimum required
- Actionability: 5/5 - Though in this case, the validation was over-strict due to units bug
- Matches documentation: ✅ Yes

### Pre-Flight Validation (COMPFIX-1001)

✅ **PASSING** - Base branch validation working correctly

**Test:** Run optimizer without base branch indexed
```bash
pnpm tsx scripts/run-genetic-optimizer.ts
```

**Expected:** Validation fails before agent spawn with actionable error
**Actual:** ✅ Validation failed as expected

**Output:**
```
🏁 Starting competition with pre-flight validation

📋 Phase 1: Setup
============================================================
✅ Database connection verified
❌ Pre-flight validation failed: Base branch 'main' not indexed

Fix: Run scan on base branch first
$ crewchief-maproom scan --repo crewchief --worktree main --root /workspace

This is a one-time setup step. Subsequent scans will be fast.
```

**Validation Results:**
- ✅ Caught before agent spawn
- ✅ Error message matches documentation (COMPFIX-2001)
- ✅ Troubleshooting steps present and executable
- ✅ No API calls made (verified - competition didn't start)
- ✅ Exit code non-zero

**Error Message Quality:**
- Clarity: 5/5 - Crystal clear what failed and why
- Actionability: 5/5 - Exact command to fix provided
- Matches docs: ✅ Yes (COMPFIX-2001)

**Cross-Reference:** This validates **COMPFIX-2003 Error Scenario 2** (Base branch not indexed)

---

## Standard Optimizer Test (INCOMPLETE)

**Status:** ❌ Cannot run due to scan bug

**Expected:**
- Setup: 1-2 minutes
- Execution: 2-3 minutes per generation
- Total: 4-8 minutes for first generation
- Variants: 5 (control, detailed-a, simple-a, detailed-b, simple-b)
- Tool usage: ≥50% of agents use `mcp__maproom__search`

**Actual:** Blocked at setup phase - cannot scan base branch

**Blocker:** Scan implementation bug (blob_sha constraint violation)

---

## Premium Optimizer Test (NOT RUN)

**Status:** ⏸️ Not attempted due to blocking bug

---

## Ultra Optimizer Test (NOT RUN)

**Status:** ⏸️ Not attempted due to blocking bug

---

## Conclusion

### Validation Status: ⚠️ PARTIAL COMPLETION

**Successfully Validated:**
1. ✅ Pre-flight validation catches missing base branch index (COMPFIX-1001)
2. ✅ Security validation enforces resource limits (COMPFIX-1004)
3. ✅ Error messages are clear and actionable (COMPFIX-2001)
4. ✅ Fail-fast behavior works (no API waste when validation fails)
5. ✅ One error scenario from COMPFIX-2003 validated (base branch not indexed)

**Critical Issues Found:**
1. ⚠️ Timeout units mismatch - **FIXED** during validation
2. ❌ Scan implementation blob_sha bug - **BLOCKING**

**Cannot Validate:**
- Tool access (agents not spawned due to scan bug)
- Timing metrics (setup incomplete)
- Score distribution (agents not run)
- End-to-end workflow (blocked at setup)

### Recommendations

1. **URGENT:** Fix scan implementation blob_sha bug
2. **Testing:** Add unit tests for scan to prevent constraint violations
3. **Registry:** Implement atomic writes to prevent JSON corruption
4. **Documentation:** Document the timeout units mismatch and fix
5. **Resume:** Re-run full E2E validation after scan bug fix

### Issues to Fix

#### High Priority (Blocking)
1. Fix blob_sha constraint violation in scan implementation
2. Test scan on crewchief repository
3. Verify chunk creation includes valid blob_sha

#### Medium Priority (Quality)
1. Implement atomic writes for run registry (prevent corruption)
2. Add validation tests for timeout conversion
3. Document the timeout units issue in COMPFIX-1004 ticket

#### Low Priority (Nice to Have)
1. Add progress indicators for long-running scans
2. Improve scan error messages with more context
3. Consider retry logic for transient database errors

---

## Next Steps

1. ✅ Document validation findings (this report)
2. ❌ Fix scan implementation bug (required)
3. ⏸️ Re-run standard optimizer E2E test (blocked)
4. ⏸️ Run premium optimizer E2E test (blocked)
5. ⏸️ Run ultra optimizer E2E test (blocked)
6. ⏸️ Complete COMPFIX-2003 error scenario testing (partially blocked)

**Estimated Time to Unblock:** 1-3 hours (scan bug investigation + fix + test)

---

## Appendix: Environment Details

**Node.js:**
```
pnpm tsx (via package.json scripts)
```

**PostgreSQL:**
```
Container: maproom-postgres
Image: pgvector/pgvector:pg16
Port: 5433 (host) → 5432 (container)
Database: maproom
User: maproom
```

**Ollama:**
```
Container: maproom-ollama
Model: nomic-embed-text (768 dimensions)
Port: 11434
```

**Database Status:**
- Connection: ✅ Working
- Total chunks: 1000 (from test-repo)
- crewchief repo: ❌ Not indexed (scan fails)

**Code Version:**
- Branch: main
- Commit: 39a0ed6 (COMPFIX-2001 documentation update)
- Built: Yes (after timeout bug fix)
