# .crewchief/projects - Active Projects

Active projects with planning documentation and work tickets.

## Purpose

Contains all currently active projects. Each project has planning docs (analysis, architecture, plan, quality-strategy) and tickets for execution.

## Project Structure

```
{SLUG}_{descriptive-name}/
├── README.md           # Project overview and status
├── planning/
│   ├── analysis.md
│   ├── architecture.md
│   ├── plan.md
│   └── quality-strategy.md
└── tickets/
    ├── {SLUG}_TICKET_INDEX.md
    └── {SLUG}-{NUMBER}_{description}.md
```

## Naming Conventions

- **Project folders**: `{SLUG}_{descriptive-name}` (e.g., `DKRHUB_docker-hub-publishing`)
- **Tickets**: `{SLUG}-{NUMBER}_{description}.md` (e.g., `DKRHUB-1001_setup.md`)
- **Ticket numbers**: Start at 1001 for Phase 1, 2001 for Phase 2, etc.

## Lifecycle

1. **Create**: `/create-project [description]` generates planning docs
2. **Review**: `/review-project [SLUG]` validates readiness
3. **Tickets**: `/create-project-tickets [SLUG]` generates tickets
4. **Execute**: `/work-on-project [SLUG]` or `/single-ticket [ID]`
5. **Archive**: `/archive-projects` moves completed projects to `../archive/projects/`

## Completion Criteria

A project is complete when:
- All tickets have `- [x] **Verified**` checkbox checked
- No active development planned
- Knowledge synthesized to `/docs/` (if applicable)

## What Does NOT Belong Here

- Completed projects (move to `../archive/projects/`)
- Research without defined scope (use `../research/`)
- Point-in-time reports (use `../reports/`)
