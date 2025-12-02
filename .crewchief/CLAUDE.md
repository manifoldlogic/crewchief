# CLAUDE.md - .crewchief Directory

Working with the crewchief workspace at `.crewchief/`.

## Directory Structure

```
.crewchief/
├── projects/          # Active projects (planning + tickets)
├── archive/           # Completed projects
├── reports/           # Point-in-time analysis outputs (dated)
├── research/          # Exploratory technical research
└── scratchpad/        # Temporary working space
```

## Project Structure

```
projects/{SLUG}_{name}/
├── README.md
├── planning/
│   ├── analysis.md
│   ├── architecture.md
│   ├── plan.md
│   └── quality-strategy.md
└── tickets/
    └── {SLUG}-1001_description.md
```

## Naming Conventions

- **Projects**: `{SLUG}_{descriptive-name}` (e.g., `DKRHUB_docker-hub-publishing`)
- **Tickets**: `{SLUG}-{NUMBER}_{description}.md` (e.g., `DKRHUB-1001_setup.md`)
- **Planning docs**: Standard names (`analysis.md`, `architecture.md`, `plan.md`, `quality-strategy.md`)

## Workstream Commands

Use the workstream plugin for project management:

- `/workstream:project-create [description]` - Create project with planning docs
- `/workstream:project-review [SLUG]` - Review project (and tickets if they exist)
- `/workstream:project-update [SLUG]` - Update project based on review findings
- `/workstream:project-tickets [SLUG]` - Generate tickets from plan
- `/workstream:project-work [SLUG]` - Execute all tickets for a project
- `/workstream:ticket [TICKET_ID]` - Complete a single ticket
- `/workstream:status [SLUG]` - Check project/ticket status
- `/workstream:archive [SLUG]` - Archive completed projects

## Ticket Workflow

1. Implementation agent completes work
2. `unit-test-runner` executes tests
3. `verify-ticket` checks acceptance criteria
4. `commit-ticket` creates commit

## Key Locations

- **research/** - Exploratory technical research (pre-project)
- **reports/** - Dated analysis outputs
- **scratchpad/** - Temporary notes and experiments

## Archive

Move completed projects to `archive/projects/` when:
- All tickets complete
- Knowledge synthesized to `/docs/`
- No future work planned
