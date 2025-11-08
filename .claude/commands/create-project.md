---
argument-hint: [project description]
description: Create initial project documents based on analysis, design, and planning framework.
---

# Context

Based on user input: "$ARGUMENTS" (if provided and not just ""), and/or current conversation context

# Task

Create project in .agents/projects/:

1. Review .agents/reference/project-boundry-evaluation.md to determine if this should be multiple projects. If so, create each separately following this same process.

2. Generate PROJECT_NAME and SLUG (max 8 chars, unique, representative)

3. Create folder: `.agents/projects/{SLUG}_{project-name}/` with subdirectories `planning/` and `tickets/`

4. Generate documents in `planning/` subdirectory:

   **analysis.md**: Think deeply about problem space, existing industry solutions, current project state, and research to demonstrate full understanding.

   **architecture.md**: Think deeply about best solution. Avoid enterprise mindset - focus on MVP that performs well while showing good judgement about architecture and long-term maintainability.

   **quality-strategy.md**: Think deeply about testing strategy. Avoid enterprise mindset - use shrewd judgement for pragmatic MVP approach. Tests should provide confidence and prevent backtracking/rework, not be exhaustive or ceremonial. No tests for their own sake.

   **security-review.md**: Think deeply about architecture and security gaps. Consider enterprise expectations and mention them, but focus on shipping MVP without meaningful security concerns. Use shrewd judgement - cover bases pragmatically, avoid pitfalls, not perfectly iron-clad. No elite security signalling for show.

   **agent-suggestions.md** (if applicable): List undefined agents that would help complete this project, with brief descriptions.

   **plan.md**: Break into phases and deliverables based on architecture, testing strategy, and security review. Reference best-suited agents (existing + suggested). High-level only, not individual tickets.

5. Generate **README.md** in project root: Project summary with problem, solution, relevant agents, and links to planning documents.

Think sequentially and work systematically to thoroughly complete this large task.