---
argument-hint: [PROJECT_SLUG]
description: Create tickets for a project based on the project plan and supporting documents
---

# Project Context

Project: $ARGUMENTS
Project folder: .agents/projects/$ARGUMENTS-*/
Plan: .agents/projects/$ARGUMENTS-*/planning/{$ARGUMENTS}_PLAN.md
Output: .agents/projects/$ARGUMENTS-*/tickets/$ARGUMENTS-*

# Task

Use the ticket-creator agent to systematically create all tickets for project "$ARGUMENTS" based on the project plan and supporting documents.

## Preparation

1. Review .claude/agents/ticket-creator.md and .agents/reference/work-ticket-template.md
2. Read all project documents in project folder
3. Identify phases, components, and dependencies from plan

## Ticket Naming Convention

Format: `$ARGUMENTS-{PHASE}{SEQUENTIAL}_{descriptive-name}.md`

**Phase-based numbering:**
- Phase 1 = 1xxx (e.g., $ARGUMENTS-1001_initial-setup.md)
- Phase 2 = 2xxx (e.g., $ARGUMENTS-2003_implement-core.md)
- Test tickets = x9xx (e.g., $ARGUMENTS-1901_test-critical-path.md)

## Context for ticket-creator

Provide rich context for each ticket:

**Title & Summary:** Clear, action-oriented description of the work

**Background:** Problem being solved, project context, Design Thinking alignment

**Acceptance Criteria:** 3-5 specific, measurable outcomes that define done

**Technical Requirements:** Concrete specs, constraints, DDD patterns, architecture decisions

**Implementation Notes:** Suggested approach, key considerations, relevant documentation links

**Dependencies:** Other tickets that must complete first (with ticket IDs)

**Risks:** Technical risks and mitigation strategies

**Files/Packages:** Expected files to create or modify

**Agent Assignments:** Primary and supporting agents needed (backend-engineer, ddd-expert, etc.)

## Test Strategy (MVP)

Focus on high-confidence critical paths only:
- Core business logic and integration points
- Complex transformations and edge cases
- Skip exhaustive coverage to maintain velocity

## Quality Gates

Before delegating each ticket to ticket-creator, verify:
- ✓ Acceptance criteria are specific and measurable (not vague)
- ✓ Technical requirements are concrete (not just "implement X")
- ✓ Agent assignments are appropriate for the work
- ✓ Scope is realistic (2-8 hours of work)
- ✓ Documentation links are relevant and accessible
- ✓ Dependencies are clearly identified

## Output Organization

Create ticket index: `.agents/projects/$ARGUMENTS-*/tickets/$ARGUMENTS_TICKET_INDEX.md`
- List all tickets organized by phase
- Include ticket IDs, titles, and status
- Reference plan sections for traceability

Work systematically through the plan, providing rich context to ticket-creator for each ticket to ensure quality and completeness.