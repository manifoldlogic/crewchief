# Test Execution Enforcement - Process Improvements

**Date**: 2025-11-09
**Context**: Addressing failure to run integration tests in BRWATCH-2901

## Problem Summary

In ticket BRWATCH-2901, integration tests were created (`watcher_integration.rs`, 608 lines) but never executed. The "Tests pass" checkbox was marked without running `cargo test --test watcher_integration -- --ignored`, violating the ticket workflow template requirement.

This reduced confidence in BRWATCH implementation from 85-90% to 70-75%.

## Root Cause

**Ambiguous test requirements** in workflow documentation:
- "Tests pass - related tests pass" could mean "test files exist" OR "tests ran and passed"
- No explicit requirement to show test execution output
- verify-ticket agent didn't validate test execution, only file existence

## Changes Made

### 1. Updated `/workspace/.claude/commands/single-ticket.md`

**Added to Implementation checklist** (lines 43-44):
```markdown
- ✓ **If tests created/modified: Execute tests and capture output**
- ✓ **Fix any test failures before proceeding to verification**
```

**Added new section "Test Execution Requirements"** (lines 52-81):
- Specific commands for Rust, TypeScript, Python
- Required output format with examples
- Clear DO NOT list (❌ don't check "Tests pass" without running tests)

**Updated Quality Gates** (lines 85-86, 90):
```markdown
After Implementation:
- **Tests executed with output shown** (if tests created/modified)
- **All tests passing** (no failures, no skipped critical tests)

After Verification:
- **Test execution evidence validated** (output confirms tests ran and passed)
```

**Updated verification requirements** (line 59):
```markdown
- **Tests executed and passing** (requires test output evidence if tests created/modified)
```

### 2. Updated `/workspace/.claude/agents/ticket-workflow/verify-ticket.md`

**Added test detection step** (lines 24-27):
```markdown
4. **Determine if ticket involves testing:**
   - Check if tests were created/modified
   - Check if test files appear in git diff
   - Look for test-related acceptance criteria
```

**Added test execution validation** (lines 35-40):
```markdown
5. **Verify test execution evidence** (if tests created/modified):
   - Search conversation history for test execution commands
   - Look for test output (pass/fail counts, execution time)
   - Verify tests were actually RUN, not just created
   - Check for evidence like "cargo test", "pnpm test", "pytest" output
```

**Added new section "Step 3.5: Test Execution Validation (CRITICAL)"** (lines 53-95):
- Mandatory test execution verification for test-related tickets
- Explicit failure conditions:
  - ❌ "Tests pass" checked but NO test execution output found
  - ❌ Test files exist but no evidence they were run
  - ❌ Test output shows failures that weren't addressed
  - ❌ Tests marked `#[ignore]` weren't run with `--ignored` flag
- Examples of VALID vs INVALID test evidence

**Updated Common Oversights** (lines 124-125):
```markdown
- **"Tests pass" checked without test execution evidence (CRITICAL)**
- **Test files created but never run (CRITICAL)**
```

**Updated Failed Verification template** (lines 169-173):
```markdown
Test Execution Validation:
✗ [CRITICAL] "Tests pass" checked but no test execution evidence found
   - Test files created: tests/watcher_integration.rs
   - Required: Run `cargo test --test watcher_integration -- --ignored`
   - Missing: Test output showing pass/fail results
```

### 3. Updated `/workspace/.agents/reference/work-ticket-template.md`

**Changed "Tests pass" description** (line 9):
```markdown
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
```

**Added clarifying note** (lines 12-16):
```markdown
**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement
```

## Impact

These changes create **explicit multi-step test validation**:

### Before (Ambiguous)
```markdown
- [x] Tests pass - related tests pass
```
Could mean: "test files exist" OR "tests were run and passed"

### After (Explicit)
Implementation agent must:
1. Create/modify tests
2. **Execute tests** with appropriate command
3. **Capture output** showing pass/fail counts
4. **Report results** in completion message
5. Only then check "Tests pass"

Verify-ticket agent must:
1. Detect if ticket involves testing
2. **Search for test execution evidence** in conversation
3. **Validate test output** shows execution and passing
4. **FAIL verification** if evidence missing

## Prevention Mechanism

This creates a **three-layer enforcement**:

**Layer 1: Template clarity**
- Work ticket template explicitly states test execution requirement
- Examples show what valid evidence looks like

**Layer 2: Single-ticket command guidance**
- Implementation checklist includes test execution step
- Test Execution Requirements section provides specific commands
- Quality gates require test execution evidence

**Layer 3: Verify-ticket validation**
- Step 3.5 mandates test execution validation
- Agent searches for execution evidence
- Fails verification if evidence missing

## How This Prevents the BRWATCH-2901 Mistake

**My specific failure**:
- Created `watcher_integration.rs` (608 lines)
- Checked "Tests pass" without running tests
- Conflated test file existence with test execution

**How improvements prevent it**:

1. **Explicit steps**: Template now separates "tests created" from "tests executed" from "tests passing"
2. **Evidence requirement**: Cannot check "Tests passing" without showing test output
3. **Verify-ticket enforcement**: Agent will FAIL verification if evidence missing
4. **Clear examples**: Documentation shows exactly what's required

**Result**: Impossible to check "Tests passing" without showing test output demonstrating execution.

## Recommended Next Steps

1. **Apply to active projects**: Any in-progress tickets should follow new requirements
2. **Document in .agents/README.md**: Add "Test Execution Requirements" section
3. **Update agent prompts**: Ensure all implementation agents reference these requirements
4. **Training example**: Use BRWATCH-2901 as cautionary example in documentation

## Files Modified

1. `/workspace/.claude/commands/single-ticket.md` - Added test execution guidance
2. `/workspace/.claude/agents/ticket-workflow/verify-ticket.md` - Added test validation
3. `/workspace/.agents/reference/work-ticket-template.md` - Clarified test requirements

## Summary

The ambiguous "Tests pass" requirement has been replaced with explicit, multi-step test execution validation enforced at three levels: template, workflow, and verification. This prevents test file creation being confused with test execution and ensures all tests are run and validated before marking tickets complete.
