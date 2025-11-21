# Migrating from .claude to Antigravity

This guide documents the migration of your `.claude` configuration to Antigravity's `.agent` structure.

## Migration Status: Complete

All assets from `.claude` have been migrated to `.agent`.

- **Commands** (`.claude/commands/*.md`) have been converted to **Workflows** (`.agent/workflows/*.md`).
- **Agents** (`.claude/agents/*.md`) have been moved to **Reference** (`.agent/reference/`).

## Available Workflows

You can now use the following workflows by asking Antigravity to run them:

- **Archive Projects**: `archive-projects`
- **Create Project**: `create-project`
- **Create Project Tickets**: `create-project-tickets`
- **Review Project**: `review-project`
- **Review Tickets**: `review-tickets`
- **Single Ticket**: `single-ticket`
- **Skill Creator**: `skill-creator`
- **Ultrathink**: `ultrathink`
- **Update Reviewed Project**: `update-reviewed-project`
- **Work on Project**: `work-on-project`

## Agent References

Agent definitions are now located in `.agent/reference/`. You can refer to them in your instructions or workflows. For example:
> "Act as the [Agent Name] defined in .agent/reference/[Agent Name].md"

Additional agents from `agent-bench` have been migrated to `.agent/reference/agent-bench/`.

## Directory Structure

```
.agent/
├── workflows/          # Actionable workflows (formerly commands)
│   ├── create-project.md
│   ├── single-ticket.md
│   └── ...
├── reference/          # Reference material (formerly agents)
│   ├── ticket-creator.md
│   └── ...
└── MIGRATION_GUIDE.md  # This file
```
