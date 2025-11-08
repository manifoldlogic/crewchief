---
argument-hint: [PROJECT_SLUG]
description: Complete, verify, and commit tickets for a project
---

# Project Context

Project: $ARGUMENTS
Plan: .agents/projects/$ARGUMENTS-*/planning/plan.md
Tickets: .agents/projects/$ARGUMENTS-*/tickets/

# Task

Work through ALL tickets for project "$ARGUMENTS" systematically until complete. Employ appropriate agents as specified in the plan.

**Operating Principles:**
- Use project documents in .agents/projects/$ARGUMENTS-*/planning/ for context and guidance
- Search web for information as needed
- Work autonomously - do not ask for user input or permission
- Move forward confidently with conviction

**Workflow Rhythm:**
1. Use the command `/single-ticket <ticket-id>` to work on a ticket.
2. Move to the next ticket and repeat.

**Maintain this rhythm.** Do not break the flow. 1,2,1,2,1,2... until all tickets are complete.

**Escape Hatches (use sparingly):**
- Create follow-up ticket via ticket-creator agent if gaps identified
- Skip ticket with annotation explaining why (only if blocking issue)

Work through all tickets sequentially with focus and momentum.