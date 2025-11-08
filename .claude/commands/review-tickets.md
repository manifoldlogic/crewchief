---
argument-hint: [PROJECT_SLUG]
description: Comprehensive review of all created tickets for quality, consistency, and integration
---

# Project Context

Project: $ARGUMENTS
Project directory: `.agents/projects/$ARGUMENTS-*/`
Plan: `.agents/projects/$ARGUMENTS-*/planning/plan.md`
Architecture: `.agents/projects/$ARGUMENTS-*/planning/architecture.md`
Tickets: `.agents/projects/$ARGUMENTS-*/tickets/`
Output: `.agents/projects/$ARGUMENTS-*/planning/tickets-review-report.md`

# Task

Conduct thorough review of all tickets for project "$ARGUMENTS" to ensure quality, feasibility, and integration before execution begins.

## Preparation

1. **Load project context:**
   - Read plan.md for overall project objectives and phases
   - Review architecture.md for technical decisions and constraints
   - Check quality-strategy.md for testing approach
   - Review security-review.md for security considerations
   - Examine ticket index for full inventory

2. **Load repository context:**
   - Review existing codebase structure (if applicable)
   - Identify current functionality and working features
   - Note critical paths that must remain functional
   - Map existing dependencies and integrations

3. **Understand current state:**
   - Identify baseline functionality to preserve
   - Note any known technical debt or constraints
   - Review recent changes or active development areas

## Review Criteria

### Integration & Impact Assessment

**Codebase integration:**
- Will tickets properly integrate with existing code?
- Are current working features protected from breaking changes?
- Do tickets account for existing architectural patterns?
- Are shared dependencies and modules handled correctly?

**Cross-ticket coordination:**
- Do tickets work together cohesively?
- Are handoff points between tickets clear?
- Will parallel tickets conflict or duplicate work?
- Are integration points explicitly addressed?

**Dependency validation:**
- Are all ticket dependencies accurate and achievable?
- Do dependency chains make logical sense?
- Are circular dependencies absent?
- Can tickets execute in the specified order?

### Quality & Feasibility

**Scope assessment:**
- Is each ticket realistically scoped (2-8 hours)?
- Are tickets atomic with single clear purpose?
- Is complexity appropriate for assigned agents?
- Are any tickets too vague or too detailed?

**Requirements clarity:**
- Are acceptance criteria specific and measurable?
- Are technical requirements concrete and actionable?
- Do implementation notes provide sufficient guidance?
- Is success clearly defined for each ticket?

**Agent assignments:**
- Are appropriate agents assigned to each ticket?
- Do agent capabilities match ticket requirements?
- Are supporting agents identified where needed?
- Are any specialized agents missing?

**Testing coverage:**
- Does testing strategy align with quality-strategy.md?
- Are critical paths covered by test tickets?
- Is testing pragmatic for MVP (not exhaustive)?
- Are test tickets properly scoped and placed?

### Architecture & Consistency

**Architecture alignment:**
- Do tickets implement the planned architecture?
- Are architectural decisions reflected consistently?
- Will implementation maintain architectural integrity?
- Are any tickets diverging from architecture.md?

**Pattern consistency:**
- Do tickets follow consistent patterns across phases?
- Are naming conventions uniform?
- Is technical approach consistent?
- Do tickets reflect project conventions?

**Security considerations:**
- Do tickets address security concerns from security-review.md?
- Are security-sensitive tickets properly scoped?
- Is authentication/authorization handled appropriately?
- Are data protection requirements met?

### Completeness & Coverage

**Plan coverage:**
- Do tickets cover all plan.md deliverables?
- Are any planned features missing tickets?
- Are all phases adequately represented?
- Is the ticket set sufficient for project completion?

**Gap identification:**
- Are there unstated assumptions needing tickets?
- Are integration points all covered?
- Do tickets handle edge cases and error scenarios?
- Are deployment/operations considerations included?

## Review Process

Work systematically through all tickets:

1. **Phase-by-phase review:**
   - Review all Phase 1 tickets together for consistency
   - Check Phase 2 tickets build properly on Phase 1
   - Continue through all phases sequentially
   - Note cross-phase dependencies and integration points

2. **Critical path analysis:**
   - Identify tickets on critical path to MVP
   - Verify critical path is unblocked and achievable
   - Check for bottlenecks or serial dependencies
   - Ensure critical functionality protected

3. **Risk assessment:**
   - Flag tickets with high complexity or uncertainty
   - Identify tickets likely to impact working features
   - Note tickets requiring external dependencies
   - Mark tickets with significant technical risk

4. **Integration simulation:**
   - Mentally walk through ticket execution sequence
   - Identify potential integration issues
   - Check for race conditions or ordering problems
   - Verify handoffs between tickets work

## Categorize Findings

**Issues requiring action:**

**Critical Issues (must fix before execution):**
- Tickets that will break existing functionality
- Missing critical dependencies
- Scope problems making tickets unworkable
- Architecture misalignment causing failures
- Circular or impossible dependencies

**Warnings (should address):**
- Scope concerns (too large or too vague)
- Missing supporting agents
- Unclear acceptance criteria
- Potential integration conflicts
- Testing gaps in critical areas

**Recommendations (consider improvements):**
- Scope optimizations
- Better sequencing opportunities
- Additional test coverage suggestions
- Documentation enhancements
- Agent assignment refinements

## Report Structure

Create comprehensive report in `.agents/projects/$ARGUMENTS-*/planning/tickets-review-report.md`:

### Executive Summary
- Total tickets reviewed
- Overall assessment (ready/needs work/major issues)
- Critical issues count
- Key recommendations

### Critical Issues
For each critical issue:
- Ticket ID(s) affected
- Specific problem description
- Impact on project/existing functionality
- Required action to resolve
- Priority (block execution / must address)

### Warnings
For each warning:
- Ticket ID(s) affected
- Concern description
- Potential impact if unaddressed
- Suggested remediation

### Recommendations
For each recommendation:
- Area of improvement
- Affected tickets (if specific)
- Suggested enhancement
- Expected benefit

### Ticket Actions Required

**Tickets to rework:**
- List ticket IDs needing significant revision
- Specify required changes for each
- Note what makes them unworkable as-is

**Tickets to defer:**
- List ticket IDs to move to later phase/backlog
- Explain deferral reasoning
- Note dependencies or blockers

**Tickets to skip:**
- List ticket IDs to remove from project
- Explain why no longer needed
- Note any resulting gaps to address differently

**Tickets to split:**
- List ticket IDs that are too large
- Suggest split approach
- Identify logical boundaries

**Tickets to merge:**
- List ticket IDs that are too granular
- Suggest merge combinations
- Explain efficiency gains

### Integration Assessment
- Overall integration health
- Key integration points and status
- Risks to existing functionality
- Mitigation recommendations

### Dependency Analysis
- Dependency chain validation results
- Any problematic dependencies
- Sequencing recommendations
- Parallel execution opportunities

### Recommendations for Execution
- Suggested ticket execution order
- Risk mitigation strategies
- Key checkpoints during execution
- Success criteria for project completion

## Quality Standards

Before completing review:
- ✓ All tickets examined individually
- ✓ Cross-ticket interactions analyzed
- ✓ Integration with existing code assessed
- ✓ Dependencies validated
- ✓ Scope and feasibility checked
- ✓ Architecture alignment verified
- ✓ Critical issues clearly identified
- ✓ Actionable recommendations provided

## Output Expectations

Report should be:
- **Comprehensive:** Cover all tickets and key considerations
- **Specific:** Reference exact ticket IDs and concrete issues
- **Actionable:** Provide clear steps to address problems
- **Balanced:** Acknowledge good work while identifying issues
- **Risk-focused:** Prioritize issues that could break functionality

Conduct thorough, systematic review and produce detailed report with clear findings and actionable recommendations.