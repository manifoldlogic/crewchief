---
argument-hint: [project description]
description: Create initial project documents based on analysis, design, and planning framework
---

# Context

User input: "$ARGUMENTS"
Additional context from current conversation as available

# Task

Create comprehensive project structure in `.agents/projects/` following analysis → design → planning workflow.

## Project Setup

1. **Evaluate scope:** Review .agents/reference/project-boundry-evaluation.md to determine if this should be multiple projects. If so, create each separately.

2. **Generate identifiers:**
   - PROJECT_NAME: Descriptive, clear name
   - SLUG: Max 8 characters, unique, representative

3. **Create structure:** `.agents/projects/{SLUG}_{project-name}/`
   - `planning/` subdirectory for all planning documents
   - `tickets/` subdirectory for future tickets
   - `README.md` in project root

## Planning Documents

Generate in `planning/` subdirectory:

### analysis.md
Deep understanding of problem space:
- Problem definition and context
- Existing industry solutions and approaches
- Current project state (if applicable)
- Research findings and insights

### architecture.md
MVP-focused solution design:
- Architecture decisions and rationale
- Technology choices and constraints
- Performance considerations
- Long-term maintainability without over-engineering
- Focus on shipping value, not enterprise complexity

### quality-strategy.md
Pragmatic testing approach:
- Test strategy focused on confidence, not coverage
- Critical paths and integration points
- Risk mitigation through targeted testing
- MVP mindset: tests prevent rework, not ceremonial checkboxes

### security-review.md
Practical security assessment:
- Architecture security analysis
- Known gaps and risk evaluation
- MVP-appropriate mitigations
- Enterprise considerations mentioned, not implemented exhaustively
- Ship without meaningful security concerns

### agent-suggestions.md (if needed)
Undefined agents that would benefit this project:
- Agent name and brief description
- Specific capabilities needed
- How they fit the workflow

### plan.md
High-level execution plan:
- Phases and deliverables based on architecture
- Testing milestones from quality strategy
- Security checkpoints from security review
- Agent assignments (existing + suggested)
- Phase-based organization (not individual tickets)

## Project README

Create `README.md` in project root:
- Project summary and objectives
- Problem statement and proposed solution
- Relevant agents for execution
- Links to all planning documents

## Execution

Work systematically through document creation:
1. Start with analysis to establish understanding
2. Design architecture based on analysis insights
3. Define quality and security strategies aligned with architecture
4. Identify agent needs
5. Create comprehensive plan synthesizing all inputs
6. Summarize in README

Think sequentially and complete thoroughly.