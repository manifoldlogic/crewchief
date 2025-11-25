# Execution Plan: Workflow Commands

## Phase 1: Scaffolding (Tickets 1-3)
1.  **Command Structure**: Create `src/cli/project.ts` and register commands.
2.  **Templates**: Create markdown templates for planning docs.
3.  **Init Command**: Implement `crewchief project init <slug> <name>`.

## Phase 2: Management (Tickets 4-5)
4.  **List Command**: Implement `crewchief project list` scanning `.agents/projects`.
5.  **Status Command**: Parse markdown checkboxes to report % complete.

## Agent Assignments
- **Typescript Engineer**: All tickets.

## Success Definition
- We can delete the manual instructions in `.claude/commands/create-project.md` and replace them with "Run `crewchief project init ...`".

