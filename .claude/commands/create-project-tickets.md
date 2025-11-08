---
argument-hint: [PROJECT_SLUG]
description: Create tickets for a project
---

# Project Context

Project: $ARGUMENTS
Project folder: .agents/projects/$ARGUMENTS-*/
Plan: .agents/projects/$ARGUMENTS-*/planning/{$ARGUMENTS}_PLAN.md
Output: .agents/projects/$ARGUMENTS-*/tickets/$ARGUMENTS-*

# Task

Use the ticket-creator agent to systematically create all tickets for project "$ARGUMENTS" based on the project plan and supporting documents.

**Preparation:**
1. Review .claude/agents/ticket-creator.md and .agents/reference/work-ticket-template.md to understand required inputs
2. Read all project documents in project folder
3. Identify phases, components, and dependencies from plan

**Ticket Naming Convention:**
$ARGUMENTS-{PHASE}{SEQUENTIAL}_{descriptive-name}.md

- Examples: $ARGUMENTS-1001_initial-setup.md, $ARGUMENTS-2003_implement-core.md
- Test tickets use 900s range: $ARGUMENTS-1901_test-critical-path.md

**Context to Provide ticket-creator:**

For each ticket, give ticket-creator:
- **Title & Summary**: Clear, action-oriented description
- **Background**: Problem being solved, how it fits the project, Design Thinking context
- **Acceptance Criteria**: 3-5 specific, measurable outcomes
- **Technical Requirements**: Concrete specs, constraints, DDD patterns
- **Implementation Notes**: Suggested approach, architecture, links to maproom docs
- **Dependencies**: Other tickets that must complete first
- **Risks**: Technical risks and mitigation strategies
- **Files/Packages**: Expected files to create/modify
- **Agent Assignments**: Primary and supporting agents (backend-engineer, ddd-expert, etc.)

**Test Strategy (MVP):**
- Only create tests for high-confidence critical paths
- Focus on core logic, integration points, complex transformations
- Skip exhaustive coverage; prioritize velocity

**Quality Gates:**
Before delegating each ticket, ensure:
- Specific acceptance criteria (not vague)
- Concrete technical requirements (not just "implement X")
- Appropriate agent assignments
- Realistic scope (2-8 hours)
- Relevant documentation links

**Output Organization:**
- Create ticket index: .agents/projects/$ARGUMENTS-*/tickets/$ARGUMENTS_TICKET_INDEX.md
- List all tickets organized by phase with references to plan

Work systematically through the plan, delegating rich context to ticket-creator for quality tickets.