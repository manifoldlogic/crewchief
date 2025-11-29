# Project Review: GITPOLL - Git Polling File Watcher

**Review Date:** 2024-11-29
**Project Status:** Ready
**Overall Risk:** Low

## Executive Summary

This project is well-defined with a clear, focused scope: replace the `notify`-based file watcher with git status polling to eliminate "too many open files" errors. The planning documents demonstrate strong analysis of the problem, careful consideration of alternatives, and a pragmatic solution that aligns with Maproom's core purpose (indexing git repositories).

The architecture is sound, preserving existing interfaces (`FileEvent`, `IndexingEvent`) for seamless integration. The phased approach (implement → integrate → cleanup) minimizes risk. The project correctly identifies existing components to reuse (`IgnorePatternMatcher`, `normalize_to_relpath`, `FileEvent` types) and avoids unnecessary duplication.

**Key Strengths:**
- Problem is well-understood with clear root cause analysis
- Solution is appropriately scoped (MVP mindset, not overengineered)
- Interfaces preserved for drop-in replacement
- Existing path validation utilities will be reused
- Testing strategy is pragmatic and comprehensive

**Minor Concerns:**
- Architecture mentions `CancellationToken` but this isn't in current dependencies
- State diff logic for detecting deletions needs clarification
- Non-git directory fallback not fully specified

**Recommendation:** Proceed with ticket creation after addressing minor clarifications.

## Critical Issues (Blockers)

**None identified.** The project is well-planned and ready for execution.

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds

**None identified.** The project correctly plans to:
- Reuse existing `FileEvent` and `IndexingEvent` types
- Reuse `IgnorePatternMatcher` for non-git filtering scenarios
- Reuse `normalize_to_relpath` for path validation

### Missed Reuse Opportunities

| Available Component | Purpose | Recommendation |
|---------------------|---------|----------------|
| `path_utils::normalize_to_relpath` | Path validation with `..` rejection | Already identified in security-review.md |
| `IgnorePatternMatcher` | Fallback filtering | Already identified in analysis.md |
| Existing debouncing task in `watcher.rs` | Event debouncing | May not be needed with polling, but worth reviewing |

### Pattern Consistency

**Strong alignment with existing patterns:**
- Uses same async patterns (tokio, mpsc channels)
- Follows existing error handling conventions (anyhow, thiserror)
- Maintains existing public interfaces

### Boundary Violations

**None identified.** The project respects module boundaries:
- New components (`git_poller.rs`, `git_state.rs`) in correct location
- Modifies only the watcher layer, not downstream consumers
- Preserves `FileEvent` interface for change detector and processor

## High-Risk Areas (Warnings)

### Risk 1: Git Not Available

**Risk Level:** Medium
**Category:** Execution
**Description:** Git binary may not be available on some systems (e.g., minimal Docker containers, restricted environments).
**Probability:** Low (Maproom is designed for git repos)
**Impact:** Medium (watch command completely broken)
**Mitigation:** Already addressed in plan - check for git on startup with clear error message.

### Risk 2: State Diff Logic for Deletions

**Risk Level:** Medium
**Category:** Technical
**Description:** The `GitState::diff()` method needs to handle the transition from "file exists in old state" to "file absent in new state" correctly. Git status only shows modified/untracked files, not clean files.
**Probability:** Medium
**Impact:** Medium (deletions might not be detected)
**Mitigation:** Need to clarify in architecture:
- Track ALL files seen (clean + modified) in previous state
- Detect deletion when file disappears from git status AND filesystem

### Risk 3: Non-Git Directory Handling

**Risk Level:** Low
**Category:** Technical
**Description:** What happens if user runs watch on a non-git directory? Currently says "error gracefully" but specific behavior unspecified.
**Probability:** Low
**Impact:** Low (clear error message is fine)
**Mitigation:** Specify exact error behavior: return `GitPollerError::NotGitRepository` on creation, don't start extension features.

### Risk 4: CancellationToken Dependency

**Risk Level:** Low
**Category:** Technical
**Description:** Architecture mentions `tokio_util::sync::CancellationToken` but this isn't in Cargo.toml dependencies.
**Probability:** High (missing dependency)
**Impact:** Low (easy to add or use alternative)
**Mitigation:** Either add `tokio-util` dependency or use existing `tokio::sync::watch` channel for shutdown signaling.

## Gaps & Ambiguities

### Requirements Gaps

1. **Clean file tracking**: Git status only shows dirty files. How do we track clean files to detect when they're deleted?
   - **Impact:** May miss deletion events
   - **Suggestion:** Initial scan captures all tracked files; deletions detected when file disappears from subsequent status AND doesn't exist on filesystem

2. **Submodule handling**: Analysis mentions submodules but doesn't specify behavior.
   - **Impact:** Submodules may not be indexed correctly
   - **Suggestion:** For MVP, treat submodules as regular directories. Document as known limitation.

### Technical Gaps

1. **Debouncing interaction**: Current watcher has debouncing. Is this still needed with 3-second polling?
   - **Impact:** Minor (probably not needed)
   - **Suggestion:** Remove debouncing for git poller since polling is inherently debounced

2. **Initial state population**: How is `previous_state` initialized on first poll?
   - **Impact:** Minor
   - **Suggestion:** First poll: empty previous state, emit Modified events for all dirty files. Document that first poll after restart may trigger re-indexing.

### Process Gaps

None identified. The plan has clear phases, deliverables, and acceptance criteria.

## Scope & Feasibility Concerns

### Scope Creep Indicators

1. **"Future Enhancements" section** in architecture.md mentions:
   - Auto-detect based on directory count
   - Watchman integration
   - Hybrid mode with editor integration

   **Assessment:** These are correctly marked as future work and NOT in scope for this project. Good discipline.

2. **Configurable git path** in security-review.md "Nice to Have"

   **Assessment:** Correctly deferred. MVP should use system git.

### Feasibility Assessment

**Highly feasible.** This is straightforward systems programming:
- Parsing git status output is well-documented
- State comparison is simple HashMap operations
- Integration points are clear
- Estimated ~500 lines of new code is reasonable

## Alignment Assessment

### MVP Discipline
**Rating:** Strong

- Solution does exactly what's needed (eliminate FD errors)
- No unnecessary features
- Phases deliver incremental value
- Future enhancements correctly deferred

### Pragmatism Score
**Rating:** Strong

- Git polling is the simplest solution that works
- Polling latency (2-5s) correctly deemed acceptable
- No overengineering for hypothetical future needs
- Testing strategy focused on critical paths

### Agent Compatibility
**Rating:** Strong

- Tasks are appropriately sized (2-8 hour chunks)
- Single agent (rust-indexer-engineer) owns entire project
- Clear acceptance criteria for each phase
- No cross-agent dependencies

### Codebase Integration
**Rating:** Strong

- Correctly identifies files to modify vs. replace
- Preserves existing interfaces
- Reuses existing utilities (path validation)
- Follows established patterns

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [x] Dependencies on existing systems documented

### Technical
- [x] Technology choices are appropriate
- [x] Dependencies are identified and available
- [x] Integration points are well-defined
- [x] Performance requirements are clear
- [x] Error handling is specified
- [x] Existing tools/libraries identified for reuse
- [x] No unnecessary duplication of functionality

### Process
- [x] Agent assignments are appropriate
- [x] Task boundaries are clear
- [x] Verification criteria are explicit
- [x] Handoffs are defined (single agent)
- [ ] Rollback plan exists (implicit - keep notify as fallback in Phase 2)
- [x] Integration with existing workflows considered

### Integration & Reuse
- [x] Existing solutions evaluated before building new
- [x] Current patterns and conventions followed
- [x] Reusable components identified
- [x] Integration points with existing systems mapped
- [x] No reinvention of available functionality
- [x] Component boundaries respected
- [x] Public interfaces used (not internals)
- [x] Appropriate coupling levels maintained

### Risk
- [x] Major risks are identified
- [x] Mitigation strategies exist
- [ ] Dependencies have fallbacks (git availability)
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Clarify deletion detection**: Add a note to architecture.md explaining how deletions will be detected when git status doesn't show clean files.

2. **Specify shutdown mechanism**: Either add `tokio-util` to dependencies for `CancellationToken`, or document using `tokio::sync::watch` channel instead.

3. **Document initial state behavior**: Specify what happens on first poll (empty previous state → all dirty files emit events).

### Phase 1 Adjustments

- Consider removing debouncing logic since polling is inherently debounced
- Add test case for deletion detection (tracked file removed from filesystem)

### Risk Mitigations

- **Git availability**: Add runtime check with clear error message (already planned)
- **Fallback strategy**: Keep notify watcher code until Phase 3 confirms git polling works in production

### Documentation Updates

- **architecture.md**: Add subsection on deletion detection strategy
- **architecture.md**: Clarify shutdown mechanism (CancellationToken or alternative)
- **plan.md**: Add note about keeping notify as fallback until Phase 3

## Review Conclusion

### Readiness Assessment

**Can this project succeed as currently defined?** Yes, with minor clarifications.

**Primary concerns:**
1. Deletion detection logic needs explicit specification
2. Missing `tokio-util` dependency for `CancellationToken`
3. Initial state behavior should be documented

These are all easily addressed and don't block ticket creation.

### Recommended Path Forward

**PROCEED:** Project is well-defined and ready for execution with minor clarifications.

The planning documents demonstrate thorough analysis, appropriate technical decisions, and proper scope control. The solution is aligned with Maproom's purpose and the implementation is straightforward.

### Success Probability

Given current state: **85%**
After recommended changes: **95%**

### Final Notes

This is a well-planned project that solves a real problem (FD exhaustion) with an elegant solution (leverage git, which Maproom already requires). The team correctly identified that the slight latency trade-off is acceptable for the massive gain in reliability.

The phased approach with notify fallback in Phase 2 is particularly good - it allows real-world validation before committing to the new approach.

Strong recommendation to proceed.
