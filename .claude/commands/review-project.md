---
argument-hint: [PROJECT_SLUG]
description: Critical review of project readiness, execution risks, and alignment with development principles. Run BEFORE ticket creation to catch issues early, or after for complete assessment.
---

# Project Context

Project: $ARGUMENTS
Project folder: `.agents/projects/$ARGUMENTS-*/`
Planning documents: `.agents/projects/$ARGUMENTS-*/planning/`
Tickets (if created): `.agents/projects/$ARGUMENTS-*/tickets/`
Output: `.agents/projects/$ARGUMENTS-*/planning/project-review.md`

# Task

Conduct a comprehensive critical review of project "$ARGUMENTS" to identify gaps, risks, and execution concerns. This review should be run BEFORE creating tickets to catch issues early, but can also be run after ticket creation for a more complete assessment.

## Optimal Workflow Positioning

**Recommended sequence:**
1. `/create-project` → Creates initial planning docs
2. **`/review-project`** → Critical evaluation of plan (BEFORE ticket creation)
3. `/create-project-tickets` → Generate tickets (if review passes)
4. `/review-project` → Re-run for complete assessment (optional)
5. `/review-tickets` → Quality check of created tickets
6. `/work-on-project` → Execute all tickets

**Key insight:** Running this BEFORE ticket creation saves significant effort by catching issues in the planning phase.

## Review Perspective

You are a senior technical architect and project risk assessor with deep experience in:
- Identifying projects that will fail or spiral out of control
- Spotting vague requirements that lead to endless rework
- Recognizing overengineering and scope creep
- Detecting misalignment with stated principles
- Finding hidden complexity and unstated dependencies

**Your mandate:** Be constructively critical. Find the problems NOW, not after weeks of wasted effort.

## Preparation

1. **Load all project documentation:**
   - analysis.md - Problem understanding
   - architecture.md - Solution design
   - plan.md - Execution approach
   - quality-strategy.md - Testing approach
   - security-review.md - Security considerations
   - agent-suggestions.md - Agent requirements (if exists)
   - README.md - Project overview

2. **Check for tickets (optional):**
   - If `tickets/` directory exists, review created tickets
   - If no tickets yet, focus on planning document quality
   - Note: Review is MORE valuable before tickets exist

3. **Understand project principles:**
   - MVP-focused development (ship value, not ceremonies)
   - Pragmatic over enterprise (avoid bloat)
   - AI agent-sized work chunks (2-8 hour tasks)
   - Test for confidence, not coverage
   - Complete-verify-commit rhythm

4. **Analyze existing repository (CRITICAL):**
   - **Available tools/utilities:** 
     - Check `/src/utils`, `/lib`, `/tools` for existing functionality
     - Review helper functions, shared libraries, common utilities
     - Identify data transformation, validation, formatting tools
   - **Existing patterns:**
     - Review architectural patterns in use (MVC, hexagonal, etc.)
     - Check error handling patterns and conventions
     - Understand logging and monitoring approaches
   - **Shared components:**
     - Identify reusable modules, services, middleware
     - Check for existing authentication, caching, rate limiting
     - Review data access layers and ORM usage
   - **Integration capabilities:**
     - Map current APIs, webhooks, event systems
     - Check existing database connections and schemas
     - Review message queues, job processors
   - **Development infrastructure:**
     - Test utilities, fixtures, mocks, test data generators
     - Build scripts, deployment tools, CI/CD pipelines
     - Development environment setup and tooling
   - **Domain implementations:**
     - Check if similar problems already solved
     - Review business logic implementations
     - Identify domain models and services

5. **Inventory reuse opportunities:**
   - What problems are already solved that this project needs?
   - Which utilities/helpers should be used instead of rebuilt?
   - What patterns must be followed for consistency?
   - Which existing integrations can be leveraged?
   - What would be wasteful duplication?

## Critical Review Dimensions

### 1. Codebase Integration & Reuse (CRITICAL)

**Reinvention detection:**
- Is the project rebuilding functionality that already exists?
- Are we creating new utilities when existing ones would work?
- Is the project ignoring established patterns in the codebase?
- Are we duplicating integration code that's already available?

**Existing tool leverage:**
- Which existing tools/libraries solve parts of this problem?
- Are all reusable components identified in the plan?
- Does the architecture build on existing infrastructure?
- Are we using existing test utilities and patterns?

**Pattern consistency:**
- Does the approach match existing architectural patterns?
- Are we following established error handling conventions?
- Is the data model consistent with existing schemas?
- Do API designs follow current patterns?

**Integration assessment:**
- Will this integrate cleanly with existing systems?
- Are we using existing authentication/authorization?
- Can we reuse existing configuration management?
- Are database connections and pools being shared appropriately?

**Duplication audit:**
- List any proposed functionality that duplicates existing code
- Identify where existing solutions should be used instead
- Note any justified reasons for not reusing (if valid)
- Calculate wasted effort from unnecessary duplication

### 2. Scope & Feasibility Analysis

**Scope creep detection:**
- Is the project trying to solve too many problems at once?
- Are there features that could be separate projects?
- Is Phase 1 truly an MVP or is it overloaded?
- Are "nice to have" features masquerading as requirements?

**Feasibility assessment:**
- Can the stated goals actually be achieved with current approach?
- Are the technical choices appropriate for the problem?
- Is the timeline realistic given the complexity?
- Are resource requirements (agents, tools) available?

**Hidden complexity:**
- What unstated assumptions are being made?
- What integration points haven't been considered?
- What edge cases will explode the scope?
- Where will the "unknown unknowns" emerge?

### 3. Requirements & Specification Quality

**Vagueness audit:**
- Which requirements are too abstract to implement?
- Where are acceptance criteria unmeasurable?
- What success metrics are undefined?
- Which technical specs lack concrete details?

**Existing solutions check:**
- Has the team checked if this requirement is already met elsewhere?
- Could existing functionality be extended instead of rebuilt?
- Are requirements aware of current system capabilities?
- Do specs reference and build on existing components?

**Pre-ticket specific checks:**
- Can each phase deliverable be broken into specific tasks?
- Are requirements detailed enough to write acceptance criteria?
- Do technical specifications enable ticket creation?
- Is there sufficient detail for 2-8 hour work chunks?

**Post-ticket validation (if tickets exist):**
- Do tickets accurately reflect plan requirements?
- Are all plan deliverables covered by tickets?
- Have requirements been properly decomposed?
- Are ticket acceptance criteria measurable?

**Completeness check:**
- What's missing from the analysis that will block execution?
- Which architectural decisions are deferred or unclear?
- What dependencies are unstated or assumed?
- Where are the gaps between phases?

**Consistency validation:**
- Do all documents tell the same story?
- Are there conflicting technical decisions?
- Is the plan aligned with the architecture?
- Do security/quality strategies match the design?

### 4. Execution Readiness

**Planning completeness:**
- Is the plan detailed enough to create tickets from?
- Are phases clearly defined with deliverables?
- Can work be decomposed into 2-8 hour chunks?
- Are there enough specifics to write acceptance criteria?

**Integration planning:**
- Are integration points with existing systems identified?
- Do we have a plan for using existing tools/libraries?
- Is there a clear migration/deployment strategy?
- How will this coexist with current functionality?

**Agent capability matching:**
- Do assigned agents have the skills needed?
- Are specialized agents defined but not available?
- Will agents understand the requirements as written?
- Are handoffs between agents clear?

**Dependency analysis:**
- Are external dependencies identified and accessible?
- Do phase dependencies create bottlenecks?
- Are there circular or impossible dependencies?
- What happens if a dependency fails?

**Ticket readiness (if tickets exist):**
- Do tickets match the plan's intent?
- Are ticket scopes appropriate?
- Is coverage complete across phases?
- Are dependencies properly sequenced?

**Risk identification:**
- What are the top 5 things likely to go wrong?
- Where are the technical risks highest?
- What could cause project abandonment?
- Which assumptions are most fragile?

### 5. Principle Alignment

**MVP discipline:**
- Is this truly minimum viable or minimum marketable?
- Can we ship something useful after Phase 1?
- Are we building for current needs or imagined futures?
- Is each phase independently valuable?

**Pragmatism check:**
- Are we overengineering for problems we don't have?
- Is the testing strategy appropriate (not ceremonial)?
- Are we adding complexity for "best practices" sake?
- Would a simpler solution work just as well?

**AI agent compatibility:**
- Are tasks sized for autonomous completion (2-8 hours)?
- Can agents work independently with clear boundaries?
- Are verification criteria explicit and testable?
- Do tasks avoid requiring human judgment or creativity?

### 6. Integration & Maintenance

**System integration:**
- How will this integrate with existing systems?
- What existing functionality might be affected?
- Are we respecting current system boundaries?
- Can we leverage existing integration patterns?
- What will break when this is deployed?

**Reuse vs rebuild:**
- Are we using existing authentication/authorization systems?
- Can we extend existing APIs rather than create new ones?
- Should we use existing data models and schemas?
- Are there existing services that solve parts of this?

**Compatibility requirements:**
- Are backwards compatibility requirements addressed?
- Will existing tools and scripts continue to work?
- How do we maintain existing integrations?
- Can existing clients/consumers adapt easily?

**Maintenance burden:**
- What technical debt are we creating?
- How maintainable is the proposed architecture?
- Are we creating future migration problems?
- What ongoing operational costs are implied?
- Will this increase or decrease overall system complexity?

### 7. Documentation & Knowledge

**Documentation quality:**
- Is documentation sufficient for agents to execute?
- Are technical decisions explained with rationale?
- Can someone understand the project in 6 months?
- Are examples and references provided where needed?

**Existing system documentation:**
- Are dependencies on existing systems documented?
- Do docs explain how this fits into the current architecture?
- Are integration points clearly described?
- Is reuse of existing components documented?

**Knowledge gaps:**
- What domain knowledge is assumed but not documented?
- Which technical areas need more research?
- Where do we lack expertise to evaluate properly?
- What industry patterns should we consider?
- What existing internal patterns should be referenced?

## Review Methodology

### Phase 1: Codebase Analysis
Before reviewing project docs, understand what already exists:
- Scan repository for similar functionality
- Identify reusable components and utilities
- Map existing integrations and patterns
- Note tools and libraries in use

### Phase 2: Document Analysis
Read all planning documents critically, noting:
- Vague language ("implement properly", "handle appropriately")
- Missing specifics (no concrete acceptance criteria)
- Conflicting statements between documents
- Unstated assumptions
- Optimistic estimates
- **Reinvention indicators** (building what already exists)
- **Missed reuse opportunities** (not leveraging existing tools)

**Key focus areas when tickets don't exist:**
- Is the plan specific enough to generate tickets?
- Are there clear work boundaries defined?
- Can acceptance criteria be derived from requirements?
- Are technical specifications concrete enough to implement?
- **Does the plan acknowledge and build on existing systems?**

### Phase 3: Cross-Reference Validation
### Phase 3: Cross-Reference Validation
Compare documents against each other AND existing codebase:
- Does plan match architecture?
- Are all architectural components in the plan?
- Do test strategies cover critical paths?
- Are security concerns addressed in implementation?
- **Does architecture align with existing patterns?**
- **Are proposed solutions consistent with current approaches?**

### Phase 4: Integration Analysis
Specifically evaluate integration with existing systems:
- Which existing components can be reused?
- What current functionality will be affected?
- Are there conflicts with existing patterns?
- Can existing tools solve proposed problems?
- Where is duplication being introduced?

### Phase 5: Execution Simulation
Mentally simulate project execution:
- **Without tickets:** Can you decompose the plan into specific tasks?
- **With tickets:** Do tickets cover all plan requirements?
- Walk through each phase as an agent would
- Identify where agents would get stuck
- Find missing information or decisions
- Spot integration problems
- **Check if agents would rebuild existing functionality**

### Phase 6: Risk Assessment
Evaluate project risks systematically:
- Technical risks (complexity, unknowns)
- Execution risks (dependencies, resources)
- Quality risks (testing gaps, verification issues)
- Maintenance risks (technical debt, operational burden)
- **Integration risks** (breaking existing functionality, incompatibility)
- **Duplication risks** (wasted effort on existing solutions)
- **Planning risks** (if pre-ticket: risk that tickets can't be properly created)

## Review Output Structure

Create comprehensive review in `.agents/projects/$ARGUMENTS-*/planning/project-review.md`:

```markdown
# Project Review: {PROJECT_NAME}

**Review Date:** {date}
**Project Status:** {Not Ready | Needs Work | Proceed with Caution | Ready}
**Overall Risk:** {Low | Medium | High | Critical}

## Executive Summary

{2-3 paragraph assessment of project readiness, major concerns, and recommendation}

## Critical Issues (Blockers)

Issues that MUST be resolved before proceeding:

### Issue 1: {Title}
**Severity:** Critical
**Category:** {Scope|Requirements|Architecture|Execution|Integration|Duplication}
**Description:** {Specific problem description}
**Impact:** {What happens if not addressed}
**Required Action:** {Concrete steps to resolve}
**Documents Affected:** {List of planning docs needing update}

### Issue 2: {Continue for all critical issues}

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds
**Existing Solution:** {What already exists}
**Proposed Duplication:** {What project plans to rebuild}
**Wasted Effort:** {Estimated hours/days of unnecessary work}
**Recommendation:** {Use existing solution X instead}

### Missed Reuse Opportunities
**Available Component:** {Existing tool/library/service}
**Could Solve:** {What problem it addresses}
**Integration Effort:** {Low|Medium|High}
**Recommendation:** {How to leverage it}

### Pattern Violations
**Existing Pattern:** {Current approach in codebase}
**Proposed Deviation:** {How project differs}
**Consistency Impact:** {Why this matters}
**Recommendation:** {Follow existing pattern or justify deviation}

## High-Risk Areas (Warnings)

Significant concerns that should be addressed:

### Risk 1: {Title}
**Risk Level:** High
**Category:** {Technical|Execution|Maintenance|Integration}
**Description:** {Specific risk description}
**Probability:** {Low|Medium|High}
**Impact:** {Low|Medium|High}
**Mitigation:** {Suggested risk mitigation approach}

### Risk 2: {Continue for all high-risk areas}

## Gaps & Ambiguities

### Requirements Gaps
- {Specific missing requirement or vague specification}
- {Impact on execution}
- {Suggested clarification}

### Technical Gaps
- {Missing technical decision or specification}
- {Blocking tickets or phases}
- {Required research or decision}

### Process Gaps
- {Missing workflow or handoff definition}
- {Impact on agent execution}
- {Suggested process definition}

## Scope & Feasibility Concerns

### Scope Creep Indicators
- {Feature or requirement that expands scope}
- {Suggested deferral or removal}
- {Impact on MVP delivery}

### Feasibility Challenges
- {Technical challenge or complexity}
- {Resource or capability concern}
- {Alternative approach suggestion}

## Alignment Assessment

### MVP Discipline
**Rating:** {Strong|Adequate|Weak|Failing}
- {Specific observation about MVP focus}
- {Areas of overengineering or bloat}
- {Recommendations for simplification}

### Pragmatism Score
**Rating:** {Strong|Adequate|Weak|Failing}
- {Assessment of pragmatic vs ceremonial approach}
- {Unnecessary complexity identified}
- {Simplification opportunities}

### Agent Compatibility
**Rating:** {Strong|Adequate|Weak|Failing}
- {Task sizing and boundaries assessment}
- {Agent capability matching}
- {Automation feasibility}

## Execution Readiness Checklist

### Documentation
- [ ] Requirements are specific and measurable
- [ ] Architecture decisions are clear and justified
- [ ] Plan has concrete milestones and deliverables
- [ ] Plan is detailed enough to create tickets from (if pre-ticket)
- [ ] Test strategy is defined and pragmatic
- [ ] Security concerns are addressed
- [ ] Dependencies on existing systems documented

### Technical
- [ ] Technology choices are appropriate
- [ ] Dependencies are identified and available
- [ ] Integration points are well-defined
- [ ] Performance requirements are clear
- [ ] Error handling is specified
- [ ] Existing tools/libraries identified for reuse
- [ ] No unnecessary duplication of functionality

### Process
- [ ] Agent assignments are appropriate (or determinable)
- [ ] Task boundaries are clear (or can be derived)
- [ ] Verification criteria are explicit (or definable)
- [ ] Handoffs are defined
- [ ] Rollback plan exists
- [ ] Integration with existing workflows considered

### Integration & Reuse
- [ ] Existing solutions evaluated before building new
- [ ] Current patterns and conventions followed
- [ ] Reusable components identified
- [ ] Integration points with existing systems mapped
- [ ] No reinvention of available functionality

### Tickets (if they exist)
- [ ] Tickets align with plan objectives
- [ ] All plan deliverables have corresponding tickets
- [ ] Dependencies are properly sequenced
- [ ] Scope per ticket is appropriate (2-8 hours)
- [ ] Acceptance criteria are measurable

### Risk
- [ ] Major risks are identified
- [ ] Mitigation strategies exist
- [ ] Dependencies have fallbacks
- [ ] Critical path is protected
- [ ] Failure modes are understood

## Recommendations

### Immediate Actions (Before Starting)
1. {Specific action with clear outcome}
2. {Document to update or decision to make}
3. {Gap to fill or clarification needed}

### Phase 1 Adjustments
- {Scope adjustment to ensure MVP delivery}
- {Requirement clarification needed}
- {Technical decision required}

### Risk Mitigations
- {Specific risk mitigation to implement}
- {Monitoring or checkpoint to add}
- {Contingency plan to develop}

### Documentation Updates
- {Document}: {Specific updates needed}
- {Document}: {Sections to clarify or expand}

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** {Yes with caveats | No without changes | High risk of failure}

**Primary concerns:**
1. {Top concern affecting success}
2. {Second major concern}
3. {Third significant issue}

### Recommended Path Forward

{One of the following}:

**PROCEED:** Project is well-defined and ready for execution with minor adjustments.

**REVISE THEN PROCEED:** Address critical issues and high-risk items before starting execution.

**SIGNIFICANT REWORK:** Project requires major revision to planning documents before it's executable.

**RECONSIDER:** Project scope, approach, or feasibility needs fundamental reconsideration.

### Success Probability
Given current state: {percentage}%
After recommended changes: {percentage}%

### Final Notes
{Any additional observations, suggestions, or concerns not covered above}
```

## High-Level Summary Output

After writing the review document, provide a concise summary in the conversation:

```
📋 PROJECT REVIEW COMPLETE: {PROJECT_NAME}

Status: {Not Ready | Needs Work | Proceed with Caution | Ready}
Risk Level: {Low | Medium | High | Critical}
Tickets Created: {Yes - X tickets | No - Pre-ticket review}

🔄 REINVENTION ISSUES ({count}):
• {Component being rebuilt unnecessarily}
• {Tool being duplicated}
• {Pattern being ignored}

🚨 CRITICAL ISSUES ({count}):
• {Most severe issue - brief description}
• {Second critical issue}
• {Additional critical issues}

⚠️ HIGH-RISK AREAS ({count}):
• {Top risk area}
• {Second risk area}
• {Additional significant risks}

📊 ALIGNMENT SCORES:
• MVP Discipline: {Strong|Adequate|Weak|Failing}
• Pragmatism: {Strong|Adequate|Weak|Failing}  
• Agent Compatibility: {Strong|Adequate|Weak|Failing}
• Codebase Integration: {Strong|Adequate|Weak|Failing}

🔍 KEY GAPS IDENTIFIED:
• {Most significant gap}
• {Second major gap}
• {Additional important gaps}

🎯 REUSE OPPORTUNITIES:
• {Existing tool that should be used}
• {Component that can be leveraged}
• {Pattern to follow}

✅ RECOMMENDED ACTION: {Proceed | Revise Then Proceed | Significant Rework | Reconsider}

📈 SUCCESS PROBABILITY:
• Current state: {X}%
• With recommended changes: {Y}%

🎯 TOP 3 ACTIONS BEFORE {CREATING TICKETS|PROCEEDING}:
1. {Most important action}
2. {Second priority action}
3. {Third key action}

Full review available at: .agents/projects/{SLUG}-*/planning/project-review.md
```

## Review Standards

**Be specific:** Point to exact files, sections, and line items. No generic "improve documentation" feedback.

**Be actionable:** Every issue needs concrete steps to resolve. No problems without solutions.

**Be honest:** Don't sugarcoat problems. Better to catch them now than after failed execution.

**Be constructive:** Critical but not destructive. Focus on making the project succeed.

**Be thorough:** Check everything. The issue you miss is the one that will cause failure.

Execute comprehensive review and deliver both detailed document and high-level summary.