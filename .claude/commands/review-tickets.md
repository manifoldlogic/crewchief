---
argument-hint: [PROJECT_SLUG]
description: Review the created tickets
---

# Project Context

Project: $ARGUMENTS
Project directory: .agents/projects/{$ARGUMENTS}-*/
Tickets: .agents/projects/{$ARGUMENTS}-*/tickets/

# Task

Now that you've created the $ARGUMENTS tickets, carefully review all the tickets, make sure they are taking the whole repo into account and will not break working functionality. Create a report containing any problems you find and any tickets that need to be reworked, deferred, or skipped.

# Output

Report: .agents/projects/{$ARGUMENTS}-*/planning/tickets-review-report.md