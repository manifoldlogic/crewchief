# Plan: FTS Query Sanitization

## Overview

This is a focused bug fix that can be completed in a single phase. The change refactors the sanitization logic from individual `.replace()` calls to a regex whitelist approach, providing comprehensive coverage for all special characters.

## Phases

### Phase 1: Fix and Test

**Objective:** Refactor sanitization to use regex whitelist for comprehensive special character handling

**Deliverables:**
- Refactored sanitization logic in `/crates/maproom/src/db/sqlite/fts.rs`
- Comprehensive unit test covering dots, slashes, brackets, braces, at-signs, operators
- All existing tests pass
- Performance baseline measurement
- Manual verification with real queries

**Agent Assignments:**
- **rust-engineer**: Refactor to regex whitelist and write comprehensive unit test
- **unit-test-runner**: Execute `cargo test -p crewchief-maproom` to verify all tests pass
- **verify-ticket**: Validate the fix works for example queries (package.json, src/main.rs, array[0], user@email.com)
- **commit-ticket**: Create commit with conventional commit message

**Estimated Time:** 45-60 minutes

**Task Breakdown:**

1. **Refactor sanitization logic** (15 min)
   - Edit `/crates/maproom/src/db/sqlite/fts.rs` lines 49-56
   - Replace individual `.replace()` calls with regex whitelist
   - Add `use once_cell::sync::Lazy;` and `use regex::Regex;`
   - Define `static SPECIAL_CHAR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9_\s]").unwrap());`
   - Replace sanitization chain with `let clean = SPECIAL_CHAR_REGEX.replace_all(t, " ").to_string();`

2. **Write comprehensive unit test** (15 min)
   - Add `test_build_fts_query_comprehensive_sanitization` after line 301
   - Test cases: dots, slashes, brackets, braces, at-signs, backslashes, mixed chars, operators
   - Follow existing test pattern
   - 8+ test cases covering all character categories

3. **Measure performance baseline** (5 min)
   - Note current test execution time for fts tests
   - Document baseline in test output or comment

4. **Run tests** (5 min)
   - Execute `cargo test -p crewchief-maproom fts`
   - Verify new test passes
   - Verify all existing tests pass
   - Compare execution time to baseline (should be <5% difference)

5. **Manual verification** (10 min)
   - Test with CLI: `crewchief-maproom search --query "package.json" --repo crewchief --mode fts`
   - Test: `crewchief-maproom search --query "src/main.rs" --repo crewchief --mode fts`
   - Test: `crewchief-maproom search --query "array[0]" --repo crewchief --mode fts`
   - Test: `crewchief-maproom search --query "user@email.com" --repo crewchief --mode fts`
   - Verify no syntax errors
   - Verify results returned for each

6. **Commit** (5 min)
   - Conventional commit: `fix(maproom): comprehensive FTS5 query sanitization`
   - Body explains regex whitelist approach and all characters handled
   - References this project slug: FTSFIX

### Phase 2: (No additional phases needed)

This project requires only one phase due to its simplicity.

## Dependencies

### Internal Dependencies
- None - self-contained change to one function

### External Dependencies
- None - uses only Rust standard library

### Cross-Phase Dependencies
- N/A - single phase

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaks existing queries | Low | Medium | Comprehensive test suite catches regressions |
| Performance degradation | Very Low | Low | String replace is O(n), negligible overhead |
| Edge cases not covered | Low | Low | Existing filter logic handles all edge cases |
| Deployment issues | Very Low | Low | No migration required, instant rollback available |

### Risk Details

**Risk 1: Breaks existing queries**
- **Why low probability:** Only adds handling for a character that previously caused errors
- **Detection:** Unit tests would fail
- **Recovery:** Rollback to previous binary

**Risk 2: Performance degradation**
- **Why very low probability:** Adding one string replace has negligible cost
- **Detection:** No performance monitoring needed (impact too small to measure)
- **Recovery:** N/A (impact negligible)

**Risk 3: Edge cases not covered**
- **Why low probability:** Existing logic already handles whitespace normalization
- **Detection:** Unit tests cover edge cases (multiple dots, leading dots, etc.)
- **Recovery:** Add additional test cases if discovered

## Success Metrics

### Acceptance Criteria

- [ ] Code change: Refactor to regex whitelist `[^a-zA-Z0-9_\s]` in `build_fts_query()`
- [ ] Test: `test_build_fts_query_comprehensive_sanitization` added and passing
- [ ] Test: All existing tests pass (`cargo test -p crewchief-maproom`)
- [ ] Performance: Baseline measured, no regression >5%
- [ ] Manual: `package.json` query returns results (no syntax error)
- [ ] Manual: `src/main.rs` query returns results (no syntax error)
- [ ] Manual: `array[0]` query returns results (no syntax error)
- [ ] Manual: `user@email.com` query returns results (no syntax error)
- [ ] Commit: Created with conventional commit format

### Verification Commands

```bash
# 1. Performance baseline (before change)
time cargo test -p crewchief-maproom fts

# 2. Unit tests (after change)
cargo test -p crewchief-maproom fts

# 3. Full test suite
cargo test -p crewchief-maproom

# 4. Manual verification (requires indexed repo)
crewchief-maproom search --query "package.json" --repo crewchief --mode fts
crewchief-maproom search --query "src/main.rs" --repo crewchief --mode fts
crewchief-maproom search --query "array[0]" --repo crewchief --mode fts
crewchief-maproom search --query "user@email.com" --repo crewchief --mode fts
crewchief-maproom search --query "template{value}" --repo crewchief --mode fts
crewchief-maproom search --query "path\to\file" --repo crewchief --mode fts

# 5. Verify no syntax errors in output
# Expected: Search results (or empty results)
# Not expected: "fts5: syntax error near '.'" or similar
```

## Rollout Strategy

### Deployment

1. **No special deployment needed**
   - Fix is in Rust binary (crewchief-maproom)
   - Binary is statically linked
   - Next build includes fix automatically

2. **Testing in development**
   - Run tests locally before committing
   - CI will run full test suite on PR

3. **Release process**
   - Fix will be included in next CLI release
   - Release process managed by existing `release-config.json`
   - No special release notes needed (bug fix, not feature)

### Rollback

If issues discovered post-deployment:
1. Revert commit
2. Rebuild binary
3. Redeploy

No data cleanup needed (query-time fix only).

## Timeline

**Total estimated time:** 45-60 minutes

| Phase | Task | Duration | Cumulative |
|-------|------|----------|------------|
| Phase 1 | Refactor sanitization logic | 15 min | 15 min |
| Phase 1 | Write comprehensive unit test | 15 min | 30 min |
| Phase 1 | Measure performance baseline | 5 min | 35 min |
| Phase 1 | Run unit tests | 5 min | 40 min |
| Phase 1 | Manual verification | 10 min | 50 min |
| Phase 1 | Create commit | 5 min | 55 min |

**Contingency buffer:** +15 min for unexpected test failures or regex debugging (total 60-75 min)

## Post-Completion

### Documentation Updates

No documentation updates needed:
- User-facing behavior doesn't change (queries that failed now work)
- No API changes
- No new features

### Knowledge Transfer

- Commit message explains the fix
- This planning document serves as reference
- Pattern documented for future sanitization additions

### Follow-up Work

None required - this fix provides comprehensive coverage for ALL special characters.

### Future Considerations

The regex whitelist approach means no future character-by-character fixes should be needed. If FTS5 rules change (extremely unlikely):
1. Update the regex pattern in `SPECIAL_CHAR_REGEX`
2. Update tests to reflect new rules
3. Verify with manual queries

**No per-character maintenance required going forward.**
