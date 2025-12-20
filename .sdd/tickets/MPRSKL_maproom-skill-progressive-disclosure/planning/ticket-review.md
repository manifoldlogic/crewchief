# Ticket Review: Maproom Skill Progressive Disclosure (MPRSKL)

**Review Date:** 2025-12-20
**Review Type:** POST-TASK REVIEW (Tasks created, now reviewing for execution readiness)
**Status:** Needs Work (Critical Divergence)
**Risk Level:** High
**Tickets Reviewed:** 7 tasks across 3 phases
**Critical Finding:** Codebase has diverged significantly from planned implementation

## Executive Summary

This post-task review reveals a **critical mismatch** between planned tasks and current codebase state. The dimension mismatch bug that Phase 1 tasks (MPRSKL.1001, MPRSKL.1002) were designed to fix has already been addressed through a DIFFERENT implementation approach in the codebase.

**The Problem:**
- **Tasks expect**: Add `from_env_with_provider(Option<Provider>)` method and update factory to call it
- **Codebase reality**: Already implemented dimension inference INSIDE `from_env()` using different logic (lines 118-149 of config.rs)
- **Result**: Phase 1 tasks would duplicate existing functionality or require significant rework

**Impact on Other Phases:**
- Phase 2 (documentation tasks) can proceed but references to "Phase 1 fix" are inaccurate
- Phase 3 (error enhancement) can proceed but may reference wrong implementation details
- Overall project narrative is broken - tasks don't match reality

**Recommendation:** SUSPEND execution and update planning to reflect current implementation, OR rewrite Phase 1 tasks to work with existing code.

---

## Critical Issues (Blockers)

### Issue 1: Phase 1 Tasks Based on Outdated Codebase Analysis
**Severity:** Critical
**Location:** MPRSKL.1001, MPRSKL.1002
**Description:**

The planning documents analyzed factory.rs:213 and config.rs to identify a bug where auto-detected Ollama wasn't passed to config, causing dimension mismatch. The proposed solution was to add `from_env_with_provider(Option<Provider>)`.

**However, the current codebase shows a DIFFERENT fix has been implemented:**

```rust
// config.rs lines 118-149 - ALREADY IMPLEMENTED
if config.provider == Provider::Ollama && config.model == "text-embedding-3-small" {
    config.model = "mxbai-embed-large".to_string();
    tracing::debug!("Defaulting to mxbai-embed-large for Ollama provider");
}

// Dimension inference ALREADY exists in from_env()
if explicit_dimension.is_none() && config.provider == Provider::Ollama {
    if let Some(inferred_dim) = infer_ollama_dimension(&config.model) {
        config.dimension = inferred_dim;
    }
}
```

**The factory STILL calls `from_env()` at line 213, but the bug is now fixed INSIDE config.rs rather than through provider propagation.**

**Impact:**
- MPRSKL.1001 would add a method that's unnecessary (inference already works)
- MPRSKL.1002 would modify factory to pass provider that config doesn't need
- Tests would pass but create redundant code paths
- 4-6 hours of implementation time would be wasted

**Required Action:**
1. EITHER: Rewrite Phase 1 tasks to acknowledge existing fix and add provider propagation as ENHANCEMENT (cleaner architecture)
2. OR: Remove Phase 1 entirely and proceed with Phases 2-3, updating references to "the fix already in config.rs"
3. OR: Update analysis.md and architecture.md to reflect current implementation approach

### Issue 2: Task Dependencies Reference Non-Existent Work
**Severity:** Critical
**Location:** MPRSKL.2003 (troubleshooting.md), MPRSKL.3001 (error enhancement)
**Description:**

Multiple tasks reference "the Phase 1 fix" or "MPRSKL.1001-1002" as context:

**MPRSKL.2003 line 105:**
```markdown
**Note:** As of MPRSKL.1001-1002, auto-detected Ollama should correctly infer dimension without manual configuration.
```

**MPRSKL.2003 line 219:**
```
## Dependencies
- **MPRSKL.1001, MPRSKL.1002** (Phase 1 bug fix) - Should reference that dimension mismatch is fixed for auto-detected Ollama
```

But these tasks would create REDUNDANT code, not the actual fix that's already live.

**Impact:**
- Documentation would reference the wrong fix
- Future maintainers would be confused about which code path is authoritative
- Troubleshooting guide would be inaccurate

**Required Action:**
- Update all task references to cite current implementation (config.rs lines 118-149)
- Remove dependencies on MPRSKL.1001-1002 if those tasks are revised/removed

### Issue 3: Architecture Document Contradicts Codebase
**Severity:** High
**Location:** architecture.md Decision 1, Component 2
**Description:**

Architecture.md describes adding `from_env_with_provider()` as the solution with detailed implementation notes. However, codebase has implemented dimension inference differently (model defaulting + inference check in from_env() itself).

**Contradiction:**
- **Architecture says**: "Provider override is applied BEFORE env var loading"
- **Codebase does**: Provider is already set from env var, then model and dimension are adjusted based on provider

**Impact:** Tasks implementing architecture.md would conflict with existing code

**Required Action:** Update architecture.md to reflect actual implementation OR justify why provider propagation is still valuable as refactoring

---

## High-Risk Areas (Warnings)

### Risk 1: Documentation Tasks May Reference Wrong Implementation
**Risk Level:** High
**Description:**

Tasks MPRSKL.2001, MPRSKL.2002, MPRSKL.2003 create skill documentation. They're written assuming Phase 1 adds `from_env_with_provider()`, but that method doesn't exist (and may not be created if Phase 1 is revised).

**Specific References:**
- MPRSKL.2003 implementation notes show error message referencing provider override
- Quality-strategy.md test cases test `from_env_with_provider()` method that may not exist

**Mitigation:** Review all doc task acceptance criteria and remove references to Phase 1 implementation details. Focus on USER-FACING behavior (dimension inference works with Ollama) rather than HOW it works.

### Risk 2: Current Implementation May Still Have Edge Cases
**Risk Level:** Medium
**Description:**

Current fix in config.rs only works if `MAPROOM_EMBEDDING_PROVIDER` is set explicitly. If Ollama is auto-detected in factory.rs but no env var is set, the bug might still exist!

**Code Analysis:**
```rust
// factory.rs:174-177 - Ollama detected but provider_name is local variable
match detect_ollama_endpoint().await {
    Some(endpoint) => {
        ("ollama".to_string(), Some(endpoint))  // NOT propagated to config!
    }
}

// factory.rs:213 - Still calls from_env()
let config = EmbeddingConfig::from_env()?;  // Provider defaults to OpenAI!
```

**Verification Needed:** Check if auto-detected Ollama (without `MAPROOM_EMBEDDING_PROVIDER` env var) actually triggers dimension inference. The current code may STILL have the bug!

**Required Action:**
1. Test zero-config scenario: `unset MAPROOM_EMBEDDING_PROVIDER`, run with Ollama
2. If bug exists, Phase 1 tasks are still valid but need updated context
3. If bug is fixed differently, document HOW and update tasks accordingly

### Risk 3: Test Strategy May Not Match Implementation
**Risk Level:** Medium
**Description:**

quality-strategy.md defines tests for `from_env_with_provider()` method:

```rust
fn test_from_env_with_provider_ollama_infers_dimension() {
    let config = EmbeddingConfig::from_env_with_provider(Some(Provider::Ollama)).unwrap();
    assert_eq!(config.dimension, 1024);
}
```

If this method is never created, these test cases are invalid.

**Mitigation:** Update quality-strategy.md to test current implementation approach, OR keep test strategy if Phase 1 proceeds as planned (adding the method as refactoring)

---

## Task-by-Task Review

### Phase 1: Factory/Config Bug Fix

#### MPRSKL.1001: Add from_env_with_provider() to EmbeddingConfig
**Status:** ⚠️ NEEDS REVISION (Redundant with existing code)
**Issues:**
1. **Acceptance Criteria Mismatch**: "Provider override enables correct dimension inference for Ollama (1024 for mxbai-embed-large)" - This ALREADY works in current code
2. **Implementation Notes**: Show pattern that would duplicate existing inference logic in lines 133-149
3. **Missing Context**: Task doesn't acknowledge existing dimension inference implementation

**Specific Problems:**
- Line 34: "Provider override enables correct dimension inference" - Already works without override
- Line 68-76: Implementation pattern shows creating new method when inference already exists in from_env()
- Line 86: "Dependencies: None" - Should note "May conflict with existing dimension inference at config.rs:133-149"

**Recommended Changes:**
- EITHER: Reframe as "refactoring to enable cleaner factory/config separation" (architectural improvement)
- OR: Remove entirely if current implementation is satisfactory
- Add acceptance criterion: "Does not duplicate existing dimension inference logic"
- Add verification check: "Existing inference tests still pass"

#### MPRSKL.1002: Update factory.rs to propagate provider
**Status:** ⚠️ NEEDS REVISION (May be unnecessary)
**Issues:**
1. **Line 32**: "Dimension is correctly inferred as 1024 for mxbai-embed-large" - This should ALREADY work with current config.rs
2. **Line 56**: Shows calling `from_env_with_provider(Some(Provider::Ollama))` but this method may not exist
3. **Line 100**: "Dependencies: MPRSKL.1001" - Broken if 1001 is revised/removed

**Critical Question:** Does auto-detected Ollama (factory.rs:174-177) actually propagate to config? Current code suggests NO - bug may still exist!

**Recommended Changes:**
- Test current behavior first: Does zero-config Ollama work?
- If yes: Task becomes "add integration test documenting the fix"
- If no: Task is valid but needs updated context ("bug still exists despite inference code")

### Phase 2: Progressive Disclosure Skill Restructure

#### MPRSKL.2001: Create brief SKILL.md
**Status:** ✅ READY with minor revisions
**Issues:**
1. **Line 101**: "Dependencies: None" - Actually DOES depend on understanding current CLI behavior
2. **Lines 48-90**: Implementation notes show 35-45 lines - very detailed, good guidance

**Strengths:**
- Clear line count target (< 50 lines)
- Good structure guidance
- Concrete command examples
- Verification checklist is thorough

**Recommended Changes:**
- Remove references to Phase 1 tasks (lines don't explicitly reference but architecture might)
- Add acceptance criterion: "Commands verified against `crewchief-maproom --help` output"
- Clarify: "Brief SKILL.md replaces existing 196-line version or creates new file?"

**Verification Concern:** Line 134-138 shows line count command that may be incorrect syntax. Test verification commands before task execution.

#### MPRSKL.2002: Create cli-reference.md
**Status:** ✅ READY
**Issues:** None significant

**Strengths:**
- Clear scope (all commands documented)
- Good organization by category
- Verification includes checking against actual CLI
- Commands enumerated from real CLI

**Recommended Changes:**
- Add to acceptance criteria: "Environment variables match current config.rs implementation"
- Note: Should document CURRENT dimension inference behavior (not planned from_env_with_provider)

**Minor Concern:** Line 121-130 lists commands "from crewchief-maproom --help" but doesn't specify WHICH version. Should pin to current version or acknowledge it may change.

#### MPRSKL.2003: Create troubleshooting.md
**Status:** ⚠️ NEEDS REVISION (References wrong fix)
**Issues:**
1. **Line 105**: "**Note:** As of MPRSKL.1001-1002, auto-detected Ollama should correctly infer dimension" - References tasks that may not happen
2. **Line 219**: Dependencies on MPRSKL.1001, 1002 - Broken if Phase 1 changes
3. **Lines 80-111**: Error solutions reference provider configuration but may need to match CURRENT implementation

**Strengths:**
- Good structure (error message, cause, solution, prevention)
- Covers 3+ common errors as required
- Configuration verification section is practical
- Links to other references

**Recommended Changes:**
- Remove dependencies on Phase 1 tasks
- Update dimension mismatch section to reference CURRENT fix (config.rs dimension inference)
- Change line 105 note to: "Maproom automatically infers dimension for Ollama models (mxbai-embed-large: 1024, nomic-embed-text: 768)"
- Update solutions to match current config behavior

### Phase 3: CLI Error Enhancement

#### MPRSKL.3001: Enhance dimension mismatch error
**Status:** ⚠️ NEEDS REVISION (References wrong config API)
**Issues:**
1. **Lines 96-110**: Implementation shows adding config to error variant - May be complex refactoring
2. **Line 139**: "Dependencies: MPRSKL.1001, 1002" - Broken if Phase 1 changes
3. **Lines 54-87**: Error format examples reference "config" object that may not be available at error site

**Strengths:**
- Clear before/after error format
- Actionable solutions in error message
- Test case for error message content
- Good implementation notes about error message length

**Recommended Changes:**
- Remove Phase 1 dependencies
- Simplify implementation: Enhance message WITHOUT requiring config object in error variant
- Alternative: Add config context at CATCH site, not in error definition
- Update test case to match actual error format

**Design Question:** Is it better to pass config to error variant OR capture config context when catching DimensionMismatch? Task assumes former, but latter might be simpler.

#### MPRSKL.3002: Improve --generate-embeddings documentation
**Status:** ✅ READY
**Issues:** None blocking

**Strengths:**
- Clear scope (help text only)
- Shows both short and long help options
- Specific examples of improved text
- No code logic changes (safe)

**Minor Issues:**
- Line 103: "Dependencies: MPRSKL.2003" - Not a hard dependency, just useful to reference troubleshooting
- Lines 58-79: Two alternative implementations shown - Should pick one or clarify when to use each

**Recommended Changes:**
- Clarify which implementation approach to use (with long_help vs simpler version)
- Add acceptance criterion: "Help text accuracy verified by testing both flag syntaxes"

---

## Cross-Task Analysis

### Dependency Correctness
**Status:** ❌ BROKEN dependency chain

**Dependency Map:**
```
MPRSKL.1001 (config method)
    ↓
MPRSKL.1002 (factory update) ← Depends on 1001
    ↓
MPRSKL.2003 (troubleshooting) ← References 1001-1002
MPRSKL.3001 (error message) ← References 1001-1002
```

**Problems:**
1. If MPRSKL.1001 is revised/removed, all downstream dependencies break
2. MPRSKL.2003 content references "Phase 1 fix" that may not match reality
3. No tasks are truly independent - can't cherry-pick

**Circular Dependency:** None
**Blocking Dependencies:** MPRSKL.1002 blocks on 1001 (critical if 1001 changes)

### Coverage Completeness
**Status:** ⚠️ Partial Coverage with Gaps

**Planned Work Covered:**
- ✅ Bug fix (Phase 1) - BUT implementation may be wrong/redundant
- ✅ Skill restructure (Phase 2) - Good coverage
- ✅ CLI improvements (Phase 3) - Good coverage

**Gaps Identified:**
1. **No task validates current dimension inference implementation** - Should have "verify existing fix works" task
2. **No task updates CLAUDE.md** - Significant code change should update component docs
3. **No task for integration testing zero-config workflow** - Phase 1 test is marked #[ignore], no task ensures it runs
4. **No task addresses search-best-practices.md** - Plan mentions "minor refinements if needed" but no task for this

### Scope Overlap
**Status:** ⚠️ Some overlap concerns

**Potential Conflicts:**
1. **MPRSKL.2001 and 2002**: Both document CLI commands (SKILL.md has quick reference, cli-reference.md has full docs) - Clear delineation needed
2. **MPRSKL.2003 and 3001**: Troubleshooting doc describes dimension mismatch error, task 3001 changes that error - May need coordination
3. **All Phase 2 tasks**: Touch references/ directory - Need to ensure consistent link format and cross-references

**File Boundary Issues:**
- config.rs modified by 1001, read by 2002 (for env var docs), referenced by 2003 (troubleshooting)
- No Git merge conflicts but logical consistency required

### Consistency with Planning Docs
**Status:** ❌ MAJOR INCONSISTENCIES

**Analysis.md Accuracy:** ❌
- Lines 17-64 describe bug flow that may no longer apply
- Lines 189-206 recommend "Option B" (pass provider to config) which may not match current implementation

**Architecture.md Accuracy:** ❌
- Decision 1 (lines 15-56) describes `from_env_with_provider()` as THE solution
- Component 1 (lines 145-169) shows interface with new method
- Component 2 (lines 171-191) shows factory changes
- **All contradict current codebase if dimension inference already works**

**Plan.md Accuracy:** ⚠️ Mostly accurate
- Phase descriptions still valid (bug fix, docs, CLI)
- Task list matches created tasks
- But Phase 1 approach may be wrong

**Quality-strategy.md Accuracy:** ❌
- Lines 54-100 define test cases for `from_env_with_provider()` that may never exist
- Lines 109-134 show integration test for zero-config that should be validated first

**Security-review.md Accuracy:** ✅
- No security issues - doc is generic enough to survive implementation changes

---

## Execution Readiness

### Checklist

- [❌] Requirements specific enough for tasks
  - Phase 1 requirements contradict current code state

- [⚠️] Technical specs implementable
  - Phase 2-3 specs are good
  - Phase 1 specs may require rework

- [✅] Agent assignments clear
  - rust-indexer-engineer: Phases 1 & 3
  - general: Phase 2
  - verify-task: All phases

- [❌] Dependencies identified correctly
  - Phase 1→2→3 chain assumes Phase 1 proceeds as planned
  - Broken if Phase 1 changes

- [❌] No blocking issues
  - Critical: Task 1001-1002 may duplicate existing code
  - High: Tasks reference wrong implementation

- [⚠️] Tasks properly scoped (2-8 hours)
  - Phase 2-3 tasks well-scoped
  - Phase 1 tasks may require investigation time to resolve conflict

- [❌] Task sequence logical
  - Sequence is logical IF Phase 1 proceeds
  - Broken if Phase 1 removed/changed

**Overall Readiness:** ❌ NOT READY - Critical divergence must be resolved first

---

## Codebase Divergence Analysis

### Current Implementation Discovery

**What exists NOW that tasks don't account for:**

1. **Dimension Inference in from_env()** (config.rs:133-149)
   ```rust
   if explicit_dimension.is_none() && config.provider == Provider::Ollama {
       if let Some(inferred_dim) = infer_ollama_dimension(&config.model) {
           config.dimension = inferred_dim;
       }
   }
   ```
   - Tasks assume this doesn't exist
   - Would be duplicated by MPRSKL.1001

2. **Model Defaulting for Ollama** (config.rs:122-125)
   ```rust
   if config.provider == Provider::Ollama && config.model == "text-embedding-3-small" {
       config.model = "mxbai-embed-large".to_string();
   }
   ```
   - Not mentioned in tasks at all
   - Part of the actual fix

3. **Provider-Aware Endpoint Validation** (config.rs:209-249)
   - Large comment block about PROVFIX-1001
   - Related fix from different ticket
   - Tasks don't account for this context

**Critical Question:** Was the planning done against an OLDER version of the code? The analysis.md references specific line numbers (factory.rs:213, config.rs:133) that suggest the planning was accurate at the time but code has since evolved.

### Recommendation on Divergence

**Option A: UPDATE TASKS to match current code**
- Remove MPRSKL.1001 entirely (method not needed)
- Change MPRSKL.1002 to "Add integration test for existing zero-config Ollama workflow"
- Update all Phase 2-3 references to cite current implementation
- **Pros:** Tasks match reality, execution straightforward
- **Cons:** Abandons architectural improvement (provider propagation)

**Option B: KEEP TASKS but reframe as refactoring**
- Update MPRSKL.1001 to "Refactor dimension inference to use provider override pattern"
- Make it explicit this is IMPROVING existing fix, not creating it
- Tests verify BOTH old and new approach work
- **Pros:** Achieves cleaner architecture from architecture.md
- **Cons:** More work, risk of regression

**Option C: VERIFY THEN DECIDE**
- Add pre-flight task: "Test zero-config Ollama to verify current fix works"
- If works: Option A
- If broken: Option B (but with updated context)
- **Pros:** Data-driven decision
- **Cons:** Delays execution

**My Recommendation:** Option C - Verify first, then choose A or B based on reality

---

## Recommendations

### Before Proceeding (CRITICAL)

1. **VERIFY Current Implementation Works**
   ```bash
   # Test zero-config Ollama
   unset MAPROOM_EMBEDDING_PROVIDER
   unset MAPROOM_EMBEDDING_DIMENSION
   unset MAPROOM_EMBEDDING_MODEL
   # Ensure Ollama running at localhost:11434
   cargo run --bin crewchief-maproom -- scan --path /some/repo
   # Check: Does this correctly use dimension 1024? Or does bug still exist?
   ```

2. **UPDATE Planning Documents** (analysis.md, architecture.md, quality-strategy.md)
   - Reflect current codebase state
   - Note what's already implemented
   - Clarify if Phase 1 is fix vs. refactoring

3. **REVISE Task Dependencies**
   - Remove broken dependency chain if Phase 1 changes
   - Make Phase 2-3 reference current implementation, not planned Phase 1

### Task Revisions Needed

#### HIGH PRIORITY (Blocking)

1. **MPRSKL.1001**:
   - Add context about existing inference at config.rs:133-149
   - Reframe as "Add provider override parameter for cleaner architecture" OR remove entirely
   - Update acceptance criteria to not duplicate existing logic

2. **MPRSKL.1002**:
   - Verify whether factory actually propagates auto-detected provider to config
   - If yes: Make this "add integration test" task
   - If no: Keep task but update context ("bug persists despite inference code")

3. **MPRSKL.2003**:
   - Remove dependencies on 1001-1002
   - Update line 105 note to reference current implementation
   - Update dimension mismatch solutions to match config.rs behavior

4. **MPRSKL.3001**:
   - Remove dependencies on Phase 1
   - Simplify error enhancement to not require config in error variant
   - Update implementation notes to match current error handling

#### MEDIUM PRIORITY (Quality)

5. **MPRSKL.2001**:
   - Clarify whether this REPLACES existing SKILL.md or creates new file
   - Add verification step for command accuracy

6. **MPRSKL.2002**:
   - Note that env var documentation should match config.rs:209+ provider-aware endpoint validation
   - Add current dimension inference behavior to docs

7. **MPRSKL.3002**:
   - Choose ONE implementation approach (short help vs long_help)
   - Remove soft dependency on 2003 if not relevant

#### LOW PRIORITY (Polish)

8. **All tasks**:
   - Update file path references to use absolute paths
   - Add "read CLAUDE.md for component before starting" to technical requirements
   - Ensure verification commands are tested

### Risk Mitigations

1. **Divergence Risk**: Add pre-flight verification step to all Phase 1 tasks
2. **Documentation Accuracy**: All Phase 2 tasks must verify against actual CLI, not planned implementation
3. **Dependency Breakage**: Make Phase 2-3 reference current code, not Phase 1 tasks

---

## Alignment Assessment

**Scope Discipline:** ⚠️ Adequate
- Three-part scope (bug fix, docs, CLI) is still valid
- BUT implementation approach may differ from plan
- No scope creep detected

**Pragmatism:** ⚠️ Adequate
- Phase 2-3 are pragmatic (straightforward doc/CLI work)
- Phase 1 may be over-engineering if fix already exists
- Risk of creating redundant code paths

**Agent Compatibility:** ✅ Strong
- Task sizes appropriate (2-8 hours each)
- Clear acceptance criteria
- Good verification steps
- BUT agents may be confused by code/task mismatch

---

## Conclusion

**Recommendation:** NEEDS WORK - Resolve divergence before execution

**Success Probability:**
- **If tasks proceed as-is:** 40% (high risk of wasted work, confusion)
- **If divergence resolved first:** 85% (Phase 2-3 are solid, Phase 1 clarified)

**Risk Factors:**
- 60% risk Phase 1 tasks create redundant/conflicting code
- 40% risk documentation references wrong implementation
- 20% risk test strategy doesn't match actual code
- 5% risk of scope creep (low)

**Next Steps:**

**IMMEDIATE (Before any task execution):**
1. Run zero-config Ollama test to verify current behavior
2. Update analysis.md with findings (does bug exist or not?)
3. Decide: Fix vs. Refactoring vs. Skip Phase 1
4. Update architecture.md to match decision
5. Revise task dependencies and references

**THEN:**
1. If verification shows bug fixed: Remove MPRSKL.1001-1002, update MPRSKL.2003/3001 references, proceed with Phase 2-3
2. If verification shows bug persists: Update MPRSKL.1001-1002 context, keep all tasks but clarify existing vs. new code
3. If choosing refactoring path: Reframe MPRSKL.1001-1002 as architectural improvement, make backward compatibility explicit

**SUCCESS PATH:**
- Resolve Phase 1 divergence → Update doc task references → Execute Phase 2 (parallel) → Execute Phase 3 → Review

**This review intentionally identified problems NOW so they don't cause confusion during execution. The tasks are well-structured, but they're solving problems that may already be solved. Clarity on current state is essential before proceeding.**
