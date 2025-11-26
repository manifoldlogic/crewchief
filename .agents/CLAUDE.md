# CLAUDE.md - .agents Directory

Working with agent workspace at `/.agents`.

## Directory Structure

```
.agents/
├── projects/          # Active projects (planning + tickets)
├── archive/           # Completed projects
├── agents/            # Agent definitions
├── reference/         # Templates and conventions (read-only)
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

## Slash Commands

- `/create-project [description]` - Create planning docs
- `/create-project-tickets [PROJECT_SLUG]` - Generate tickets
- `/review-tickets [PROJECT_SLUG]` - Review quality
- `/work-on-project [PROJECT_SLUG]` - Execute all tickets
- `/single-ticket [ticket-id]` - Complete one ticket

## Ticket Workflow

1. Implementation agent completes work
2. `unit-test-runner` executes tests
3. `verify-ticket` checks acceptance criteria
4. `commit-ticket` creates commit

## Key Locations

- **agents/** - Agent definitions (see `agents/README.md`)
- **reference/** - Templates and conventions
  - `work-ticket-template.md`
  - `project-naming-guidelines.md`
  - `git-commit-scopes.txt`
- **research/** - Exploratory technical research (pre-project)
- **reports/** - Dated analysis outputs
- **scratchpad/** - Temporary notes and experiments

## Archive

Move completed projects to `archive/projects/` when:
- All tickets complete
- Knowledge synthesized to `/docs/`
- No future work planned
