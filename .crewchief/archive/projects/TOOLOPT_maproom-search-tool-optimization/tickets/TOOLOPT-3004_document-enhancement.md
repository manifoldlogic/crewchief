# Ticket: TOOLOPT-3004: Document enhancement rationale and testing recommendations

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation work)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Tests pass - N/A: This is documentation work with no code to test

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create documentation explaining the task-to-query mapping enhancement, expected impact, design rationale, and recommendations for future testing.

## Background
Enhancement represents hypothesis about breaking through the 19-20% plateau observed in genetic optimization runs. Need documentation to guide future genetic optimization runs and explain the reasoning behind variant-e-task-mapping.

This ticket implements the documentation phase of Phase 3 from the TOOLOPT project plan, completing the enhancement creation work.

## Acceptance Criteria
- [ ] Documentation created: `/workspace/docs/optimization/examples/variant-e-task-mapping.md`
- [ ] Contains all required sections:
  - Enhancement description and rationale
  - Task-to-query mapping section breakdown
  - Expected performance impact (+0.5-1.0%, target >20%)
  - Design decisions explained
  - Testing recommendations for next genetic run
  - Comparison with variant-a-detailed (what's new)
- [ ] Hypothesis clearly stated: task-to-query mapping addresses critical gap in task→strategy conversion
- [ ] Success metrics defined (>20.0% would confirm hypothesis)

## Technical Requirements
- Markdown format matching other optimization examples in `/workspace/docs/optimization/examples/`
- Side-by-side comparison showing what was added to variant-a-detailed
- Clear explanation of why this enhancement might break through the plateau
- Testing guidance for future genetic optimization runs
- Link to genetic optimization analysis that identified the gap
- Include concrete examples from the task-to-query section

## Implementation Notes
Key points to document:

**Gap Analysis:**
- Current tool descriptions teach "question→query transformation"
- Agents receive tasks like "Find where X is implemented" not questions
- Missing: systematic guidance for converting task types into search strategies
- This gap likely explains the 19-20% plateau

**Hypothesis:**
Teaching task→strategy mapping will improve agent performance by providing clearer guidance for common agent tasks.

**Expected Impact:**
- Current best: 19.6% (variant-a-detailed with transformation workflow)
- Target: 20.0-20.5% (transformation + task mapping)
- Delta: +0.5-1.0%
- Success threshold: >20.0% would confirm hypothesis

**Why This Matters:**
- Breaks through transformation-only plateau
- Addresses real agent behavior patterns
- Provides tactical guidance for common scenarios

**Testing Recommendation:**
Include variant-e-task-mapping in next genetic run with same benchmark task set to enable direct comparison.

Comparison structure:
```markdown
## What's New

**variant-a-detailed:** Question → Query transformation
**variant-e-task-mapping:** Task → Strategy → Query mapping

## Addition

The enhancement adds a task-to-query mapping section after the transformation workflow:

[Show task-to-query section content]

## Expected Impact

Current Performance: 19.6% (transformation workflow)
Target Performance: 20.0-20.5% (transformation + task mapping)
Expected Delta: +0.5-1.0%

Success Criteria: Performance >20.0% would validate the hypothesis that task-to-query mapping addresses a critical gap.
```

Reference related documentation:
- Link to genetic optimization results that showed plateau
- Link to variant-a-detailed example
- Link to optimization patterns guide

## Dependencies
- TOOLOPT-3002 (variant-e-task-mapping.json created)
- TOOLOPT-3003 (variant validated and confirmed working)

## Risk Assessment
- **Risk**: Documentation may not align with actual variant content if changes were made during validation
  - **Mitigation**: Review final variant-e-task-mapping.json before writing documentation
- **Risk**: Expected performance impact may be too optimistic
  - **Mitigation**: Clearly frame as hypothesis, not guaranteed outcome; define success metrics
- **Risk**: Testing recommendations may not account for framework changes
  - **Mitigation**: Review current genetic optimization framework capabilities before making recommendations
- **Risk**: Missing context for future developers about why this enhancement was created
  - **Mitigation**: Include thorough background on plateau analysis and gap identification

## Files/Packages Affected
- `/workspace/docs/optimization/examples/variant-e-task-mapping.md` (created)
- Potentially `/workspace/docs/optimization/examples/README.md` (update index if it exists)
