# Project Review: OLLDET - Ollama Auto-Detection Fallback Chain

**Review Date:** 2025-11-28
**Project Status:** Ready
**Overall Risk:** Low

## Executive Summary

This is a well-scoped, focused project that addresses a real pain point: Ollama auto-detection failing in containerized environments. The planning documents are thorough, specific, and appropriately sized for the problem.

The project correctly identifies the root cause (hardcoded `localhost:11434` in `is_ollama_available()`), proposes a clean solution (fallback chain with `detect_ollama_endpoint()`), and maintains backward compatibility. The single-file change scope is appropriate for what is essentially a bug fix with improved behavior.

No critical issues were identified. The project is ready for ticket creation and execution.

## Critical Issues (Blockers)

None identified.

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds

None identified. The project correctly:
- Uses existing `reqwest` for HTTP requests
- Uses existing `tracing` for logging
- Uses existing `std::env` for environment variables
- Builds on existing test patterns in `factory.rs`

### Boundary Violations

None identified. The change is entirely internal to `crates/maproom/src/embedding/factory.rs` and maintains proper encapsulation.

### Missed Reuse Opportunities

**Available Component:** `wiremock` (already in dev-dependencies)
**Could Solve:** Mock server testing for fallback chain
**Integration Method:** Dev-dependency import
**Integration Effort:** Low
**Recommendation:** Quality strategy already mentions using wiremock - this is correct approach

### Pattern Violations

None identified. The proposed solution follows existing patterns:
- Uses same timeout strategy (2s) as current implementation
- Uses same logging patterns (debug for attempts, info for success)
- Uses same error handling patterns (returning Result/Option)
- Uses same env var conventions (MAPROOM_EMBEDDING_* prefix)

### Inappropriate Coupling

None identified. The solution maintains appropriate coupling:
- `detect_ollama_endpoint()` is a private helper function
- `create_provider_from_env()` remains the public interface
- Environment variable is the configuration boundary

## High-Risk Areas (Warnings)

### Risk 1: Test Timeout Behavior

**Risk Level:** Medium
**Category:** Execution
**Description:** The existing `test_ollama_detection_timeout` test may need adjustment. Currently it tests that detection completes within 3 seconds. With fallback chain trying 3 endpoints, worst case is 6 seconds.
**Probability:** Medium
**Impact:** Low (test failure, not runtime issue)
**Mitigation:** Update test to allow 7 seconds (3 endpoints × 2s timeout + 1s margin), or mock the endpoints to avoid real timeouts in tests.

### Risk 2: Linux Docker `host.docker.internal`

**Risk Level:** Low
**Category:** Technical
**Description:** `host.docker.internal` is not automatically available on Linux Docker (only Docker Desktop on macOS/Windows). Linux requires `--add-host host.docker.internal:host-gateway` flag.
**Probability:** Low (affects only Linux Docker users, and it's a fallback)
**Impact:** Low (falls through to error, same as before)
**Mitigation:** Already documented in security-review.md. Consider adding to user-facing documentation.

## Gaps & Ambiguities

### Requirements Gaps

None significant. Minor clarification:
- The `extract_base_url()` function should handle edge cases like trailing slashes (`http://host:11434/api/embed/`). The current spec only handles exact matches.
- **Suggested clarification:** Add test case for trailing slash handling.

### Technical Gaps

None significant. The architecture provides concrete implementation details.

### Process Gaps

**OLLDET-1002 (Manual Verification):**
- This is listed as a "ticket" but is really a verification checklist
- It has no code deliverables, only manual testing
- **Suggestion:** Merge this into OLLDET-1001's acceptance criteria, or keep as a verification-only ticket but clarify it requires human execution, not agent execution.

## Scope & Feasibility Concerns

### Scope Creep Indicators

None identified. The scope is explicitly bounded:
- Out of scope: parallel detection, Kubernetes discovery, OLLAMA_URL support, caching
- These are reasonable deferrals for MVP

### Feasibility Challenges

None identified. The implementation is straightforward:
- Single file change
- Uses existing dependencies
- Clear test strategy
- Estimated 1 hour of work

## Alignment Assessment

### MVP Discipline
**Rating:** Strong
- Focused on solving one specific problem
- No unnecessary features or abstractions
- Clear value proposition (container support without config)
- Backward compatible

### Pragmatism Score
**Rating:** Strong
- Simple sequential fallback (not over-engineered parallel)
- Appropriate timeout strategy
- Minimal test surface (pure function + integration)
- Uses existing patterns

### Agent Compatibility
**Rating:** Strong
- Single file change is well-scoped
- Clear acceptance criteria
- Existing tests provide safety net
- rust-indexer-engineer is appropriate agent

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
- [x] Dependencies are identified and available (wiremock, reqwest, etc.)
- [x] Integration points are well-defined
- [x] Performance requirements are clear (2s per endpoint, 6s max)
- [x] Error handling is specified
- [x] Existing tools/libraries identified for reuse
- [x] No unnecessary duplication of functionality

### Process
- [x] Agent assignments are appropriate (rust-indexer-engineer)
- [x] Task boundaries are clear
- [x] Verification criteria are explicit
- [ ] Handoffs are defined (minor: OLLDET-1002 handoff unclear)
- [x] Rollback plan exists (revert single file)
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

### Tickets (not yet created)
- [x] Can create specific tickets from plan
- [x] Dependencies can be properly sequenced
- [x] Scope per ticket is appropriate (2-8 hours)
- [x] Acceptance criteria are measurable

### Risk
- [x] Major risks are identified
- [x] Mitigation strategies exist
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Clarify OLLDET-1002:** Decide if this is a code ticket (automated) or verification checklist (manual). If manual, note that it won't follow standard agent workflow.

2. **Add trailing slash test case:** Ensure `extract_base_url()` handles `http://host:11434/api/embed/` (with trailing slash).

3. **Update timeout test expectation:** Note in plan that `test_ollama_detection_timeout` may need adjustment for 3-endpoint fallback.

### Phase 1 Adjustments

None required. The plan is well-structured.

### Risk Mitigations

- Consider adding a debug log at start of `detect_ollama_endpoint()` listing all endpoints that will be tried, so users can see the fallback chain in action.

### Documentation Updates

- **plan.md:** Note that OLLDET-1002 may require manual execution
- **quality-strategy.md:** Add trailing slash test case

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes

**Primary concerns:**
1. OLLDET-1002 is more of a verification checklist than a code ticket
2. Minor edge case (trailing slash) not explicitly tested
3. Existing timeout test may need adjustment

These are minor refinements, not blockers.

### Recommended Path Forward

**PROCEED:** Project is well-defined and ready for execution with minor adjustments.

The planning documents are thorough, the scope is appropriate, and the technical approach is sound. This is a good example of a focused bug fix project.

### Success Probability
Given current state: 95%
After recommended changes: 98%

### Final Notes

This project exemplifies good MVP discipline:
- One problem, one solution
- Backward compatible
- Minimal scope
- Clear value

The user's original request was simply "fix the Ollama detection to check multiple endpoints." This project delivers exactly that, no more, no less. Well planned.
