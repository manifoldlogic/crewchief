# Project Review: Semantic Entry Point Ranking (SEMRANK)

**Review Date:** 2025-11-19 (Post-Update Review)
**Project Status:** Ready for Execution
**Overall Risk:** Low
**Success Probability:** 85%

## Executive Summary

The SEMRANK project is **well-planned and ready for execution**. The critical blocker identified in the initial review (missing TypeScript MCP search tool) has been **properly addressed** through the addition of Phase 0, which creates the missing tool before implementing semantic enhancements.

All planning documents are comprehensive, internally consistent, and aligned with MVP principles. The project demonstrates excellent risk management, pragmatic architecture, and appropriate testing strategy.

**Key Strengths:**
- ✅ Critical blocker resolved with Phase 0 (creates search tool prerequisite)
- ✅ Problem is real and well-documented (tests rank above implementations)
- ✅ Solution is pragmatic (SQL multipliers, no schema changes, easy rollback)
- ✅ Architecture builds on existing Rust FTS implementation
- ✅ Comprehensive testing without ceremony
- ✅ Clear success metrics (>90% top-1 accuracy, <10% latency increase)
- ✅ Low deployment risk (stateless, easily reversible)

**Minor Observations:**
- Search tool integration pattern needs minor clarification (inline vs separate file)
- Normalization complexity may require 1.5 days instead of 1 day
- Test corpus could use maproom's own codebase (meta-testing)

**Overall Assessment:** This is an exemplary project plan. Proceed with high confidence.

---

## Critical Issues (Blockers)

### ✅ NONE - All Critical Issues Resolved

The original critical blocker (missing `/packages/maproom-mcp/src/tools/search.ts`) was identified in the initial review and **successfully addressed** in `review-updates.md` Phase 1:

**Resolution Summary:**
- **Phase 0 Added**: 2 tickets (SEMRANK-0001, SEMRANK-0002) to create MCP search tool
- **Timeline Adjusted**: 2-3 weeks → 3.5-4.5 weeks (accounts for tool creation)
- **Dependency Chain Fixed**: Phase 0 gates Phase 1, which gates Phase 2
- **Ticket Count Updated**: 18 tickets → 21 tickets (including Phase 0)

**Evidence of Resolution:**
1. `plan.md` lines 25-57: Phase 0 fully documented
2. `README.md` line 55: Updated ticket count to 21
3. `review-updates.md` lines 29-54: Complete resolution tracking
4. All 21 tickets created in `tickets/` directory

**Status:** ✅ **RESOLVED - Ready to Execute**

---

## Reinvention & Duplication Analysis

### ✅ NO REINVENTION DETECTED

**Proper Reuse Strategy:**

1. **Builds on Existing Rust FTS** ✅
   - Enhances `/crates/maproom/src/search/fts.rs` (lines 77-99)
   - Replaces old +0.2 exact bonus with multiplicative scoring
   - No duplication of FTS logic

2. **Reuses Database Schema** ✅
   - Leverages existing `chunks.kind` enum
   - Leverages existing `chunks.symbol_name` field
   - No new tables or columns needed

3. **Follows MCP Tool Patterns** ✅
   - Structure mirrors `context.ts`, `open.ts`, `upsert.ts`
   - Subprocess pattern for Rust binary invocation
   - NDJSON parsing (established pattern)

4. **Uses Established Test Infrastructure** ✅
   - Vitest (already configured)
   - PostgreSQL test fixtures
   - Integration test patterns from existing tools

### Minor Observation: Search Tool Implementation Pattern

**Current State:**
- Search functionality exists **inline** in `/packages/maproom-mcp/src/index.ts` (lines 118-200+)
- Tool schema defined in `toolSchemas` array
- Handler logic likely in `tools/call` switch statement

**Phase 0 Intent:**
- Create `/packages/maproom-mcp/src/tools/search.ts` (separate file)
- Follows pattern of `context.ts`, `open.ts`, `explain.ts`

**Question for SEMRANK-0001:**
Should Phase 0:
- **Option A** (Recommended): Extract to separate `tools/search.ts` + `tools/search_schema.ts`
- **Option B**: Keep inline but enhance parameters

**Recommendation:**
Update SEMRANK-0001 acceptance criteria to specify:
```markdown
Create two new files:
- /packages/maproom-mcp/src/tools/search.ts (tool implementation)
- /packages/maproom-mcp/src/tools/search_schema.ts (zod schema)

Follow pattern from context.ts/context_schema.ts:
- Export async searchTool(params, db, log) function
- Call Rust binary via spawn()
- Parse NDJSON results
- Return SearchResult[]

Extract search logic currently inline in index.ts (around line 550+).
```

**Impact:** Low - just clarifies implementation approach, doesn't change plan validity.

---

## High-Risk Areas (Warnings)

### Risk 1: Query Normalization Complexity

**Risk Level:** Medium
**Category:** Technical Complexity
**Ticket:** SEMRANK-2004b

**Description:**
Normalization must handle multiple edge cases:
- **Basic cases**: camelCase → snake_case, kebab-case → snake_case
- **Acronyms**: XMLParser → xml_parser, HTTPSHandler → https_handler
- **Consecutive capitals**: validateHTTPRequest → validate_http_request
- **Mixed with numbers**: Base64Encoder → base64_encoder, OAuth2TokenValidator → oauth2_token_validator
- **Complex combos**: HTTPSConnectionXML → https_connection_xml

**Probability:** High (complexity underestimated)
**Impact:** Medium (incorrect normalization reduces exact match quality)

**Mitigation Options:**

**Option 1: Use Existing Library**
```bash
# Check if normalization library already exists
grep -E "(lodash|change-case|case)" /workspace/packages/maproom-mcp/package.json
```
- If `lodash` or `change-case` exists: Use `_.snakeCase()` or equivalent
- If not: Consider adding `change-case` npm package (low risk, well-tested)

**Option 2: TDD with Comprehensive Test Cases**
```typescript
// Create tests FIRST in /packages/maproom-mcp/tests/unit/normalize.test.ts
describe('normalizeForExactMatch edge cases', () => {
  test('XMLParser → xml_parser', () => { ... });
  test('HTTPSHandler → https_handler', () => { ... });
  test('validateHTTPRequest → validate_http_request', () => { ... });
  test('HTTPSConnectionXML → https_connection_xml', () => { ... });
  test('Base64URLEncoder → base64_url_encoder', () => { ... });
  test('OAuth2TokenValidator → oauth2_token_validator', () => { ... });
});
```

**Option 3: Adjust Timeline**
- Change SEMRANK-2004b estimate: 1 day → 1.5 days
- Split into subtasks if needed:
  - 2004b-1: Basic normalization (0.5 days)
  - 2004b-2: Acronym handling with tests (1 day)

**Recommendation:**
1. Check for existing normalization library in dependencies
2. If exists: Use it (update ticket to reference)
3. If not: Add comprehensive test cases + extend estimate to 1.5 days

### Risk 2: Kind Enum Value Mismatches

**Risk Level:** ✅ Low (Already Mitigated)
**Category:** Technical Correctness
**Ticket:** SEMRANK-2003

**Original Issue:**
Architecture.md initially assumed `kind='function'` but database uses `kind='func'`.

**Resolution (from review-updates.md lines 82-100):**
- ✅ Architecture.md updated with correct enum values
- ✅ CASE statement uses: 'func','class','component','hook','module','var','type','other'
- ✅ SEMRANK-2003 includes validation: "Query SELECT DISTINCT kind FROM chunks"
- ✅ Acceptance criteria: "Verify CASE statement kind values match database enum"

**Status:** ✅ **MITIGATED - No Further Action Needed**

### Risk 3: Test Corpus Creation Scope Creep

**Risk Level:** Low (Well-Mitigated)
**Category:** Execution Risk
**Ticket:** SEMRANK-1003

**Mitigation (from plan.md lines 65-75):**
- ✅ Hard limit: 50 chunks maximum
- ✅ Time box: 1 day maximum
- ✅ Fallback: "Use existing maproom codebase subset if creation exceeds time"

**Alternative Recommendation:**
**Use Maproom's Own Codebase as Test Corpus** (Meta-Testing)

Maproom already has:
- ✅ Rust code: `crates/maproom/src/search/fts.rs`, `graph.rs`, `vector.rs`
- ✅ TypeScript code: `packages/maproom-mcp/src/tools/context.ts`, `open.ts`
- ✅ Tests: `crates/maproom/tests/`, `packages/maproom-mcp/tests/`

**Test Queries:**
```
"FTSExecutor" → Should return fts.rs (implementation), NOT test file
"context" → Should return context.ts, NOT context.test.ts
"searchTool" → Should return search.ts (after Phase 0 creates it)
```

**Benefits:**
1. **Realistic**: Real production code, not synthetic examples
2. **Meta-testing**: Using maproom to test maproom improvements
3. **Time-saving**: No corpus creation needed
4. **Validation**: Already indexed, metadata already correct

**Action:** Update SEMRANK-1003 recommendation (optional enhancement).

### Risk 4: Performance Regression

**Risk Level:** Low
**Category:** Non-Functional Requirement
**Mitigation:** Comprehensive (Already in Plan)

**Mitigation Strategy (from plan.md):**
- ✅ Baseline metrics in Phase 1 (SEMRANK-1005)
- ✅ Performance benchmarks in Phase 3 (SEMRANK-3005)
- ✅ Target: p95 latency increase <10%
- ✅ Hard SLO in CI/CD (SEMRANK-4005)
- ✅ Easy rollback (revert SQL query)

**Contingency Plan (from plan.md lines 332-342):**
- Profile slow queries with EXPLAIN ANALYZE
- Simplify CASE logic if needed
- Add database indices (unlikely to be necessary)

**Status:** ✅ **WELL-MITIGATED - Acceptable Risk**

---

## Gaps & Ambiguities

### Technical Gaps

#### Gap 1: Search Tool File Structure (Minor Clarification)

**Gap:** Tickets reference `/packages/maproom-mcp/src/tools/search.ts` but current implementation is inline in `index.ts`.

**Impact:** Low - Could cause initial confusion in SEMRANK-0001

**Resolution:**
Add implementation note to SEMRANK-0001:
```markdown
**File Structure:**
Create two new files following established MCP tool pattern:
- /packages/maproom-mcp/src/tools/search.ts
- /packages/maproom-mcp/src/tools/search_schema.ts

**Pattern Reference:**
See context.ts/context_schema.ts for structure:
- Export async function searchTool(params, db, log)
- Call Rust binary via spawn()
- Parse NDJSON output
- Return typed SearchResult[]

**Extract from index.ts:**
Current search logic is inline in index.ts (around line 550+).
Extract to separate file for consistency and testability.
```

#### Gap 2: Subprocess Integration Pattern (Minor Documentation)

**Gap:** SEMRANK-0001 doesn't explicitly reference existing subprocess pattern.

**Impact:** Very Low - Pattern exists in `upsert.ts`, just needs reference

**Resolution:**
Add to SEMRANK-0001 implementation notes:
```markdown
**Subprocess Pattern Reference:**
Follow the pattern from `/packages/maproom-mcp/src/tools/upsert.ts`:
1. Import { spawn } from 'node:child_process'
2. Spawn crewchief-maproom binary with args
3. Collect stdout (NDJSON format)
4. Parse line-by-line: lines.map(l => JSON.parse(l))
5. Handle stderr for errors
6. Return typed results

**Error Handling:**
- Rust binary not found → clear error message
- Malformed NDJSON → log and skip line
- Empty results → return empty array (not error)
- Rust stderr → log as warning
```

#### Gap 3: Normalization Library Check (Action Item)

**Gap:** Plan doesn't specify whether to use existing library or implement from scratch.

**Impact:** Low - Affects implementation time and quality

**Resolution:**
Before starting SEMRANK-2004b:
```bash
# Check maproom-mcp dependencies
grep -E "(lodash|change-case|case)" /workspace/packages/maproom-mcp/package.json

# If library exists: Update ticket to use it
# If not: Decide whether to add library or implement custom
```

**Recommendation:** Use library if exists, otherwise add `change-case` (well-tested, small footprint).

### Requirements Gaps

**NONE IDENTIFIED**

All requirements are specific, measurable, and have clear acceptance criteria.

### Process Gaps

**NONE IDENTIFIED**

Workflow is clear:
1. Implementation agent completes work
2. unit-test-runner executes tests (no fixes)
3. verify-ticket checks acceptance criteria
4. commit-ticket creates conventional commit

---

## Scope & Feasibility Concerns

### Scope Creep Indicators

**✅ NONE DETECTED - Excellent MVP Discipline**

**Evidence:**

1. **Clear Deferral to Future** (plan.md lines 423-450):
   - Configurable multipliers (1-2 weeks) - Deferred
   - Graph signal integration (2-3 weeks) - Deferred
   - Learning to rank (4-6 weeks) - Deferred
   - Query intent classification (3-4 weeks) - Deferred
   - Personalized ranking (2-3 weeks) - Deferred

2. **Phase Structure is Minimal:**
   - Phase 0: Create prerequisite only
   - Phase 1: Foundation only (no implementation)
   - Phase 2: Core features only (no extras)
   - Phase 3: Essential testing only
   - Phase 4: Deployment prep only
   - Phase 5: Verification and commit

3. **Explicit Scope Constraints:**
   - Test corpus: 50 chunks maximum (line 70)
   - Time box: 1 day maximum (line 71)
   - Fallback: Use existing code (line 72)

**Assessment:** ✅ **Strong MVP Discipline**

### Feasibility Challenges

#### Challenge 1: Normalization Complexity

**Status:** Addressed in Risk 1 above

**Recommendation:**
- Check for existing library
- Add comprehensive test cases
- Adjust estimate if implementing from scratch

#### Challenge 2: Test Corpus Creation Time

**Status:** Well-mitigated with constraints + fallback

**Alternative Approach:** Use maproom's own codebase (meta-testing)

**Benefits:**
- No creation time needed
- Realistic test data
- Already indexed
- Meta-validation (using maproom to test maproom)

---

## Alignment Assessment

### MVP Discipline

**Rating:** ✅ **Strong** (9/10)

**Evidence:**
- Explicit "Future Enhancements (Out of Scope)" section
- No premature optimization (multipliers hardcoded)
- No configuration system in MVP
- Testing focuses on correctness, not coverage
- Simple SQL CASE statements, not ML

**Quote from plan.md:**
> "Fallback: Use existing maproom codebase subset if creation exceeds time"

This demonstrates pragmatic risk management.

### Pragmatism Score

**Rating:** ✅ **Strong** (9/10)

**Evidence:**
- SQL multipliers instead of ML ranking
- No schema changes (lowest risk)
- Trivial rollback (revert one SQL query)
- Time-boxed test corpus creation
- Representative samples, not full applications

**Architecture Decision (from architecture.md):**
> "Multiplicative scoring (base × kind × exact) better than additive because it compounds signals rather than diluting them."

Simple, justified, effective.

### Agent Compatibility

**Rating:** ✅ **Strong** (8/10)

**Evidence:**
- Task sizes: 0.5-2 days (appropriate for agents)
- Clear acceptance criteria (checkboxes)
- Agent assignments match capabilities
- Explicit dependencies
- No human judgment required

**Minor Note:**
SEMRANK-2004b (normalization) may be complex for autonomous agent. Suggest providing:
- Reference implementation OR
- Specific library to use (`change-case` npm package)

### Codebase Integration

**Rating:** ✅ **Strong** (9/10)

**Evidence:**
- Builds on existing Rust FTS (`fts.rs:77-99`)
- Reuses database schema (no migrations)
- Follows MCP tool patterns
- References existing code explicitly
- Phase 0 creates proper API surface

**Integration Boundaries Respected:**
```
MCP Client
  ↓ MCP Protocol
TypeScript MCP Server (index.ts, tools/search.ts)
  ↓ Subprocess/NDJSON
Rust Binary (crewchief-maproom)
  ↓ SQL (parameterized queries)
PostgreSQL Database
```

**Minor Observation:** Search tool location needs clarification (inline vs separate file).

### Separation of Concerns

**Rating:** ✅ **Strong** (9/10)

**Evidence:**
- TypeScript wraps Rust (proper boundary)
- SQL changes confined to Rust fts.rs
- No reaching across module boundaries
- Subprocess communication (loose coupling)
- Phase 0 creates API before enhancement

**No Boundary Violations Detected**

---

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from
- [x] Test strategy is defined and pragmatic
- [x] Security concerns addressed (security-review.md)
- [x] Dependencies on existing systems documented
- [x] Phase 0 addresses missing prerequisite

### Technical
- [x] Technology choices appropriate (SQL multipliers)
- [x] Dependencies identified and available (PostgreSQL, tree-sitter)
- [x] Integration points well-defined (Rust FTS, MCP protocol)
- [x] Performance requirements clear (p95 <10% increase)
- [x] Error handling specified (null kind, empty query)
- [⚠️] Normalization library check needed (action item)
- [x] No unnecessary duplication

### Process
- [x] Agent assignments appropriate
- [x] Task boundaries clear (2-8 hour chunks)
- [x] Verification criteria explicit (checkboxes)
- [x] Handoffs defined (implement → test → verify → commit)
- [x] Rollback plan exists (revert SQL query)
- [x] Integration with existing workflows considered

### Integration & Reuse
- [x] Existing solutions evaluated (Rust FTS enhanced, not replaced)
- [x] Current patterns followed (MCP tool structure)
- [x] Reusable components identified (Rust FTS, DB schema)
- [x] Integration points mapped (subprocess/NDJSON)
- [x] No reinvention of functionality
- [x] Proper integration methods chosen:
  - [x] MCP Protocol for client communication
  - [x] Subprocess/NDJSON for TypeScript→Rust
  - [x] SQL parameters for database queries
- [⚠️] Component boundaries respected (minor: search tool location)
- [x] Public interfaces used (MCP tool schema)
- [x] Appropriate coupling maintained (loose via subprocess)

### Tickets (Created: 21 tickets)
- [x] Tickets align with plan objectives
- [x] All plan deliverables have tickets
- [x] Dependencies properly sequenced
- [x] Scope per ticket appropriate (0.5-2 days)
- [x] Acceptance criteria measurable (checkboxes)
- [x] Phase 0 tickets address prerequisite
- [x] Ticket index created (`SEMRANK_TICKET_INDEX.md`)

### Risk
- [x] Major risks identified (performance, normalization, multiplier tuning)
- [x] Mitigation strategies exist (benchmarks, tests, easy adjustment)
- [x] Dependencies have fallbacks (test corpus: use maproom)
- [x] Critical path protected (Phase 0 gates Phase 1)
- [x] Failure modes understood (rollback plan)
- [x] Original blocker resolved (Phase 0 creates search tool)

---

## Recommendations

### Immediate Actions (Before Starting Phase 0)

1. **Clarify Search Tool File Structure** (SEMRANK-0001)
   - **Action**: Update ticket implementation notes
   - **Specify**: Create `tools/search.ts` + `tools/search_schema.ts` (separate files)
   - **Pattern**: Follow `context.ts`/`context_schema.ts` structure
   - **Effort**: 5 minutes to update ticket
   - **Impact**: Eliminates ambiguity for agent execution

2. **Check Normalization Library** (Before SEMRANK-2004b)
   - **Action**: `grep -E "(lodash|change-case)" /workspace/packages/maproom-mcp/package.json`
   - **If exists**: Update SEMRANK-2004b to use it
   - **If not**: Decide whether to add library or implement custom
   - **Effort**: 2 minutes to check, 5 minutes to update ticket
   - **Impact**: Reduces implementation risk

3. **Add Normalization Edge Case Tests** (SEMRANK-2004b)
   - **Action**: Enhance acceptance criteria with specific test cases
   - **Add**: `HTTPSConnectionXML`, `OAuth2TokenValidator`, `Base64URLEncoder`
   - **Effort**: 3 minutes to update ticket
   - **Impact**: Comprehensive test coverage

### Optional Enhancements (Not Blocking)

1. **Use Maproom Codebase as Test Corpus** (SEMRANK-1003)
   - **Rationale**: Realistic, already indexed, meta-testing
   - **Update**: Add recommendation to use `crates/maproom/src/search/` and `packages/maproom-mcp/src/tools/`
   - **Benefit**: Saves corpus creation time
   - **Risk**: Low - fallback already specified in plan

2. **Adjust Normalization Estimate** (SEMRANK-2004b)
   - **Current**: 1 day
   - **Recommended**: 1.5 days (if implementing from scratch)
   - **Rationale**: Acronym handling is complex
   - **Impact**: More realistic timeline

3. **Add Subprocess Pattern Reference** (SEMRANK-0001)
   - **Action**: Reference `upsert.ts` spawn() pattern in ticket
   - **Benefit**: Clear implementation guide for agent
   - **Effort**: 2 minutes to update ticket

### Phase 1-5 Execution

**No Changes Needed**

Phases 1-5 are well-planned and ready for execution as documented.

---

## Review Conclusion

### Readiness Assessment

**Can this project succeed as currently defined?** ✅ **Yes - High Confidence**

**Resolution of Original Blocker:**
The critical blocker (missing search tool) has been **completely resolved** through:
1. Addition of Phase 0 (2 tickets, 2-3 days)
2. Timeline adjustment (2-3 weeks → 3.5-4.5 weeks)
3. Proper dependency sequencing (Phase 0 → Phase 1 → Phase 2)
4. All 21 tickets created with clear acceptance criteria

**Current Concerns:**
1. Search tool file structure needs minor clarification (5-minute fix)
2. Normalization library check recommended (2-minute check)
3. Edge case test coverage should be enhanced (3-minute update)

**Total Effort to Address:** ~10 minutes of ticket updates

### Recommended Path Forward

**✅ PROCEED - Ready for Execution**

**Minor Pre-Flight Checklist:**
- [ ] Clarify search tool file structure in SEMRANK-0001 (5 min)
- [ ] Check for normalization library dependency (2 min)
- [ ] Add edge case tests to SEMRANK-2004b (3 min)

**After 10-minute checklist:**
Execute with `/work-on-project SEMRANK` or `/single-ticket SEMRANK-0001`

### Success Probability

**Current State:** 85% (Excellent)
**After 10-minute updates:** 90%

**Why 85% is High Confidence:**
- ✅ Critical blocker resolved (Phase 0)
- ✅ Problem well-understood (documented failures)
- ✅ Solution is pragmatic (SQL multipliers, not ML)
- ✅ Comprehensive testing (without ceremony)
- ✅ Easy rollback (revert one SQL query)
- ✅ Clear success metrics (>90% top-1 accuracy, <10% latency)
- ✅ Proper risk management (time boxes, fallbacks)

**Why Not 95%+?**
- Normalization has inherent complexity (many edge cases)
- First-time search tool extraction (learning curve)
- Performance benchmarks need real data (can't fully predict)

**Risk-Adjusted Timeline:**
- Planned: 18-24 days (3.5-4.5 weeks)
- Realistic: 20-26 days (4-5 weeks) with normalization complexity buffer

### Final Notes

**This is an exemplary project plan** that demonstrates:
- ✅ Excellent risk management (identified and resolved critical blocker)
- ✅ MVP discipline (defers 5 features to future)
- ✅ Pragmatic architecture (SQL multipliers, not overengineered)
- ✅ Comprehensive testing (correctness-focused, not ceremonial)
- ✅ Clear success criteria (measurable, achievable)
- ✅ Proper integration (builds on Rust FTS, respects boundaries)
- ✅ Appropriate separation of concerns (MCP → TypeScript → Rust → PostgreSQL)

**Key Success Factors:**
1. **Problem is real**: Documented failures in `analysis.md`
2. **Solution is minimal**: Just multipliers, no schema changes
3. **Execution is phased**: Prerequisites → Foundation → Implementation → Testing → Deployment
4. **Rollback is trivial**: Revert one SQL query
5. **Success is measurable**: Implementation ranks #1 (not subjective)
6. **Integration is proper**: Each layer uses appropriate interface

**Recommendation: Execute immediately with high confidence.**

This project will deliver meaningful value (correct entry points for graph traversal) with minimal risk (no schema changes, easy rollback, comprehensive testing).

---

## Appendices

### A. Integration Pattern Summary

**TypeScript → Rust Communication:**
```typescript
// Correct Pattern (from upsert.ts)
const child = spawn('crewchief-maproom', ['search', '--query', params.query]);
let output = '';
child.stdout.on('data', (chunk) => { output += chunk; });
await new Promise((resolve, reject) => {
  child.on('close', (code) => code === 0 ? resolve(null) : reject(new Error(`Exit ${code}`)));
});

// Parse NDJSON (one JSON object per line)
const lines = output.split('\n').filter(l => l.trim());
const results = lines.map(l => JSON.parse(l));
```

### B. Normalization Test Cases

**Comprehensive Edge Cases (add to SEMRANK-2004b):**
```typescript
// Basic Cases
normalize('validateProvider') → 'validate_provider'
normalize('validate-provider') → 'validate_provider'
normalize('validate provider') → 'validate_provider'

// Acronym Cases
normalize('XMLParser') → 'xml_parser'
normalize('HTTPSHandler') → 'https_handler'
normalize('validateHTTPRequest') → 'validate_http_request'
normalize('HTTPSConnectionXML') → 'https_connection_xml'

// Number Cases
normalize('Base64Encoder') → 'base64_encoder'
normalize('Base64URLEncoder') → 'base64_url_encoder'
normalize('OAuth2TokenValidator') → 'oauth2_token_validator'
normalize('validateHTTP2Request') → 'validate_http2_request'

// Complex Combos
normalize('parseJSONFromXML') → 'parse_json_from_xml'
normalize('convertHTMLToMarkdown') → 'convert_html_to_markdown'
```

### C. Test Corpus Recommendation

**Use Maproom's Own Codebase:**

| Language   | Files | Expected Results |
|------------|-------|------------------|
| Rust       | `crates/maproom/src/search/fts.rs` | Query "FTSExecutor" → returns fts.rs (not test) |
| TypeScript | `packages/maproom-mcp/src/tools/context.ts` | Query "context" → returns context.ts (not test) |
| Tests      | `crates/maproom/tests/search_test.rs` | Should rank BELOW implementations |

**Benefits:**
- ✅ Realistic production code
- ✅ Already indexed with correct metadata
- ✅ Meta-testing (using maproom to test maproom)
- ✅ No creation time needed

### D. Risk Register (Updated)

| Risk ID | Risk | Probability | Impact | Mitigation | Status |
|---------|------|------------|--------|------------|--------|
| R0 | Missing search tool (BLOCKER) | N/A | Critical | Phase 0 added | ✅ RESOLVED |
| R1 | Normalization complexity | High | Medium | Use library, add tests, +0.5 days | Monitor |
| R2 | Search tool structure ambiguity | Low | Low | Clarify in ticket (5 min) | Action Needed |
| R3 | Kind enum mismatch | Very Low | High | Already mitigated in Phase 2 | ✅ RESOLVED |
| R4 | Performance regression | Low | Medium | Benchmarks, easy rollback | Accepted |
| R5 | Multipliers poorly tuned | Medium | Low | Monitor, easy to adjust | Accepted |
| R6 | Test corpus creation time | Low | Low | Time box + fallback (use maproom) | Mitigated |

### E. Success Metrics Dashboard

**Must-Have (Blocking):**
- [ ] Search "authenticate" returns implementation #1 (not test)
- [ ] Implementation ranks higher than test (same symbol)
- [ ] Implementation ranks higher than documentation
- [ ] Case-insensitive exact match works
- [ ] Null symbol_name doesn't crash
- [ ] Unknown kind doesn't crash
- [ ] Debug mode returns score breakdown
- [ ] p95 latency increase <10% vs baseline
- [ ] All existing search tests pass

**Should-Have (Important):**
- [ ] Multi-word queries normalized correctly
- [ ] Acronym normalization works (XML, HTTPS, OAuth2)
- [ ] Concurrent load test passes (100 queries)
- [ ] Performance benchmarks documented

**Target Metrics:**
- Top-1 accuracy: **>90%** (exact function searches return implementation #1)
- Average implementation rank: **<3** (top 3 results)
- p95 latency: **<200ms** (no significant regression)

---

**End of Review**

**Final Recommendation:** ✅ **EXECUTE IMMEDIATELY**

This project is ready. The critical blocker has been resolved. The plan is solid. The risks are manageable. The success criteria are clear.

**Next Steps:**
1. Complete 10-minute pre-flight checklist (optional but recommended)
2. Execute: `/work-on-project SEMRANK` or `/single-ticket SEMRANK-0001`
3. Monitor normalization complexity in Phase 2
4. Celebrate when implementations rank above tests! 🎉

**Confidence Level:** ✅ **High (85% → 90% with minor updates)**
