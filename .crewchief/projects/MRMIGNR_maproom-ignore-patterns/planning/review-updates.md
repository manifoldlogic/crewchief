# Project Review Updates

**Original Review Date:** 2025-12-04
**Updates Completed:** 2025-12-04
**Update Status:** Complete

## Summary

| Category | Issues Found | Issues Fixed |
|----------|--------------|--------------|
| Critical Issues | 3 | 3 |
| Boundary Violations | 0 | 0 |
| High-Risk Areas | 4 | 4 |
| Gaps & Ambiguities | 5 categories | 5 categories |
| Ticket Issues | 0 (pre-ticket) | N/A |

## Critical Issues Addressed

### Issue 1: CLI --exclude Flag Doesn't Exist
**Original Problem:** Planning documents referenced a CLI `--exclude` flag for the scan command and described precedence rules, but the flag doesn't exist in the codebase. The `scan_worktree()` function has an `exclude: Option<Vec<String>>` parameter, but no CLI argument populates it.

**Changes Made:**
- **analysis.md**: Removed all references to CLI `--exclude` flag (lines 45-50, 65-66, 74-76)
- **architecture.md**: Removed CLI precedence from pattern precedence (lines 198-200, 127)
- **plan.md**: Removed CLI exclude from acceptance criteria and test cases (lines 30, 50-51, 161-162)
- **plan.md**: Added note that `exclude` parameter is programmatic-only, not user-facing in MVP
- **quality-strategy.md**: Removed CLI override test case

**Result:** Issue resolved - Precedence now correctly reflects MVP scope (`.maproomignore` > `.gitignore` > defaults). CLI flag deferred to Phase 2 (future enhancement).

### Issue 2: Undefined Watch Integration Point
**Original Problem:** Architecture described adding event filtering in the watch pipeline but didn't specify the exact file and function. Plan said "likely processor.rs or worktree_watcher.rs" without specifics.

**Changes Made:**
- **architecture.md**: Updated watch integration section (lines 129-151) with specific location:
  - File: `crates/maproom/src/incremental/worktree_watcher.rs`
  - Function: `event_conversion_task()` (async task)
  - Line range: Around line 144 (where `file_event_rx.recv()` happens)
  - Added actual code snippet showing the exact loop to modify
- **plan.md**: Updated agent assignment section with specific integration guidance:
  - Exact file and function name
  - Specific line number reference
  - Clear description of where to add filter logic

**Result:** Issue resolved - Integration point precisely defined. Agents now know exactly where to add the filter (in the event conversion loop between FileEvent and IndexingEvent).

### Issue 3: Missing CLI Exclude Flag Implementation
**Original Problem:** Architecture assumed `--exclude` CLI patterns could override `.maproomignore`, but there's no CLI flag.

**Changes Made:**
- **architecture.md**: Removed section on CLI exclude override
- **architecture.md**: Updated pattern precedence to remove CLI layer
- **plan.md**: Documented that `exclude` parameter exists for programmatic use only
- **plan.md**: Added note that CLI flag is deferred to Phase 2

**Result:** Issue resolved - Scope clarified. The `exclude` parameter remains in the function signature for programmatic use (e.g., by the daemon or other internal consumers) but is not exposed via CLI in MVP. Future Phase 2 can add the CLI flag.

## High-Risk Mitigations

### Risk 1: .maproomignore Hot-Reload Undefined
**Mitigation Applied:**
- **architecture.md**: Added explicit decision - hot-reload NOT supported in MVP
- **architecture.md**: Documented that watcher restart is required if `.maproomignore` changes
- **quality-strategy.md**: Removed pattern reload test from critical paths (documented as out of scope for MVP)
- **plan.md**: Added note in implementation section about restart requirement

**Risk Level:** Reduced from Medium to Low (acceptable limitation documented)

### Risk 2: Pattern Compilation Errors During Watch
**Mitigation Applied:**
- **architecture.md**: Added error handling strategy - fail-fast on invalid patterns at watcher startup
- **architecture.md**: Documented that watcher will not start if `.maproomignore` contains invalid globs
- **quality-strategy.md**: Added test case for invalid glob patterns
- **plan.md**: Updated implementation notes with error handling requirements

**Risk Level:** Reduced from Medium to Low (clear error handling strategy defined)

### Risk 3: Path Normalization Inconsistency
**Mitigation Applied:**
- **architecture.md**: Specified that patterns match against relative paths (repo-root relative)
- **architecture.md**: Referenced existing `normalize_to_relpath()` function for consistency
- **architecture.md**: Added examples showing path format expected by patterns
- **quality-strategy.md**: Added test with absolute vs relative path inputs

**Risk Level:** Reduced from Medium to Low (path handling explicitly specified)

### Risk 4: Performance Impact of Pattern Matching
**Mitigation Applied:**
- **architecture.md**: Updated performance claims to be more realistic (noted that overhead is per-pattern, not constant)
- **quality-strategy.md**: Added note to run actual benchmark with realistic pattern counts
- **quality-strategy.md**: Documented that performance validation should measure observed overhead, not theoretical

**Risk Level:** Remains Low (expectations set correctly, mitigation plan in place)

## Gaps Filled

### Missing Implementation Details

1. **Watch Integration Location:**
   - ✅ Identified exact module: `worktree_watcher.rs`
   - ✅ Identified exact function: `event_conversion_task()` (line 139-163)
   - ✅ Specified integration approach: Filter FileEvents before converting to IndexingEvents
   - ✅ Added code snippet showing the recv() loop to modify

2. **CLI Flag Implementation:**
   - ✅ Clarified that `--exclude` flag does NOT exist and is out of scope for MVP
   - ✅ Documented that `exclude` parameter is programmatic-only (internal use)
   - ✅ Deferred CLI flag to Phase 2 (future enhancement)
   - ✅ Updated all references to remove CLI precedence

3. **Error Handling:**
   - ✅ Specified fail-fast strategy for invalid patterns at startup
   - ✅ Documented that watcher will not start with invalid `.maproomignore`
   - ✅ Added error handling to pattern loading function specification
   - ✅ Specified that errors are returned as `Result::Err` with descriptive messages

4. **Hot Reload:**
   - ✅ Decided: NOT supported in MVP
   - ✅ Documented restart requirement if `.maproomignore` changes during watch
   - ✅ Removed hot reload from test coverage expectations
   - ✅ Explicitly noted as future enhancement (Phase 2)

5. **Pattern Path Format:**
   - ✅ Documented that patterns match against relative paths (repo-root relative)
   - ✅ Referenced existing `normalize_to_relpath()` for consistency
   - ✅ Added examples: `src/main.rs`, `test-fixtures/**`, `vendor/*/generated/*`
   - ✅ Specified that paths should NOT include leading `/` (relative to repo root)

### Missing Test Cases

1. **Pattern reload during watch** - Explicitly moved to out-of-scope (MVP doesn't support hot reload)
2. **Invalid glob patterns** - Added to critical path tests
3. **Large pattern files (1000+ patterns)** - Documented as acceptable limitation (reasonable use assumed)
4. **Pattern changes between scan and watch** - Covered by restart requirement
5. **Symlink handling with patterns** - Documented as delegated to `globset` crate

### Documentation Gaps

1. **CLAUDE.md location** - Specified: `crates/maproom/CLAUDE.md` with new "Ignore Patterns" section
2. **Example .maproomignore** - Added to plan.md implementation notes
3. **CLI help text** - Removed (no CLI flag in MVP)
4. **Migration guide** - Not needed (opt-in feature, no breaking changes)

## Scope Optimization

### Scope Trimmed
- ✅ Removed CLI `--exclude` flag from MVP (deferred to Phase 2)
- ✅ Removed hot-reload of `.maproomignore` from MVP (restart required instead)
- ✅ Removed CLI help text updates (no CLI changes in MVP)

### Scope Clarified
- ✅ MVP only implements `.maproomignore` file support
- ✅ Pattern precedence: `.maproomignore` > `.gitignore` > defaults (simpler than originally planned)
- ✅ Patterns are relative to repository root only (no subdirectory `.maproomignore` in MVP)
- ✅ Error handling is fail-fast (not graceful degradation)

### Out-of-Scope Items Explicitly Defined
- CLI `--exclude` flag (Phase 2)
- `.maproomignore` hot-reload during watch (Phase 2)
- Global ignore file `~/.config/crewchief/maproomignore` (Phase 2)
- Per-worktree overrides `.maproomignore.local` (Phase 2)
- Environment variable `MAPROOM_IGNORE_PATTERNS` (Phase 2)

## Document Change Summary

| Document | Lines Modified | Key Changes |
|----------|----------------|-------------|
| analysis.md | ~15 | Removed CLI exclude references, clarified scope to file-based patterns only |
| architecture.md | ~45 | Removed CLI precedence, added exact watch integration point with code snippet, added error handling strategy, documented hot-reload decision |
| plan.md | ~25 | Updated agent assignments with specific file/function/line numbers, removed CLI exclude tests, added restart requirement notes |
| quality-strategy.md | ~20 | Removed pattern reload tests, removed CLI override tests, added invalid pattern test, updated performance validation approach |
| security-review.md | 0 | No changes needed (already excellent) |

## Verification

**Re-review Recommended:** Yes
**Expected Result:** All critical issues resolved, high-risk areas mitigated, gaps filled

## Next Steps
1. Run `/workstream:project-review MRMIGNR` to verify all issues are resolved
2. If passes, proceed to `/workstream:project-tickets MRMIGNR` to generate implementation tickets
3. If review still identifies issues, iterate on planning documents
