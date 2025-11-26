# .agents Directory

This directory contains AI agent-focused documentation, project planning, work tickets, and execution tracking for the CrewChief project.

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

## Quick Navigation

### 🚀 Active Work
- **[Projects](./projects/README.md)** - Current projects and their tickets
- **[Agents](./agents/README.md)** - Available agents and their capabilities

### 📖 Process & Reference
- **[Reference](./reference/)** - Development process and conventions
  - [Spec-Driven Development](./reference/spec-driven-development.md)
  - [Work Ticket Template](./reference/work-ticket-template.md)
  - [Git Commit Scopes](./reference/git-commit-scopes.txt)

### 🔬 Research & Reports
- **[Research](./research/)** - Exploratory technical research (pre-project)
- **[Reports](./reports/)** - Point-in-time analysis outputs (dated)

### 📦 Archive
- **[Archive](./archive/README.md)** - Completed projects

### 🗒️ Temporary
- **[Scratchpad](./scratchpad/)** - Temporary notes and experiments

## For AI Agents

### Finding Active Work
```bash
# List all active projects
ls -1 .agents/projects/

# Find tickets for a specific project
ls -1 .agents/projects/DKRHUB_docker-hub-publishing/tickets/

# Get project context
cat .agents/projects/DKRHUB_docker-hub-publishing/README.md
cat .agents/projects/DKRHUB_docker-hub-publishing/planning/*.md
```

### Agent Capabilities
```bash
# List all available agents
ls -1 .agents/agents/

# Read agent definition
cat .agents/agents/database-engineer.md
```

### Project Lifecycle

1. **Planning Phase**: Create planning docs in `projects/{PROJECT}/planning/`
   - `analysis.md` - Problem analysis
   - `architecture.md` - Technical design
   - `plan.md` - Implementation phases
   - `quality-strategy.md` - Testing approach
   - `security-review.md` - Security considerations

2. **Execution Phase**: Work on tickets in `projects/{PROJECT}/tickets/`
   - Track progress with checkboxes in ticket files
   - Update ticket status as work progresses

3. **Archive Phase**: When project complete and knowledge synthesized to `/docs`:
   - Move entire project folder to `archive/projects/{PROJECT}/`

## Conventions

### Project Naming
- Format: `{SLUG}_{descriptive-name}`
- **SLUG**: UPPERCASE, short (4-8 chars), matches ticket prefix
- **Description**: lowercase-with-dashes, clear and specific
- Examples: `DKRHUB_docker-hub-publishing/`, `LOCAL_local-deployment/`
- See [Project Naming Guidelines](./reference/project-naming-guidelines.md) for details

### File Naming
- Use lowercase-with-dashes: `analysis.md`, `quality-strategy.md`
- Ticket files: `{SLUG}-{NUMBER}_{description}.md` (e.g., `DKRHUB-001_setup.md`)
- Index files: `{SLUG}_TICKET_INDEX.md` or `README.md`

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
├── tickets/                   # Active work tickets
│   ├── {SLUG}-001.md
│   └── ...
└── archive/                   # Completed tickets (optional)
    └── tickets/
```

## Maintenance

### When to Archive a Project

Archive when ALL of the following are true:
1. ✅ All tickets are complete (checkboxes marked)
2. ✅ Knowledge has been synthesized into `/docs`
3. ✅ No future work planned for this project area

**How to archive:**
```bash
mv .agents/projects/{PROJECT} .agents/archive/projects/
```

### Adding New Projects

1. Choose a project slug and name following [naming guidelines](./reference/project-naming-guidelines.md)

2. Create project structure:
```bash
mkdir -p .agents/projects/{SLUG}_{descriptive-name}/{planning,tickets}
```

3. Create planning documents in `planning/`:
   - Start with `analysis.md` and `architecture.md`
   - Generate tickets from `plan.md`

4. Update this README and `projects/README.md`
