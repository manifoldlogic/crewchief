# .crewchief Directory

This directory contains project planning, work tickets, and execution tracking for the CrewChief project.

## Directory Structure

```
.crewchief/
├── projects/          # Active projects (planning + tickets)
├── archive/           # Completed projects
├── reports/           # Point-in-time analysis outputs (dated)
├── research/          # Exploratory technical research
└── scratchpad/        # Temporary working space
```

## Quick Navigation

### Active Work
- **[Projects](./projects/README.md)** - Current projects and their tickets

### Research & Reports
- **[Research](./research/)** - Exploratory technical research (pre-project)
- **[Reports](./reports/)** - Point-in-time analysis outputs (dated)

### Archive
- **[Archive](./archive/README.md)** - Completed projects

### Temporary
- **[Scratchpad](./scratchpad/)** - Temporary notes and experiments

## Workstream Plugin

Project management is handled by the workstream plugin. Key commands:

```bash
/workstream:project-create [description]   # Create project
/workstream:project-review [SLUG]          # Review project and tickets
/workstream:project-update [SLUG]          # Fix review findings
/workstream:project-tickets [SLUG]         # Generate tickets
/workstream:project-work [SLUG]            # Execute all tickets
/workstream:ticket [TICKET_ID]             # Complete single ticket
/workstream:status [SLUG]                  # Check status
```

## For AI Agents

### Finding Active Work
```bash
# List all active projects
ls -1 .crewchief/projects/

# Find tickets for a specific project
ls -1 .crewchief/projects/DKRHUB_docker-hub-publishing/tickets/

# Get project context
cat .crewchief/projects/DKRHUB_docker-hub-publishing/README.md
cat .crewchief/projects/DKRHUB_docker-hub-publishing/planning/*.md
```

### Project Lifecycle

1. **Planning Phase**: Create planning docs in `projects/{PROJECT}/planning/`
   - `analysis.md` - Problem analysis
   - `architecture.md` - Technical design
   - `plan.md` - Implementation phases
   - `quality-strategy.md` - Testing approach
   - `security-review.md` - Security considerations

2. **Review Phase**: Review and update based on findings
   - `/workstream:project-review` identifies issues
   - `/workstream:project-update` fixes issues

3. **Execution Phase**: Work on tickets in `projects/{PROJECT}/tickets/`
   - Track progress with checkboxes in ticket files
   - Update ticket status as work progresses

4. **Archive Phase**: When project complete and knowledge synthesized to `/docs`:
   - Move entire project folder to `archive/projects/{PROJECT}/`

## Conventions

### Project Naming
- Format: `{SLUG}_{descriptive-name}`
- **SLUG**: UPPERCASE, short (4-8 chars), matches ticket prefix
- **Description**: lowercase-with-dashes, clear and specific
- Examples: `DKRHUB_docker-hub-publishing/`, `LOCAL_local-deployment/`

### File Naming
- Use lowercase-with-dashes: `analysis.md`, `quality-strategy.md`
- Ticket files: `{SLUG}-{NUMBER}_{description}.md` (e.g., `DKRHUB-001_setup.md`)
- Index files: `README.md`

### Project Organization
```
projects/{SLUG}_{descriptive-name}/
├── README.md                  # Project status and overview
├── planning/                  # Strategic planning docs
│   ├── analysis.md
│   ├── architecture.md
│   ├── plan.md
│   ├── quality-strategy.md
│   └── security-review.md
└── tickets/                   # Active work tickets
    ├── {SLUG}-1001.md
    └── ...
```

## Maintenance

### When to Archive a Project

Archive when ALL of the following are true:
1. All tickets are complete (checkboxes marked)
2. Knowledge has been synthesized into `/docs`
3. No future work planned for this project area

**How to archive:**
```bash
/workstream:archive [SLUG]
```

### Adding New Projects

Use the workstream plugin:
```bash
/workstream:project-create [description]
```

This will scaffold the project structure and create initial planning documents.
