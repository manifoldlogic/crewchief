---
description: Systematically update project planning documents based on project-review findings to address critical issues, gaps, and recommendations
---

# Project Context

Project: [PROJECT_SLUG]
Project folder: `.agents/projects/[PROJECT_SLUG]-*/`
Review document: `.agents/projects/[PROJECT_SLUG]-*/planning/project-review.md`
Planning documents: `.agents/projects/[PROJECT_SLUG]-*/planning/`
Output: Updates to planning documents + `.agents/projects/[PROJECT_SLUG]-*/planning/review-updates.md`

# Task

Read the project review and systematically update planning documents to address all critical issues, high-risk areas, and recommendations. Work methodically to ensure project readiness.

## Update Priority

Address issues in this order:
1. **Critical Issues (Blockers)** - Must fix immediately
2. **Boundary Violations** - Fix improper integrations
3. **High-Risk Areas** - Should address to reduce project risk
4. **Gaps & Ambiguities** - Fill in missing information
5. **Scope & Feasibility** - Adjust scope for success
6. **Alignment Issues** - Improve MVP discipline and pragmatism
7. **Documentation Gaps** - Enhance clarity and completeness

## Preparation

1. **Read project-review.md thoroughly:**
   - Extract all critical issues with required actions
   - Note high-risk areas with mitigations
   - List gaps and ambiguities to fill
   - Identify scope adjustments needed
   - Review recommended actions

2. **Create tracking document:**
   Create `review-updates.md` to track all changes made

3. **Load current planning documents:**
   - analysis.md
   - architecture.md  
   - plan.md
   - quality-strategy.md
   - security-review.md
   - agent-suggestions.md (if exists)
   - README.md

## Update Methodology

### Phase 1: Critical Issue Resolution

For each critical issue from the review:

1. **Identify affected documents:**
   - Map issue to specific planning documents
   - Determine which sections need updates
   - Note dependencies between documents

2. **Implement required actions:**
   - Follow the specific "Required Action" from review
   - Make concrete, specific changes (not vague improvements)
   - Ensure changes align with project principles

3. **Maintain consistency:**
   - Update all affected documents
   - Ensure changes don't conflict with other sections
   - Keep technical decisions aligned across documents

4. **Document changes:**
   - Log what was changed in review-updates.md
   - Note why the change addresses the issue
   - Track which documents were modified

### Phase 2: Boundary Violation Fixes

For each boundary violation from the review:

1. **Identify improper integration:**
   - Note where direct function calls are used
   - Find tight coupling between components
   - Locate internal API usage

2. **Determine proper integration method:**
   - CLI for high-level orchestration
   - Public APIs for service communication
   - Library imports only for utilities
   - Binary execution for standalone operations

3. **Update architecture.md:**
   - Specify correct integration approach
   - Define public interfaces clearly
   - Document component boundaries
   - Add integration diagrams if helpful

4. **Update plan.md:**
   - Revise implementation approach
   - Specify integration method for each touchpoint
   - Update agent instructions for proper boundaries
   - Add verification that boundaries are maintained

5. **Document rationale:**
   - Explain why this integration method was chosen
   - Note benefits of proper separation
   - Identify what problems this prevents

### Phase 3: Risk Mitigation Implementation

For each high-risk area:

1. **Apply mitigation strategies:**
   - Implement suggested mitigations from review
   - Add risk management sections where missing
   - Define fallback approaches for dependencies

2. **Strengthen weak areas:**
   - Add specificity to vague requirements
   - Define concrete acceptance criteria
   - Clarify technical specifications

3. **Add contingency planning:**
   - Document rollback procedures
   - Define failure handling
   - Specify monitoring points

### Phase 4: Gap Filling

For each identified gap:

1. **Requirements gaps:**
   - Add missing requirements with specifics
   - Define measurable success criteria
   - Clarify ambiguous specifications

2. **Technical gaps:**
   - Make deferred decisions explicit
   - Add missing technical details
   - Specify integration points clearly

3. **Process gaps:**
   - Define missing workflows
   - Clarify agent handoffs
   - Add verification procedures

### Phase 5: Scope Optimization

Based on scope concerns:

1. **Trim scope creep:**
   - Move non-MVP features to "Future Phases" section
   - Remove unnecessary complexity
   - Focus on core value delivery

2. **Clarify boundaries:**
   - Define explicit out-of-scope items
   - Set clear phase boundaries
   - Specify MVP deliverables precisely

3. **Simplify approach:**
   - Replace complex solutions with pragmatic ones
   - Remove ceremonial processes
   - Focus on shipping working software

### Phase 6: Alignment Improvements

Address alignment issues:

1. **MVP discipline:**
   - Ensure Phase 1 delivers usable value
   - Remove features that don't serve immediate needs
   - Focus on "good enough" not perfect

2. **Pragmatism enhancement:**
   - Replace "best practices" with "fit for purpose"
   - Remove unnecessary abstractions
   - Simplify testing to confidence-building only

3. **Agent compatibility:**
   - Ensure tasks are 2-8 hour chunks
   - Make acceptance criteria bot-verifiable
   - Remove tasks requiring human judgment

## Document Update Patterns

### For analysis.md

**Common updates:**
- Add missing problem context
- Clarify current state assessment
- Define success metrics explicitly
- Add concrete examples
- Remove vague generalizations

### For architecture.md

**Common updates:**
- Make technology choices explicit with rationale
- Add concrete component specifications
- Define clear interfaces and contracts
- Specify error handling approaches
- Include performance requirements
- **Define component boundaries explicitly**
- **Specify public APIs vs internal implementations**
- **Document integration methods for each touchpoint**

### For plan.md

**Common updates:**
- Break vague deliverables into specific outputs
- Add concrete milestones with verification criteria
- Define clear phase boundaries
- Specify exact agent assignments
- Include dependency chains

### For quality-strategy.md

**Common updates:**
- Define specific test scenarios
- Set concrete coverage targets for critical paths
- Remove ceremonial testing requirements
- Add pragmatic verification approaches

### For security-review.md

**Common updates:**
- Identify specific vulnerabilities to address
- Add concrete mitigation steps
- Define security boundaries explicitly
- Remove paranoid over-engineering

## Review Updates Tracking Document

Create `review-updates.md` with this structure:

```markdown
# Project Review Updates

**Original Review Date:** {date from project-review.md}
**Updates Completed:** {current date}
**Update Status:** Complete | In Progress

## Critical Issues Addressed

### Issue 1: {Issue title from review}
**Original Problem:** {Brief description}
**Changes Made:**
- {Document}: {Specific change description}
- {Document}: {What was added/modified}
**Result:** Issue resolved - {how it's now fixed}

### Issue 2: {Continue for all critical issues}

## Boundary Violations Fixed

### Violation 1: {Component boundary violation}
**Original Problem:** Direct function calls to {component}
**Changes Made:**
- architecture.md: Changed to use {CLI/API/library} instead
- plan.md: Updated integration approach
**Result:** Proper separation maintained via {method}

### Violation 2: {Continue for all violations}

## High-Risk Mitigations Implemented

### Risk 1: {Risk title from review}
**Mitigation Applied:**
- {Document}: {Mitigation added}
- {Specific change made}
**Risk Level:** Reduced from {High} to {Medium/Low}

## Gaps Filled

### Requirements Gaps
- ✅ {Gap description} → Added to {document} as {specific addition}
- ✅ {Gap description} → Clarified in {document}

### Technical Gaps
- ✅ {Missing decision} → Decided: {specific decision} (documented in {document})
- ✅ {Missing spec} → Specified: {concrete specification}

## Scope Adjustments

### Removed from MVP
- {Feature} → Moved to Phase 3 (reason: {not critical for MVP})
- {Complexity} → Simplified to {simpler approach}

### Clarified Boundaries
- Phase 1 now explicitly: {concrete deliverable list}
- Out of scope: {explicit exclusions}

## Alignment Improvements

### MVP Discipline
- Reduced Phase 1 from {X} features to {Y} core features
- Focused on: {specific value delivery}

### Pragmatism
- Replaced {complex approach} with {simple solution}
- Removed {ceremonial process}

## Document Change Summary

### analysis.md
- Lines modified: ~{X}
- Key changes: {brief summary}

### architecture.md  
- Lines modified: ~{X}
- Key changes: {brief summary}

### plan.md
- Lines modified: ~{X}
- Key changes: {brief summary}

### quality-strategy.md
- Lines modified: ~{X}
- Key changes: {brief summary}

### security-review.md
- Lines modified: ~{X}
- Key changes: {brief summary}

## Verification

**Next Steps:**
1. Re-run `review-project [PROJECT_SLUG]` to verify improvements
2. Address any remaining issues
3. Proceed to `create-project-tickets [PROJECT_SLUG]` if review passes

**Success Metrics:**
- [ ] All critical issues resolved
- [ ] High-risk areas mitigated
- [ ] Requirements specific and measurable
- [ ] Scope appropriate for MVP
- [ ] Plan ready for ticket creation
```

## Update Execution Process

1. **Read and analyze** project-review.md completely
2. **Create** review-updates.md to track changes
3. **Work systematically** through each issue category
4. **Update documents** with specific, concrete improvements
5. **Maintain consistency** across all documents
6. **Document all changes** in review-updates.md
7. **Verify completeness** against review recommendations

## Quality Standards

**Every update must be:**
- **Specific:** No vague improvements - concrete changes only
- **Measurable:** Add metrics, counts, thresholds where applicable  
- **Consistent:** Changes align across all documents
- **Pragmatic:** Favor simple solutions over complex ones
- **Complete:** Address the entire issue, not partially

**Avoid these anti-patterns:**
- Making cosmetic changes that don't address core issues
- Adding complexity while trying to add clarity
- Creating new inconsistencies while fixing old ones
- Over-correcting into excessive detail
- Losing sight of MVP focus

## Success Criteria

Updates are complete when:
- [ ] All critical issues have been addressed
- [ ] All boundary violations have been fixed
- [ ] All high-risk areas have mitigations
- [ ] All identified gaps are filled
- [ ] Scope is appropriate for MVP
- [ ] Documents are consistent and complete
- [ ] Integration methods are properly specified
- [ ] Component boundaries are clearly defined
- [ ] Review-updates.md documents all changes
- [ ] Project is ready for ticket creation (or execution if tickets exist)

## Output Summary

After completing updates, provide concise summary:

```
📝 PROJECT UPDATES COMPLETE: {PROJECT_NAME}

✅ CRITICAL ISSUES RESOLVED: {X}/{X}
• {Most significant fix}
• {Second major fix}

🔧 BOUNDARY VIOLATIONS FIXED: {X}/{X}
• {Changed from direct calls to CLI interface}
• {Switched from internals to public API}

⚠️ RISKS MITIGATED: {X}/{X}  
• {Key mitigation implemented}
• {Second risk addressed}

🔧 GAPS FILLED: {X}/{X}
• {Major gap resolved}
• {Important clarification added}

📊 SCOPE OPTIMIZED:
• Removed: {X features/complexity}
• Clarified: {key boundaries}
• MVP focus: {core value delivery}

📁 DOCUMENTS UPDATED:
• analysis.md - {major change}
• architecture.md - {major change}
• plan.md - {major change}
• quality-strategy.md - {major change}
• security-review.md - {major change}

✨ KEY IMPROVEMENTS:
1. {Most impactful improvement}
2. {Second major improvement}
3. {Third significant change}

📋 NEXT STEP: Run `review-project {SLUG}` to verify all issues resolved

Full update log: .agents/projects/{SLUG}-*/planning/review-updates.md
```
