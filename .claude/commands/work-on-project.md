---
argument-hint: [PROJECT_SLUG]
description: Complete all tickets for a project systematically from start to finish
---

# Project Context

Project: $ARGUMENTS
Project folder: `.agents/projects/$ARGUMENTS-*/`
Plan: `.agents/projects/$ARGUMENTS-*/planning/plan.md`
Tickets: `.agents/projects/$ARGUMENTS-*/tickets/`

# Task

Execute all tickets for project "$ARGUMENTS" systematically using autonomous workflow until project completion.

## Preparation

1. **Review project context:**
   - Read plan.md to understand phases and overall approach
   - Review architecture.md for technical context
   - Check quality-strategy.md for testing approach
   - Note agent assignments and dependencies

2. **Load ticket inventory:**
   - Read ticket index: `$ARGUMENTS_TICKET_INDEX.md`
   - Identify all tickets organized by phase
   - Note dependencies between tickets
   - Plan execution sequence

## Execution Workflow

**Autonomous operation:** Work independently without requesting user input or permission. Make decisions confidently based on project documentation.

**Execution rhythm:**
```
1. Execute: /complete-ticket <ticket-id>
2. Move to next ticket
3. Repeat until all tickets complete
```

**Maintain momentum:** 1→2→1→2→1→2... Keep this rhythm flowing through all phases.

## Ticket Sequencing

**Phase-based progression:**
- Complete Phase 1 (1xxx) tickets before Phase 2 (2xxx)
- Within each phase, follow dependency order
- Test tickets (x9xx) after implementation tickets in same phase

**Dependency handling:**
- Check ticket dependencies before starting
- If dependency incomplete, skip to next available ticket
- Return to skipped tickets after dependencies satisfied

## Resource Usage

**Project documentation:** Use planning documents for context and decision guidance

**External research:** Search web for technical information, best practices, or current solutions as needed

**Agent expertise:** Leverage specialized agents as specified in tickets and plan

## Decision Framework

**When to create follow-up ticket:**
- Gap discovered that's out of scope for current ticket
- Enhancement opportunity identified during implementation
- Technical debt that should be tracked but not block current work
- Integration point needing separate ticket

**When to skip ticket:**
- Blocking dependency failure (annotate reason, plan retry)
- Fundamental requirement change making ticket obsolete
- External blocker beyond project control
- **Use sparingly** - skipping breaks momentum

**When to modify approach:**
- Implementation reveals better solution aligned with architecture
- Testing strategy shows different approach more pragmatic
- Security consideration requires adjustment
- Always document reasoning in commit or ticket notes

## Progress Tracking

After each ticket completion:
- Note ticket ID and status in mental model
- Track any follow-up tickets created
- Monitor for patterns suggesting plan adjustments
- Maintain awareness of remaining work

**Checkpoint after each phase:**
- Verify all phase tickets complete or accounted for
- Review phase deliverables against plan
- Note any adjustments needed for next phase
- Confirm ready to proceed

## Quality Assurance

**Per-ticket quality gates:**
- Each ticket passes verify-ticket checks
- All acceptance criteria met
- Clean commits with proper messages

**Project-level quality:**
- Maintain consistency across tickets
- Follow project conventions throughout
- Ensure integration between components
- Keep architecture vision intact

## Contingency Handling

**If ticket fails verification:**
- Address issues immediately
- Re-verify after fixes
- Do not move forward until passing

**If blocking issue arises:**
- Document issue clearly with specifics
- **Do not compromise requirements or use workarounds**
- Create follow-up ticket to properly address the block
- Annotate current ticket with block reason and follow-up reference
- Skip current ticket, proceed with other work

**If plan requires adjustment:**
- Note specific issue with current plan
- Make minimal necessary adjustment
- Document change reasoning
- Continue execution with updated approach

## Completion Criteria

Project complete when:
- ✓ All tickets in all phases completed or explicitly resolved
- ✓ All test tickets passing
- ✓ No orphaned changes or uncommitted work
- ✓ Follow-up tickets documented in backlog
- ✓ Project deliverables match plan objectives

## Output

Provide completion summary:
- Total tickets completed
- Phases delivered
- Follow-up tickets created
- Key accomplishments
- Any outstanding items or recommendations

Work systematically with focus and momentum through all tickets to project completion.