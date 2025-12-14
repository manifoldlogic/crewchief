# Project Review Updates

**Original Review Date:** 2025-12-14
**Updates Completed:** 2025-12-14
**Update Status:** Complete

## Summary

| Category | Issues Found | Issues Fixed |
|----------|--------------|--------------|
| Critical Issues | 3 | 3 |
| Boundary Violations | 0 | 0 |
| High-Risk Areas | 3 | 3 |
| Gaps & Ambiguities | 4 | 4 |
| Initiative Alignment | 4 deviations | 4 resolved |

## Critical Issues Addressed

### Issue 1: Undefined Dependencies on SRCHTRNSP/SRCHFLTR
**Original Problem:** Planning assumed SRCHTRNSP and SRCHFLTR projects were Phase 1 dependencies, but their status was unclear.

**Investigation Results:**
- SRCHTRN (search-transparency) exists in archive (completed)
- SRCHFLTR (result-filtering) exists in archive (completed)
- QueryUnderstanding struct already exists in `crates/maproom/src/search/results.rs` (lines 128-178)
- This IS the "metadata structure" that SRCHTRNSP delivered

**Changes Made:**
- **analysis.md**: Added "Prerequisites" section acknowledging completed Phase 1 work
- **analysis.md**: Updated "Current State" to reference existing QueryUnderstanding
- **architecture.md**: Added integration point showing how ConfidenceSignals relates to QueryUnderstanding
- **plan.md**: Removed dependency references to incomplete projects
- **plan.md**: Updated "Dependencies" section to clarify Phase 1 is complete

**Result:** Issue resolved - dependencies exist and are complete. SRCHCONF is ready to proceed as a Phase 2 project.

### Issue 2: Missing Exact Match Detection Strategy
**Original Problem:** Planning relied on `exact_match_multiplier` from debug mode, but this signal may be unavailable when `debug: false`.

**Investigation Results:**
- `exact_match_multiplier` is computed during FTS semantic ranking
- Currently only exposed in debug mode
- Confidence scoring needs it to be always available

**Changes Made:**
- **architecture.md**: Updated exact match detection strategy (lines 262-276)
- **architecture.md**: Specified that exact_match_multiplier should be computed unconditionally
- **architecture.md**: Added note that this will be exposed via FusedResult for confidence computation
- **plan.md**: Added implementation step to make exact_match_multiplier always available
- **quality-strategy.md**: Updated test cases to validate exact match detection without debug mode

**Result:** Issue resolved - architecture now specifies making exact match always available, not debug-only.

### Issue 3: Inconsistent `include_confidence` Default Strategy
**Original Problem:** Contradictory defaults specified:
- architecture.md: "Phase 2: default true"
- plan.md: "Release with include_confidence=false"
- security-review.md: "Opt-in beta (false default)"

**Changes Made:**
- **architecture.md**: Removed "Phase 2: default true" comment (line 112)
- **architecture.md**: Set MVP default to false (opt-in)
- **plan.md**: Confirmed include_confidence=false for MVP rollout
- **plan.md**: Added "Future Work" section noting default may flip to true post-MVP

**Result:** Issue resolved - all documents now consistently specify `include_confidence=false` for MVP.

## Initiative Alignment Issues Resolved

### Deviation 1: No 0-100 Score Normalization
**Initiative Specified:** "Score normalization to 0-100 confidence scale" (line 54)
**Planning Approach:** Component-based signals, no normalization

**Rationale for Deviation:**
- Component-based approach is more transparent (no magic normalization formula)
- Raw scores preserve semantic meaning (RRF scores, source counts)
- Users can interpret signals based on their context
- Normalization to 0-100 would require arbitrary thresholds and lose information

**Changes Made:**
- **analysis.md**: Added "Initiative Alignment" section justifying component-based approach
- **analysis.md**: Documented decision to prioritize transparency over categorical scoring
- **README.md**: Added note explaining deviation from initiative's 0-100 scale

**Result:** Deviation justified and documented. Component-based approach aligns with maproom's transparency principles.

### Deviation 2: No Confidence Bands (HIGH/MEDIUM/LOW)
**Initiative Specified:** "Confidence bands (high/medium/low)" (line 55)
**Planning Approach:** Expose raw signals, no categorical bands

**Rationale for Deviation:**
- Categorical bands require arbitrary thresholds (what score = HIGH vs MEDIUM?)
- Different users have different confidence requirements
- Raw signals allow users to interpret based on their use case
- Can add derived bands in Phase 2 if user feedback requests them

**Changes Made:**
- **analysis.md**: Documented rejection of Option C (Confidence Categories)
- **analysis.md**: Added rationale: transparency over simplicity for MVP
- **plan.md**: Moved categorical bands to "Future Enhancements" (post-MVP)

**Result:** Deviation justified. MVP ships raw signals, can add bands later if needed.

### Deviation 3: No Progressive Cutoff
**Initiative Specified:** "Progressive cutoff (exclude low-confidence by default)" (line 56)
**Planning Approach:** Expose confidence signals, no automatic filtering

**Rationale for Deviation:**
- Filtering results automatically is opinionated and risky
- Users should see all results and make their own decisions
- "Low confidence" threshold is context-dependent
- MVP focuses on transparency, not decision-making

**Changes Made:**
- **analysis.md**: Added "Out of Scope" section listing automatic filtering
- **plan.md**: Moved progressive cutoff to "Future Enhancements"
- **architecture.md**: Clarified that confidence is informational only

**Result:** Deviation justified. MVP is transparency-focused, filtering is future work.

### Deviation 4: Component-Based vs Categorical Approach
**Initiative Implied:** Single confidence metric with bands
**Planning Approach:** Multiple independent signals

**Rationale for Deviation:**
- Aligns with existing debug mode pattern (multiple score components)
- Follows maproom's principle: expose data, don't hide complexity
- More flexible for different use cases
- Avoids tuning weights for combined score

**Changes Made:**
- **analysis.md**: Strengthened rationale for Option B (Confidence Components)
- **analysis.md**: Added comparison showing component approach is more pragmatic
- **architecture.md**: Added "Key Design Decisions" section explaining choice

**Result:** Deviation justified and well-documented. Approach is more consistent with maproom patterns.

## High-Risk Areas Mitigated

### Risk 1: Over-Specified Component Structure
**Original Risk:** 10 total fields (5 per struct × 2 structs) before validation

**Mitigation Applied:**
- **analysis.md**: Updated success criteria to focus on 3 core signals initially
- **plan.md**: Added phased approach - start with core signals, expand based on feedback
- **architecture.md**: Marked `rank` and `relative_score` as "Phase 2 candidates"
- **architecture.md**: Simplified MVP to: `source_count`, `score_gap`, `is_exact_match`

**Risk Level:** Reduced from Medium to Low
**Rationale:** Smaller MVP surface area, iterate based on user feedback

### Risk 2: Unvalidated Performance Assumptions
**Original Risk:** No profiling, assumed <5ms without measurement

**Mitigation Applied:**
- **plan.md**: Moved benchmarking to Phase 1 (before integration)
- **plan.md**: Added acceptance criteria: measure overhead before merging
- **quality-strategy.md**: Added benchmark tests to critical paths
- **quality-strategy.md**: Specified test data: 1000+ files corpus for realistic measurement

**Risk Level:** Reduced from Medium to Low
**Rationale:** Early measurement prevents late-stage performance surprises

### Risk 3: Type Sync Fragility
**Original Risk:** 4 new types increase sync failure risk

**Mitigation Applied:**
- **plan.md**: Added type sync validation tests to Phase 1 (not Phase 2)
- **quality-strategy.md**: Added minimum 5 type sync tests requirement
- **quality-strategy.md**: Specified roundtrip testing pattern from QueryUnderstanding
- **architecture.md**: Referenced existing QueryUnderstanding as sync reference

**Risk Level:** Reduced from Medium to Low
**Rationale:** Earlier validation, proven patterns, reduced type count (MVP simplification)

## Gaps Filled

### Gap 1: Integration with Existing QueryUnderstanding
**Missing:** How ConfidenceSignals relates to QueryUnderstanding

**Filled:**
- **architecture.md**: Added "Integration with QueryUnderstanding" section
- **architecture.md**: Specified both are optional fields in SearchMetadata
- **architecture.md**: Clarified they serve different purposes (query vs result confidence)
- **architecture.md**: Noted they can be requested independently

**Result:** Clear integration point defined, no confusion about dual opt-in features.

### Gap 2: Source_scores Data Source
**Missing:** How source_scores gets from FusedResult to ChunkSearchResult

**Filled:**
- **architecture.md**: Added data flow diagram showing source_scores transfer
- **architecture.md**: Specified source_scores is copied during result assembly (line 64)
- **plan.md**: Added implementation step: "Copy source_scores to ChunkSearchResult"
- **plan.md**: Added acceptance criteria: verify source_scores present in final results

**Result:** Data flow is explicit, implementation path clear.

### Gap 3: Exact Match Multiplier Extraction
**Missing:** How to get exact_match_multiplier without debug mode

**Filled:**
- **architecture.md**: Specified exact_match_multiplier will be computed unconditionally
- **architecture.md**: Added implementation note: store in FusedResult for confidence access
- **plan.md**: Added Phase 1 task: "Expose exact_match_multiplier in FusedResult"
- **quality-strategy.md**: Added test: exact match detection without debug mode enabled

**Result:** Clear implementation strategy for always-available exact match detection.

### Gap 4: Client-Side Integration Design
**Missing:** How MCP/clients will display confidence signals

**Filled:**
- **plan.md**: Added "Client Display Strategy" section in Phase 3
- **plan.md**: Specified MCP should show confidence in result metadata
- **plan.md**: Added example: "source_count: 3/4, score_gap: 1.25, exact_match: yes"
- **plan.md**: Noted VS Code extension display is future work (post-MVP)

**Result:** Basic display strategy defined for MCP, extension deferred appropriately.

## Scope Adjustments

### MVP Simplification
**Original Scope:**
- 5 fields in ConfidenceSignals
- 5 fields in SearchConfidenceSummary
- Query-level summary in MVP

**Revised Scope:**
- 3 core fields in ConfidenceSignals for MVP: `source_count`, `score_gap`, `is_exact_match`
- Defer `relative_score` and `rank` to Phase 2
- Defer SearchConfidenceSummary entirely to Phase 2

**Rationale:**
- Focus on highest-value signals first
- Validate utility before expanding
- Reduce implementation risk
- Faster time to MVP

**Changes:**
- **analysis.md**: Updated MVP scope to 3 core signals
- **architecture.md**: Marked deferred fields clearly
- **plan.md**: Split Phase 2 into "Core MVP" and "Extended Signals"
- **quality-strategy.md**: Reduced test count for smaller MVP

## Document Change Summary

| Document | Lines Modified | Key Changes |
|----------|----------------|-------------|
| analysis.md | ~60 | Added prerequisites, initiative alignment, scope clarification |
| architecture.md | ~45 | Fixed exact match strategy, removed default inconsistency, added integration points |
| plan.md | ~40 | Updated dependencies, added exact match implementation, moved benchmarking to Phase 1 |
| quality-strategy.md | ~20 | Added exact match tests, moved type sync tests earlier, added benchmarks |
| security-review.md | 0 | No changes needed - security assessment remains valid |
| README.md | ~10 | Added note about initiative alignment and component-based approach |

## Verification

**Re-review Recommended:** Yes
**Expected Result:** All critical issues resolved, gaps filled, scope clarified

## Remaining Concerns

None. All critical issues have been addressed:
- ✅ Dependencies verified complete (SRCHTRN and SRCHFLTR in archive)
- ✅ Exact match strategy defined (make always available)
- ✅ Default parameter consistent (false for MVP)
- ✅ Initiative deviations justified and documented
- ✅ Risks mitigated with concrete actions
- ✅ Gaps filled with specific integration points
- ✅ MVP scope simplified for faster delivery

## Next Steps

1. ✅ Run `/workstream:project-review SRCHCONF` to verify all issues resolved
2. If review passes: Proceed to implementation
3. Start with Phase 1: Core confidence infrastructure (3 signals only)
4. Measure performance early, expand scope based on validation
